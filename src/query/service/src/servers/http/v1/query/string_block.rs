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

use std::cell::RefCell;
use std::ops::DerefMut;
use std::sync::Arc;
use serde::ser::SerializeSeq;
use databend_common_exception::Result;
use databend_common_expression::Column;
use databend_common_expression::DataBlock;
use databend_common_formats::field_encoder::FieldEncoderValues;
use databend_common_io::prelude::FormatSettings;

pub type BlocksSerializerRef = Arc<BlocksSerializer>;

#[derive(Debug, Clone, Default)]
pub struct StringBlock {
    pub(crate) data: Vec<Vec<Option<String>>>,
}

fn data_is_null(column: &Column, row_index: usize) -> bool {
    match column {
        Column::Null { .. } => true,
        Column::Nullable(box inner) => !inner.validity.get_bit(row_index),
        _ => false,
    }
}

#[derive(Debug, Clone)]
pub struct BlocksSerializer {
    // Vec<Column> for a Block
    columns: Vec<(Vec<Column>, usize)>,
    pub(crate) format: Option<FormatSettings>,
}

impl BlocksSerializer {
    pub fn empty() -> Self {
        Self {
            columns: vec![],
            format: None,
        }
    }

    pub fn new(format: Option<FormatSettings>) -> Self {
        Self {
            columns: vec![],
            format,
        }
    }

    pub fn has_format(&self) -> bool {
        self.format.is_some()
    }

    pub fn set_format(&mut self, format: FormatSettings) {
        self.format = Some(format);
    }

    pub fn append(&mut self, columns: Vec<Column>, num_rows: usize) {
        self.columns.push((columns, num_rows));
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn num_rows(&self) -> usize {
        self.columns
            .iter()
            .map(|(_, num_rows)| *num_rows)
            .sum()
    }
}

impl serde::Serialize for BlocksSerializer {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut serialize_seq = serializer.serialize_seq(Some(self.num_rows()))?;
        if let Some(format) = &self.format {
            let mut buf = RefCell::new(Vec::new());
            let encoder = FieldEncoderValues::create_for_http_handler(
                format.jiff_timezone.clone(),
                format.timezone,
                format.geometry_format,
            );
            for (columns, num_rows) in self.columns.iter() {
                for i in 0..*num_rows {
                    serialize_seq.serialize_element(&RowSerializer {
                        format,
                        data_block: columns,
                        encodeer: &encoder,
                        buf: &mut buf,
                        row_index: i,
                    })?
                }
            }
        }
        serialize_seq.end()
    }
}

struct RowSerializer<'a> {
    format: &'a FormatSettings,
    data_block: &'a [Column],
    encodeer: &'a FieldEncoderValues,
    buf: &'a RefCell<Vec<u8>>,
    row_index: usize,
}

impl<'a> serde::Serialize for RowSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut serialize_seq = serializer.serialize_seq(Some(self.data_block.len()))?;

        for column in self.data_block.iter() {
            if !self.format.format_null_as_str && data_is_null(column, self.row_index) {
                serialize_seq.serialize_element(&None::<String>)?;
                continue;
            }
            let string = self.encodeer
                .try_direct_as_string(&column, self.row_index, false)
                .unwrap_or_else(|| {
                    let mut buf = self.buf.borrow_mut();
                    buf.clear();
                    self.encodeer.write_field(column, self.row_index, buf.deref_mut(), false);
                    String::from_utf8_lossy(buf.deref_mut()).into_owned()
                });
            serialize_seq.serialize_element(&Some(string))?;
        }
        serialize_seq.end()
    }
}

pub fn block_to_strings(
    block: &DataBlock,
    format: &FormatSettings,
) -> Result<Vec<Vec<Option<String>>>> {
    if block.is_empty() {
        return Ok(vec![]);
    }
    let rows_size = block.num_rows();
    let columns: Vec<Column> = block
        .columns()
        .iter()
        .map(|column| column.to_column(block.num_rows()))
        .collect();

    let mut res = Vec::new();
    let encoder = FieldEncoderValues::create_for_http_handler(
        format.jiff_timezone.clone(),
        format.timezone,
        format.geometry_format,
    );
    let mut buf = vec![];
    for row_index in 0..rows_size {
        let mut row: Vec<Option<String>> = Vec::with_capacity(block.num_columns());
        for column in &columns {
            if !format.format_null_as_str && data_is_null(column, row_index) {
                row.push(None);
                continue;
            }
            buf.clear();
            let string = encoder
                .try_direct_as_string(&column, row_index, false)
                .unwrap_or_else(|| {
                    encoder.write_field(column, row_index, &mut buf, false);
                    String::from_utf8_lossy(&buf).into_owned()
                });
            row.push(Some(string));
        }
        res.push(row)
    }
    Ok(res)
}

impl StringBlock {
    pub fn empty() -> Self {
        Self { data: vec![] }
    }

    pub fn new(block: &DataBlock, format: &FormatSettings) -> Result<Self> {
        Ok(StringBlock {
            data: block_to_strings(block, format)?,
        })
    }

    pub fn concat(blocks: Vec<StringBlock>) -> Self {
        if blocks.is_empty() {
            return Self::empty();
        }
        let results = blocks.into_iter().map(|b| b.data).collect::<Vec<_>>();
        let data = results.concat();
        Self { data }
    }

    pub fn num_rows(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_data<'n>(&'n self, null_as: &'n str) -> Vec<Vec<&'n str>> {
        self.data
            .iter()
            .map(|row| {
                row.iter()
                    .map(|v| match v {
                        Some(v) => v,
                        None => null_as,
                    })
                    .collect()
            })
            .collect()
    }
}

impl From<StringBlock> for Vec<Vec<Option<String>>> {
    fn from(block: StringBlock) -> Self {
        block.data
    }
}
