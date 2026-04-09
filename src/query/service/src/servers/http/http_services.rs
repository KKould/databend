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

use databend_common_config::GlobalConfig;
use databend_common_exception::ErrorCode;
use poem::EndpointExt;
use poem::Route;
use poem::endpoint::BoxEndpoint;
use poem::get;
use poem::middleware::CatchPanic;
use poem::middleware::CookieJarManager;
use poem::middleware::NormalizePath;
use poem::middleware::TrailingSlash;

use databend_query_http_handler_server as http_handler_server;
use databend_query_server_api::Server;

use crate::servers::http::middleware::EndpointKind;
use crate::servers::http::middleware::HTTPSessionMiddleware;
use crate::servers::http::middleware::PanicHandler;
use crate::servers::http::middleware::json_response;
use crate::servers::http::v1::clickhouse_router;
use crate::servers::http::v1::query_route;

#[derive(Copy, Clone)]
pub enum HttpHandlerKind {
    Query,
    Clickhouse,
}

impl HttpHandlerKind {
    pub fn usage(&self, sock: SocketAddr) -> String {
        match self {
            HttpHandlerKind::Query => {
                format!(
                    r#" curl -u${{USER}} -p${{PASSWORD}}: --request POST '{:?}/v1/query/' --header 'Content-Type: application/json' --data-raw '{{"sql": "SELECT avg(number) FROM numbers(100000000)"}}'
"#,
                    sock,
                )
            }
            HttpHandlerKind::Clickhouse => {
                let json = r#"{"foo": "bar"}"#;
                format!(
                    r#" echo 'create table test(foo string)' | curl -u${{USER}} -p${{PASSWORD}}: '{:?}' --data-binary  @-
echo '{}' | curl -u${{USER}} -p${{PASSWORD}}: '{:?}/?query=INSERT%20INTO%20test%20FORMAT%20JSONEachRow' --data-binary @-"#,
                    sock, json, sock,
                )
            }
        }
    }
}

pub struct HttpHandler {
    inner: Box<http_handler_server::HttpHandlerServer>,
}

impl HttpHandler {
    pub fn create(kind: HttpHandlerKind) -> Box<dyn Server> {
        let config = GlobalConfig::instance();
        let tls_cert_path = config.query.common.http_handler_tls_server_cert.clone();
        let tls_key_path = config.query.common.http_handler_tls_server_key.clone();

        Box::new(HttpHandler {
            inner: http_handler_server::HttpHandlerServer::create(
                Box::new(move |sock| Self::build_router(kind, sock)),
                tls_cert_path,
                tls_key_path,
            ),
        })
    }

    #[allow(clippy::let_with_type_underscore)]
    fn build_router(kind: HttpHandlerKind, sock: SocketAddr) -> BoxEndpoint<'static> {
        let ep_clickhouse = Route::new()
            .nest("/", clickhouse_router())
            .with(HTTPSessionMiddleware::create(kind, EndpointKind::Clickhouse))
            .with(CookieJarManager::new());

        let ep_usage = Route::new().at(
            "/",
            get(poem::endpoint::make_sync(move |_| {
                HttpHandlerKind::Query.usage(sock)
            })),
        );
        let ep_health = Route::new().at("/", get(poem::endpoint::make_sync(move |_| "ok")));

        let ep = match kind {
            HttpHandlerKind::Query => Route::new()
                .at("/", ep_usage)
                .nest("/health", ep_health)
                .nest("/v1", query_route())
                .nest("/clickhouse", ep_clickhouse),
            HttpHandlerKind::Clickhouse => Route::new()
                .nest("/", ep_clickhouse)
                .nest("/health", ep_health),
        };
        ep.with(NormalizePath::new(TrailingSlash::Trim))
            .with(CatchPanic::new().with_handler(PanicHandler::new()))
            .around(json_response)
            .boxed()
    }
}

#[async_trait::async_trait]
impl Server for HttpHandler {
    #[async_backtrace::framed]
    async fn shutdown(&mut self, graceful: bool) {
        self.inner.shutdown(graceful).await;
    }

    #[async_backtrace::framed]
    async fn start(&mut self, listening: SocketAddr) -> Result<SocketAddr, ErrorCode> {
        self.inner.start(listening).await
    }
}
