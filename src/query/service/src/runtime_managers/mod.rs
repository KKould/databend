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

#[path = "session/client_session_manager.rs"]
mod client_session_manager;
#[path = "query/http_query_manager.rs"]
mod http_query_manager;

pub mod session {
    pub(crate) use databend_query_runtime_managers::session::SessionClaim;
    pub use databend_query_runtime_managers::session::consts;
    pub(crate) use databend_query_runtime_managers::session::unix_ts;

    pub use super::client_session_manager::ClientSessionManager;
}

pub mod query {
    pub(crate) use super::http_query_manager::CloseReason;
    pub use super::http_query_manager::HttpQueryManager;
}

pub use query::HttpQueryManager;
pub use session::ClientSessionManager;
pub(crate) use session::SessionClaim;
pub(crate) use session::unix_ts;
