// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;
use itertools::Itertools;
use databend_common_base::base::tokio;
use databend_common_exception::ErrorCode;
use databend_common_exception::Result;
use databend_common_expression::{BlockEntry, Column, DataBlock, ScalarRef};
use databend_common_io::prelude::FormatSettings;
use log::debug;
use log::info;
use parking_lot::RwLock;

use super::string_block::{block_to_strings, BlocksSerializer};
use super::string_block::StringBlock;
use crate::servers::http::v1::query::sized_spsc::SizedChannelReceiver;

#[derive(Debug, PartialEq, Eq)]
pub enum Wait {
    Async,
    Deadline(Instant),
}

#[derive(Clone)]
pub struct Page {
    pub data: Arc<BlocksSerializer>,
}

pub struct ResponseData {
    pub page: Page,
    pub next_page_no: Option<usize>,
}

pub struct PageManager {
    max_rows_per_page: usize,
    total_rows: usize,
    total_pages: usize,
    end: bool,
    block_end: bool,
    last_page: Option<Page>,
    row_buffer: Option<Vec<Column>>,
    block_receiver: SizedChannelReceiver<DataBlock>,
    format_settings: Arc<RwLock<Option<FormatSettings>>>,
}

impl PageManager {
    pub fn new(
        max_rows_per_page: usize,
        block_receiver: SizedChannelReceiver<DataBlock>,
        format_settings: Arc<RwLock<Option<FormatSettings>>>,
    ) -> PageManager {
        PageManager {
            total_rows: 0,
            last_page: None,
            total_pages: 0,
            end: false,
            block_end: false,
            row_buffer: Default::default(),
            block_receiver,
            max_rows_per_page,
            format_settings,
        }
    }

    pub fn next_page_no(&mut self) -> Option<usize> {
        if self.end {
            None
        } else {
            Some(self.total_pages)
        }
    }

    #[async_backtrace::framed]
    pub async fn get_a_page(&mut self, page_no: usize, tp: &Wait) -> Result<Page> {
        let next_no = self.total_pages;
        if page_no == next_no {
            let mut serializer = BlocksSerializer::new(self.format_settings.read().clone());
            if !self.end {
                let end = self.collect_new_page(&mut serializer, tp).await?;
                let num_row = serializer.num_rows();
                self.total_rows += num_row;
                let page = Page { data: Arc::new(serializer) };
                if num_row > 0 {
                    self.total_pages += 1;
                    self.last_page = Some(page.clone());
                }
                self.end = end;
                Ok(page)
            } else {
                // when end is set to true, client should recv a response with next_url = final_url
                // but the response may be lost and client will retry,
                // we simply return an empty page.
                let page = Page {
                    data: Arc::new(serializer),
                };
                Ok(page)
            }
        } else if page_no + 1 == next_no {
            // later, there may be other ways to ack and drop the last page except collect_new_page.
            // but for now, last_page always exists in this branch, since page_no is unsigned.
            Ok(self
                .last_page
                .as_ref()
                .ok_or_else(|| ErrorCode::Internal("last_page is None"))?
                .clone())
        } else {
            let message = format!("wrong page number {}", page_no,);
            Err(ErrorCode::HttpNotFound(message))
        }
    }

    fn append_block(
        &mut self,
        serializer: &mut BlocksSerializer,
        block: DataBlock,
        remain_rows: &mut usize,
        remain_size: &mut usize,
    ) -> Result<()> {
        if !serializer.has_format() {
            let guard = self.format_settings.read();
            serializer.set_format(guard.as_ref().unwrap().clone());
        }

        let columns = block.columns().iter()
            .map(|entry| {
                entry.to_column(block.num_rows())
            })
            .collect_vec();

        let mut i = 0;
        while *remain_rows > 0 && *remain_size > 0 && i < block.num_rows() {
            let size = row_size(&columns, i);
            if *remain_size > size {
                *remain_size -= size;
                *remain_rows -= 1;
                i += 1;
            } else {
                *remain_size = 0;
                if serializer.is_empty() && i == 0 {
                    i += 1
                }
            }
        }
        if i == block.num_rows() {
            serializer.append(columns, block.num_rows());
            self.row_buffer = None;
        } else {
            let fn_slice = |columns: &[Column], range: Range<usize>| {
                columns.iter()
                    .map(|column| column.slice(range.clone()))
                    .collect_vec()
            };

            serializer.append(fn_slice(&columns, 0..i), i);
            self.row_buffer = Some(fn_slice(&columns, i..block.num_rows()));
        }
        Ok(())
    }

    #[async_backtrace::framed]
    async fn collect_new_page(&mut self, serializer: &mut BlocksSerializer, tp: &Wait) -> Result<bool> {
        let mut remain_size = 10 * 1024 * 1024;
        let mut remain_rows = self.max_rows_per_page;
        while remain_rows > 0 && remain_size > 0 {
            if let Some(block) = self.row_buffer.take() {
                self.append_block(serializer, DataBlock::new_from_columns(block), &mut remain_rows, &mut remain_size)?;
            } else {
                break;
            }
        }

        while remain_rows > 0 && remain_size > 0 {
            match tp {
                Wait::Async => match self.block_receiver.try_recv() {
                    Some(block) => {
                        self.append_block(serializer, block, &mut remain_rows, &mut remain_size)?
                    }
                    None => break,
                },
                Wait::Deadline(t) => {
                    let now = Instant::now();
                    let d = *t - now;
                    if d.is_zero() {
                        // timeout() will return Ok if the future completes immediately
                        break;
                    }
                    match tokio::time::timeout(d, self.block_receiver.recv()).await {
                        Ok(Some(block)) => {
                            debug!("http query got new block with {} rows", block.num_rows());
                            self.append_block(serializer, block, &mut remain_rows, &mut remain_size)?
                        }
                        Ok(None) => {
                            info!("http query reach end of blocks");
                            break;
                        }
                        Err(_) => {
                            debug!("http query long pulling timeout");
                            break;
                        }
                    }
                }
            }
        }

        // try to report 'no more data' earlier to client to avoid unnecessary http call
        if !self.block_end {
            self.block_end = self.block_receiver.is_empty();
        }
        Ok(self.block_end && self.row_buffer.is_none())
    }

    #[async_backtrace::framed]
    pub async fn detach(&mut self) {
        self.block_receiver.close();
        self.last_page = None;
        self.row_buffer = None;
    }
}

fn row_size(columns: &[Column], row_index: usize) -> usize {
    // ["1","2",null],
    columns
        .iter()
        .map(|column| match column.index(row_index) {
            Some(s) => {
                s.memory_size() + 2
            }
            None => 2
        })
        .sum::<usize>()
        + columns.len() * 3
        + 2
}
