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

use std::sync::Arc;

use arrow_flight::FlightData;
use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use concurrent_queue::ConcurrentQueue;
use databend_common_base::runtime::Runtime;
use databend_common_exception::Result;
use parking_lot::Mutex;
use tokio::sync::Semaphore;
use tonic::Status;

use super::outbound_transport::PingPongCallback;
use super::outbound_transport::PingPongExchange;
use super::outbound_transport::PingPongResponse;
use crate::network::inbound_quota::RemoteQueueItem;

/// Configuration for ExchangeSinkBuffer.
#[derive(Clone)]
pub struct ExchangeBufferConfig {
    pub queue_capacity_factor: usize,
    pub max_batch_bytes: usize,
}

impl Default for ExchangeBufferConfig {
    fn default() -> Self {
        Self {
            queue_capacity_factor: 64,
            max_batch_bytes: 256 * 1024,
        }
    }
}

struct Channel {
    pending_queue: ConcurrentQueue<RemoteQueueItem>,
}

impl Channel {
    fn new() -> Self {
        Self {
            pending_queue: ConcurrentQueue::unbounded(),
        }
    }

    fn remaining(&self) -> usize {
        self.pending_queue.len()
    }

    fn pop_front(&mut self, max_batch_bytes: usize) -> Option<FlightData> {
        let mut items = Vec::new();
        let mut total = 0usize;

        while let Ok(next) = self.pending_queue.pop() {
            let data = next.into_data();
            total += data.data_body.len();
            items.push(data);
            if total >= max_batch_bytes {
                break;
            }
        }

        match items.len() {
            0 => None,
            1 => Some(items.into_iter().next().unwrap()),
            _ => {
                let tid_bytes: [u8; 2] = [items[0].app_metadata[0], items[0].app_metadata[1]];
                Some(merge_flight_data_batch(tid_bytes, items))
            }
        }
    }
}

const BATCH_MARKER: u8 = 0x02;

fn merge_flight_data_batch(tid_bytes: [u8; 2], items: Vec<FlightData>) -> FlightData {
    let mut app_metadata = BytesMut::with_capacity(5);
    app_metadata.put_slice(&tid_bytes);
    app_metadata.put_u16_le(items.len() as u16);
    app_metadata.put_u8(BATCH_MARKER);

    let estimated: usize = items
        .iter()
        .map(|i| 12 + (i.app_metadata.len() - 2) + i.data_header.len() + i.data_body.len())
        .sum();

    let mut body = BytesMut::with_capacity(estimated);
    for item in items {
        let inner_meta = &item.app_metadata[2..];
        body.put_u32_le(inner_meta.len() as u32);
        body.put_slice(inner_meta);
        body.put_u32_le(item.data_header.len() as u32);
        body.put_slice(&item.data_header);
        body.put_u32_le(item.data_body.len() as u32);
        body.put_slice(&item.data_body);
    }

    FlightData {
        flight_descriptor: None,
        app_metadata: app_metadata.freeze(),
        data_header: Bytes::new(),
        data_body: body.freeze(),
    }
}

struct RemoteInstanceState {
    channels: Vec<Channel>,
    last_error: Option<Status>,
}

struct RemoteInstance {
    state: Mutex<RemoteInstanceState>,
    exchange: PingPongExchange,
}

impl RemoteInstance {
    fn new(num_threads: usize, exchange: PingPongExchange) -> Self {
        let channels = (0..num_threads).map(|_| Channel::new()).collect();
        Self {
            exchange,
            state: Mutex::new(RemoteInstanceState {
                channels,
                last_error: None,
            }),
        }
    }
}

struct ExchangeSinkBufferSharedState {
    config: ExchangeBufferConfig,
    remotes: Vec<Arc<RemoteInstance>>,
}

struct ExchangeSinkBufferInner {
    state: Arc<ExchangeSinkBufferSharedState>,
}

impl Drop for ExchangeSinkBufferInner {
    fn drop(&mut self) {
        for remote in &self.state.remotes {
            remote.exchange.shutdown.notify_waiters();
        }
    }
}

impl ExchangeSinkBufferSharedState {
    fn try_flush_remote(&self, dest_idx: usize, status: Option<Status>) {
        let remote = &self.remotes[dest_idx];
        let mut state = remote.state.lock();

        let Some(status) = status else {
            let Some(channel) = state.channels.iter_mut().max_by_key(|x| x.remaining()) else {
                return remote.exchange.ready_send();
            };

            let Some(flight) = channel.pop_front(self.config.max_batch_bytes) else {
                return remote.exchange.ready_send();
            };

            let Ok(_) = remote.exchange.force_send(flight) else {
                state.last_error = Some(Status::aborted("Exchange closed"));
                return remote.exchange.ready_send();
            };

            return;
        };

        state.last_error = Some(status);
        for channel in &state.channels {
            channel.pending_queue.close();
        }

        for channel in &state.channels {
            while channel.pending_queue.pop().is_ok() {}
        }
    }
}

struct SinkBufferCallback {
    dest_idx: usize,
    buffer: Arc<ExchangeSinkBufferSharedState>,
}

impl PingPongCallback for SinkBufferCallback {
    fn has_pending(&self) -> bool {
        let state = self.buffer.remotes[self.dest_idx].state.lock();
        state.channels.iter().any(|x| !x.pending_queue.is_empty())
    }

    fn on_response(&self, response: PingPongResponse) {
        self.buffer
            .try_flush_remote(self.dest_idx, response.data.err());
    }
}

pub struct ExchangeSinkBuffer {
    semaphore: Arc<Semaphore>,
    inner: Arc<ExchangeSinkBufferInner>,
}

impl ExchangeSinkBuffer {
    pub fn create(
        exchanges: Vec<PingPongExchange>,
        config: ExchangeBufferConfig,
        runtime: &Runtime,
    ) -> Result<Self> {
        let queue_capacity = config.queue_capacity_factor * exchanges.len().max(1);

        let remotes = Vec::with_capacity(exchanges.len());

        let semaphore = Arc::new(Semaphore::new(queue_capacity));
        let mut shared_state = ExchangeSinkBufferSharedState { config, remotes };

        for exchange in exchanges {
            let num_threads = exchange.num_threads;
            let remote_instance = Arc::new(RemoteInstance::new(num_threads, exchange));
            shared_state.remotes.push(remote_instance);
        }

        let shared_state = Arc::new(shared_state);
        for (dest_idx, remote) in shared_state.remotes.iter().enumerate() {
            let _ = remote.exchange.start(
                Arc::new(SinkBufferCallback {
                    dest_idx,
                    buffer: shared_state.clone(),
                }),
                runtime,
            );
        }

        Ok(Self {
            semaphore,
            inner: Arc::new(ExchangeSinkBufferInner {
                state: shared_state,
            }),
        })
    }

    pub async fn add_data(&self, tid: usize, dest_idx: usize, data: FlightData) -> Result<()> {
        let remote = &self.inner.state.remotes[dest_idx];

        if let Some(data) = remote.exchange.try_send(data)? {
            let semaphore = self.semaphore.clone();
            let owned_semaphore_permit = semaphore.acquire_owned().await.unwrap();

            let state = remote.state.lock();

            if let Some(status) = state.last_error.clone() {
                return Err(status.into());
            }

            if let Some(data) = remote.exchange.try_send(data)? {
                let item = RemoteQueueItem::new(data, owned_semaphore_permit);
                let _ = state.channels[tid].pending_queue.push(item);
            }
        }

        Ok(())
    }
}
