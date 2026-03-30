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

#[path = "servers/flight/v1/exchange/mod.rs"]
pub mod exchange;
#[path = "servers/flight/v1/packets/mod.rs"]
pub mod packets;

pub use databend_query_distributed::flight_client;
pub use databend_query_distributed::keep_alive;
pub use databend_query_distributed::network;
pub use databend_query_distributed::request_builder;
pub use databend_query_distributed::scatter;

pub mod serde {
    pub use crate::distributed::exchange::serde::ExchangeDeserializeMeta;
    pub use crate::distributed::exchange::serde::deserialize_block;
    pub use crate::distributed::exchange::serde::serialize_block;
}

pub use exchange::BroadcastExchange;
pub use exchange::DataExchange;
pub use exchange::DataExchangeManager;
pub use exchange::DefaultExchangeInjector;
pub use exchange::ExchangeInjector;
pub use exchange::ExchangeShuffleMeta;
pub use exchange::ExchangeShuffleTransform;
pub use exchange::ExchangeSorting;
pub use exchange::MergeExchange;
pub use exchange::MergeExchangeParams;
pub use exchange::NodeToNodeExchange;
pub use exchange::ShuffleExchangeParams;
pub use exchange::serde::ExchangeDeserializeMeta;
pub use flight_client::DoExchangeParams;
pub use flight_client::FlightClient;
pub use flight_client::FlightExchange;
pub use flight_client::FlightReceiver;
pub use flight_client::FlightSender;
pub use keep_alive::build_keep_alive_config;
pub use packets::DataPacket;
pub use packets::DataflowDiagramBuilder;
pub use packets::FragmentData;
pub use packets::NodePerfCounters;
pub use packets::QueryEnv;
pub use packets::QueryFragment;
pub use packets::QueryFragments;
pub use scatter::FlightScatter;

pub use crate::servers::flight::v1::actions::KILL_QUERY;
pub use crate::servers::flight::v1::actions::SET_PRIORITY;
pub use crate::servers::flight::v1::actions::SYSTEM_ACTION;
pub use crate::servers::flight::v1::actions::TRUNCATE_TABLE;
