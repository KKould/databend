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

use std::net::SocketAddr;
use std::sync::Arc;

use databend_common_base::runtime::Runtime;
use databend_common_exception::Result;
use databend_query_mysql_server as mysql_server;
use databend_query_mysql_server::MySQLSocketHandler;
use databend_query_mysql_server::MySQLTlsConfig;
use databend_query_server_api::Server;
use log::error;
use socket2::TcpKeepalive;
use tokio::net::TcpStream;

use crate::servers::mysql::mysql_session::MySQLConnection;
use crate::sessions::SessionManager;

pub struct MySQLHandler {
    inner: Box<mysql_server::MySQLHandler>,
}

impl MySQLHandler {
    pub fn create(
        tcp_keepalive_timeout_secs: u64,
        tls_config: MySQLTlsConfig,
    ) -> Result<Box<dyn Server>> {
        let socket_handler: MySQLSocketHandler = Arc::new(
            |socket: TcpStream, keepalive: TcpKeepalive, tls, executor: Arc<Runtime>| {
                let session_manager = SessionManager::instance();
                executor.spawn(async move {
                    if let Err(error) =
                        MySQLConnection::run_on_stream(session_manager, socket, keepalive, tls)
                            .await
                    {
                        error!("Unexpected error occurred during query: {:?}", error);
                    }
                });
            },
        );

        Ok(Box::new(MySQLHandler {
            inner: mysql_server::MySQLHandler::create(
                tcp_keepalive_timeout_secs,
                tls_config,
                socket_handler,
            )?,
        }))
    }
}

#[async_trait::async_trait]
impl Server for MySQLHandler {
    #[async_backtrace::framed]
    async fn shutdown(&mut self, graceful: bool) {
        self.inner.shutdown(graceful).await
    }

    #[async_backtrace::framed]
    async fn start(&mut self, listening: SocketAddr) -> Result<SocketAddr> {
        self.inner.start(listening).await
    }
}
