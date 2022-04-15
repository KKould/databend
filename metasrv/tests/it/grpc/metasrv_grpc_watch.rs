// Copyright 2021 Datafuse Labs.
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
use std::thread;
use std::time::Duration;

use common_base::tokio;
use common_base::tokio::sync::mpsc;
use common_base::tokio::sync::mpsc::UnboundedReceiver;
use common_base::tokio::sync::mpsc::UnboundedSender;
use common_meta_api::KVApi;
use common_meta_grpc::MetaGrpcClient;
use common_meta_types::protobuf::event::SeqV;
use common_meta_types::protobuf::watch_request::FilterType;
use common_meta_types::protobuf::Event;
use common_meta_types::protobuf::WatchRequest;
use common_meta_types::MatchSeq;
use common_meta_types::Operation;
use common_meta_types::UpsertKVAction;

use crate::init_meta_ut;

async fn watcher_client_main(
    addr: String,
    watch: WatchRequest,
    start_tx: UnboundedSender<()>,
    mut events_rx: UnboundedReceiver<Vec<Event>>,
) -> anyhow::Result<()> {
    let client = MetaGrpcClient::try_create(addr.as_str(), "root", "xxx", None, None).await?;
    let mut grpc_client = client.make_client().await?;

    let request = tonic::Request::new(watch);

    let mut client_stream = grpc_client.watch(request).await?.into_inner();

    let mut watch_events = Vec::<Event>::new();

    // notify client stream has started
    let _ = start_tx.send(());

    loop {
        tokio::select! {
            resp = client_stream.message() => {
                if let Ok(Some(resp)) = resp {
                    if let Some(event) = resp.event {
                        assert!(!watch_events.is_empty());

                        assert_eq!(watch_events.get(0), Some(&event));
                        watch_events.remove(0);

                        if watch_events.is_empty() {
                            // notify has recv all the notify events
                            let _ = start_tx.send(());
                        }
                    }

                }
            },
            events = events_rx.recv() => {
                assert!(watch_events.is_empty());
                if let Some(events) = events {
                    for event in events {
                        watch_events.push(event);
                    }
                }
                // notify has recv evens
                let _ = start_tx.send(());
            }
        }
    }
}

fn wait_notify(start_rx: &mut UnboundedReceiver<()>, wait_ms: i32) {
    let mut cnt = 0;
    while start_rx.try_recv().is_err() {
        thread::sleep(Duration::from_millis(100));
        cnt += 100;
        if cnt >= wait_ms {
            panic!("wait out of time");
        }
    }
}

async fn test_watch_main(
    addr: String,
    watch: WatchRequest,
    events: Vec<Event>,
    updates: Vec<UpsertKVAction>,
) -> anyhow::Result<()> {
    let client = MetaGrpcClient::try_create(addr.as_str(), "root", "xxx", None, None).await?;

    let (start_tx, mut start_rx) = mpsc::unbounded_channel::<()>();
    let (events_tx, events_rx) = mpsc::unbounded_channel::<Vec<Event>>();

    let h = tokio::spawn(watcher_client_main(
        addr.clone(),
        watch,
        start_tx,
        events_rx,
    ));

    // wait for client stream start up
    wait_notify(&mut start_rx, 2000);

    let _ = events_tx.send(events);

    // wait client stream recv events
    wait_notify(&mut start_rx, 2000);

    for update in updates.iter() {
        let _ = client.upsert_kv(update.clone()).await;
    }

    // wait client stream recv notify
    wait_notify(&mut start_rx, 2000);

    // ok, shutdown client main
    h.abort();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn test_watch() -> anyhow::Result<()> {
    // - Start a metasrv server.
    // - Watch some key.
    // - Write some data.
    // - Assert watcher get all the update.

    let (_log_guards, ut_span) = init_meta_ut!();
    let _ent = ut_span.enter();
    let (_tc, addr) = crate::tests::start_metasrv().await?;

    let mut seq: u64 = 1;
    // 1.update some events
    {
        let watch = WatchRequest {
            key: "a".to_string(),
            key_end: Some("z".to_string()),
            filter_type: FilterType::All.into(),
        };

        let key_a = "a".to_string();
        let key_b = "b".to_string();
        let key_z = "z".to_string();

        let val_a = "a".as_bytes().to_vec();
        let val_b = "b".as_bytes().to_vec();
        let val_new = "new".as_bytes().to_vec();
        let val_z = "z".as_bytes().to_vec();

        let events = vec![
            // set a->a
            Event {
                key: key_a.clone(),
                current: Some(SeqV {
                    seq,
                    data: val_a.clone(),
                }),
                prev: None,
            },
            // set z->z
            Event {
                key: key_z.clone(),
                current: Some(SeqV {
                    seq: seq + 1,
                    data: val_z.clone(),
                }),
                prev: None,
            },
            // set b->b
            Event {
                key: key_b.clone(),
                current: Some(SeqV {
                    seq: seq + 2,
                    data: val_b.clone(),
                }),
                prev: None,
            },
            // update b->new
            Event {
                key: key_b.clone(),
                current: Some(SeqV {
                    seq: seq + 3,
                    data: val_new.clone(),
                }),
                prev: Some(SeqV {
                    seq: seq + 2,
                    data: val_b.clone(),
                }),
            },
            // delete b
            Event {
                key: key_b.clone(),
                prev: Some(SeqV {
                    seq: seq + 3,
                    data: val_new.clone(),
                }),
                current: None,
            },
        ];

        seq += 3;
        // update kv
        let updates = vec![
            UpsertKVAction::new("a", MatchSeq::Any, Operation::Update(val_a), None),
            UpsertKVAction::new("z", MatchSeq::Any, Operation::Update(val_z), None),
            UpsertKVAction::new("b", MatchSeq::Any, Operation::Update(val_b), None),
            UpsertKVAction::new("b", MatchSeq::Any, Operation::Update(val_new), None),
            UpsertKVAction::new("b", MatchSeq::Any, Operation::Delete, None),
        ];
        test_watch_main(addr.clone(), watch, events, updates).await?;
    }

    // 2. test filter
    {
        let key_str = "1";
        let watch = WatchRequest {
            key: key_str.to_string(),
            key_end: None,
            // filter only delete events
            filter_type: FilterType::Delete.into(),
        };

        let key = key_str.to_string();
        let val = "old".as_bytes().to_vec();
        let val_new = "new".as_bytes().to_vec();

        // has only delete events
        let events = vec![
            // delete 1 first time
            Event {
                key: key.clone(),
                prev: Some(SeqV {
                    seq: seq + 1,
                    data: val.clone(),
                }),
                current: None,
            },
            // delete 1 second time
            Event {
                key: key.clone(),
                prev: Some(SeqV {
                    seq: seq + 2,
                    data: val_new.clone(),
                }),
                current: None,
            },
        ];

        // update and delete twice
        let updates = vec![
            UpsertKVAction::new(key_str, MatchSeq::Any, Operation::Update(val), None),
            UpsertKVAction::new(key_str, MatchSeq::Any, Operation::Delete, None),
            UpsertKVAction::new(key_str, MatchSeq::Any, Operation::Update(val_new), None),
            UpsertKVAction::new(key_str, MatchSeq::Any, Operation::Delete, None),
        ];
        test_watch_main(addr.clone(), watch, events, updates).await?;
    }

    Ok(())
}
