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
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use arrow_flight::FlightData;
use async_channel::Sender;
use async_channel::TrySendError;
use databend_common_base::base::WatchNotify;
use databend_common_base::runtime::Runtime;
use futures::StreamExt;
use futures::future::Either;
use futures::future::select;
use futures::stream::BoxStream;
use parking_lot::Mutex;
use tokio::task::JoinHandle;
use tonic::Status;

/// Response from a ping-pong exchange.
pub struct PingPongResponse {
    pub data: Result<FlightData, Status>,
    pub rtt: Duration,
}

/// Callback trait for handling ping-pong responses.
pub trait PingPongCallback: Send + Sync + 'static {
    fn has_pending(&self) -> bool;
    fn on_response(&self, response: PingPongResponse);
}

pub struct PingPongExchangeInner {
    in_flight: AtomicBool,
    send_time: Mutex<Option<Instant>>,
    send_tx: Sender<FlightData>,
    pub shutdown: WatchNotify,
}

/// A non-blocking ping-pong style flight exchange.
pub struct PingPongExchange {
    pub num_threads: usize,
    inner: Arc<PingPongExchangeInner>,
    response_stream: Mutex<Option<BoxStream<'static, Result<FlightData, Status>>>>,
}

impl Drop for PingPongExchange {
    fn drop(&mut self) {
        self.inner.shutdown.notify_waiters();
    }
}

impl std::ops::Deref for PingPongExchange {
    type Target = PingPongExchangeInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PingPongExchangeInner {
    pub fn get_rtt(&self) -> Duration {
        self.send_time
            .lock()
            .take()
            .map(|t| t.elapsed())
            .unwrap_or_default()
    }

    pub fn try_send(&self, data: FlightData) -> Result<Option<FlightData>, Status> {
        if self.in_flight.fetch_or(true, Ordering::SeqCst) {
            return Ok(Some(data));
        }

        self.force_send(data)
    }

    pub(crate) fn force_send(&self, data: FlightData) -> Result<Option<FlightData>, Status> {
        *self.send_time.lock() = Some(Instant::now());
        match self.send_tx.try_send(data) {
            Ok(_) => Ok(None),
            Err(TrySendError::Closed(_)) => {
                *self.send_time.lock() = None;
                self.in_flight.store(false, Ordering::SeqCst);
                Err(Status::aborted("Exchange closed"))
            }
            Err(TrySendError::Full(data)) => {
                *self.send_time.lock() = None;
                self.in_flight.store(false, Ordering::SeqCst);
                Ok(Some(data))
            }
        }
    }

    pub fn ready_send(&self) {
        self.in_flight.store(false, Ordering::SeqCst);
    }
}

impl PingPongExchange {
    pub fn from_parts(
        num_threads: usize,
        send_tx: async_channel::Sender<FlightData>,
        response_stream: tonic::Streaming<FlightData>,
    ) -> Self {
        Self::from_stream(num_threads, send_tx, response_stream)
    }

    pub fn from_stream(
        num_threads: usize,
        send_tx: async_channel::Sender<FlightData>,
        stream: impl futures::Stream<Item = Result<FlightData, Status>> + Send + 'static,
    ) -> Self {
        let inner = Arc::new(PingPongExchangeInner {
            in_flight: AtomicBool::new(false),
            send_time: Mutex::new(None),
            send_tx,
            shutdown: WatchNotify::new(),
        });

        Self {
            inner,
            num_threads,
            response_stream: Mutex::new(Some(Box::pin(stream))),
        }
    }

    pub fn start(
        &self,
        callback: Arc<dyn PingPongCallback>,
        runtime: &Runtime,
    ) -> Result<JoinHandle<()>, Status> {
        let Some(mut stream) = self.response_stream.lock().take() else {
            return Err(Status::already_exists("Receiver already started"));
        };

        let inner = self.inner.clone();
        Ok(runtime.spawn(async move {
            let mut finished = false;
            let mut shutdown_fut = Box::pin(inner.shutdown.notified());

            loop {
                if finished && !callback.has_pending() {
                    inner.send_tx.close();
                }

                match select(shutdown_fut, stream.next()).await {
                    Either::Left(_) => {
                        finished = true;
                        shutdown_fut = Box::pin(inner.shutdown.notified());
                    }
                    Either::Right((None, _)) => {
                        break;
                    }
                    Either::Right((Some(Ok(data)), next_shutdown)) => {
                        shutdown_fut = next_shutdown;
                        let rtt = inner.get_rtt();
                        callback.on_response(PingPongResponse {
                            data: Ok(data),
                            rtt,
                        });
                    }
                    Either::Right((Some(Err(status)), _)) => {
                        let rtt = inner.get_rtt();
                        callback.on_response(PingPongResponse {
                            data: Err(status),
                            rtt,
                        });
                        break;
                    }
                }
            }
        }))
    }
}
