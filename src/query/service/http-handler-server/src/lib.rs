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
use std::time::Duration;

use anyerror::AnyError;
use databend_common_exception::ErrorCode;
use databend_common_http::HttpError;
use databend_common_http::HttpShutdownHandler;
use databend_query_server_api::Server;
use log::info;
use poem::endpoint::BoxEndpoint;
use poem::listener::OpensslTlsConfig;

pub type RouteFactory = Box<dyn Fn(SocketAddr) -> BoxEndpoint<'static> + Send + Sync>;

pub struct HttpHandlerServer {
    route_factory: RouteFactory,
    tls_cert_path: String,
    tls_key_path: String,
    shutdown_handler: HttpShutdownHandler,
}

impl HttpHandlerServer {
    pub fn create(
        route_factory: RouteFactory,
        tls_cert_path: String,
        tls_key_path: String,
    ) -> Box<HttpHandlerServer> {
        Box::new(HttpHandlerServer {
            route_factory,
            tls_cert_path,
            tls_key_path,
            shutdown_handler: HttpShutdownHandler::create("http handler".to_string()),
        })
    }

    fn build_tls(&self) -> Result<OpensslTlsConfig, std::io::Error> {
        let cfg = OpensslTlsConfig::new()
            .cert_from_file(self.tls_cert_path.as_str())
            .key_from_file(self.tls_key_path.as_str());
        Ok(cfg)
    }

    #[async_backtrace::framed]
    async fn start_with_tls(&mut self, listening: SocketAddr) -> Result<SocketAddr, HttpError> {
        info!("Http Handler TLS enabled");

        let tls_config = self
            .build_tls()
            .map_err(|e| HttpError::TlsConfigError(AnyError::new(&e)))?;
        let router = (self.route_factory)(listening);

        self.shutdown_handler
            .start_service(
                listening,
                Some(tls_config),
                router,
                Some(Duration::from_millis(1000)),
            )
            .await
    }

    #[async_backtrace::framed]
    async fn start_without_tls(&mut self, listening: SocketAddr) -> Result<SocketAddr, HttpError> {
        let router = (self.route_factory)(listening);
        self.shutdown_handler
            .start_service(listening, None, router, Some(Duration::from_millis(1000)))
            .await
    }
}

#[async_trait::async_trait]
impl Server for HttpHandlerServer {
    #[async_backtrace::framed]
    async fn shutdown(&mut self, graceful: bool) {
        self.shutdown_handler.shutdown(graceful).await;
    }

    #[async_backtrace::framed]
    async fn start(&mut self, listening: SocketAddr) -> Result<SocketAddr, ErrorCode> {
        let res = if self.tls_key_path.is_empty() || self.tls_cert_path.is_empty() {
            self.start_without_tls(listening).await
        } else {
            self.start_with_tls(listening).await
        };

        res.map_err(|e: HttpError| match e {
            HttpError::BadAddressFormat(any_err) => {
                ErrorCode::BadAddressFormat(any_err.to_string())
            }
            le @ HttpError::ListenError { .. } => ErrorCode::CannotListenerPort(le.to_string()),
            HttpError::TlsConfigError(any_err) => {
                ErrorCode::TLSConfigurationFailure(any_err.to_string())
            }
        })
    }
}
