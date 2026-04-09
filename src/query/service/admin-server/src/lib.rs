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

use databend_common_config::InnerConfig;
use databend_common_exception::ErrorCode;
use databend_common_http::HttpError;
use databend_common_http::HttpShutdownHandler;
use databend_meta_client::types::anyerror::AnyError;
use databend_query_server_api::Server;
use log::info;
use log::warn;
use poem::endpoint::BoxEndpoint;
use poem::listener::OpensslTlsConfig;

pub struct AdminService {
    config: InnerConfig,
    route: Option<BoxEndpoint<'static>>,
    shutdown_handler: HttpShutdownHandler,
}

impl AdminService {
    pub fn create(config: InnerConfig, route: BoxEndpoint<'static>) -> Box<AdminService> {
        Box::new(AdminService {
            config,
            route: Some(route),
            shutdown_handler: HttpShutdownHandler::create("http api".to_string()),
        })
    }

    fn take_route(&mut self) -> Result<BoxEndpoint<'static>, HttpError> {
        self.route.take().ok_or_else(|| {
            HttpError::ListenError {
                message: "admin service already started".to_string(),
                listening: "admin service".to_string(),
            }
        })
    }

    fn build_tls(config: &InnerConfig) -> Result<OpensslTlsConfig, std::io::Error> {
        let cfg = OpensslTlsConfig::new()
            .cert_from_file(config.query.common.api_tls_server_cert.as_str())
            .key_from_file(config.query.common.api_tls_server_key.as_str());
        Ok(cfg)
    }

    #[async_backtrace::framed]
    async fn start_with_tls(&mut self, listening: SocketAddr) -> Result<SocketAddr, HttpError> {
        info!("Http API TLS enabled");

        let tls_config = Self::build_tls(&self.config)
            .map_err(|e| HttpError::TlsConfigError(AnyError::new(&e)))?;
        let route = self.take_route()?;

        self.shutdown_handler
            .start_service(
                listening,
                Some(tls_config),
                route,
                Some(Duration::from_millis(1000)),
            )
            .await
    }

    #[async_backtrace::framed]
    async fn start_without_tls(&mut self, listening: SocketAddr) -> Result<SocketAddr, HttpError> {
        warn!("Http API TLS not set");
        let route = self.take_route()?;

        self.shutdown_handler
            .start_service(listening, None, route, Some(Duration::from_millis(1000)))
            .await
    }
}

#[async_trait::async_trait]
impl Server for AdminService {
    #[async_backtrace::framed]
    async fn shutdown(&mut self, _graceful: bool) {
        // intendfully do nothing: sometimes we hope to diagnose the backtraces or metrics after
        // the process got the sigterm signal, we can still leave the admin service port open until
        // the process exited. it's not an user facing service, it's allowed to force shutdown.
    }

    #[async_backtrace::framed]
    async fn start(&mut self, listening: SocketAddr) -> Result<SocketAddr, ErrorCode> {
        let config = &self.config.query.common;
        let res = if config.api_tls_server_key.is_empty() || config.api_tls_server_cert.is_empty()
        {
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
