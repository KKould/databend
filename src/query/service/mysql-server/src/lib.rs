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

use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::net::SocketAddr;
use std::sync::Arc;

use databend_common_base::runtime::Runtime;
use databend_common_exception::ErrorCode;
use databend_common_exception::Result;
use databend_query_server_api::ListeningStream;
use databend_query_server_api::Server;
use futures::StreamExt;
use futures::future::AbortHandle;
use futures::future::AbortRegistration;
use futures::future::Abortable;
use itertools::Itertools;
use log::error;
use rustls::ServerConfig;
use rustls_pemfile::certs;
use rustls_pemfile::pkcs8_private_keys;
use rustls_pemfile::rsa_private_keys;
use rustls_pki_types::PrivateKeyDer;
use socket2::TcpKeepalive;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;

pub type MySQLSocketHandler = Arc<
    dyn Fn(TcpStream, TcpKeepalive, Option<Arc<ServerConfig>>, Arc<Runtime>) + Send + Sync,
>;

#[derive(Default)]
pub struct MySQLTlsConfig {
    cert_path: String,
    key_path: String,
}

impl MySQLTlsConfig {
    pub fn new(cert_path: String, key_path: String) -> Self {
        Self {
            cert_path,
            key_path,
        }
    }

    fn enabled(&self) -> bool {
        !self.cert_path.is_empty() && !self.key_path.is_empty()
    }

    pub fn setup(&self) -> Result<Option<ServerConfig>> {
        if !self.enabled() {
            return Ok(None);
        }

        let _ = rustls::crypto::ring::default_provider().install_default();

        let cert = certs(&mut BufReader::new(File::open(&self.cert_path)?))
            .try_collect()
            .map_err(|err| ErrorCode::TLSConfigurationFailure(err.to_string()))?;

        let key = {
            let mut pkcs8: Vec<_> =
                pkcs8_private_keys(&mut BufReader::new(File::open(&self.key_path)?))
                    .try_collect()
                    .map_err(|err| ErrorCode::TLSConfigurationFailure(err.to_string()))?;
            if !pkcs8.is_empty() {
                PrivateKeyDer::Pkcs8(pkcs8.remove(0))
            } else {
                let mut rsa: Vec<_> =
                    rsa_private_keys(&mut BufReader::new(File::open(&self.key_path)?))
                        .try_collect()
                        .map_err(|err| ErrorCode::TLSConfigurationFailure(err.to_string()))?;
                if !rsa.is_empty() {
                    PrivateKeyDer::Pkcs1(rsa.remove(0))
                } else {
                    return Err(ErrorCode::TLSConfigurationFailure(
                        "invalid key".to_string(),
                    ));
                }
            }
        };

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key)
            .map_err(|err| ErrorCode::TLSConfigurationFailure(err.to_string()))?;

        Ok(Some(config))
    }
}

pub struct MySQLHandler {
    abort_handle: AbortHandle,
    abort_registration: Option<AbortRegistration>,
    join_handle: Option<JoinHandle<()>>,
    keepalive: TcpKeepalive,
    tls: Option<Arc<ServerConfig>>,
    socket_handler: MySQLSocketHandler,
}

impl MySQLHandler {
    pub fn create(
        tcp_keepalive_timeout_secs: u64,
        tls_config: MySQLTlsConfig,
        socket_handler: MySQLSocketHandler,
    ) -> Result<Box<MySQLHandler>> {
        let (abort_handle, registration) = AbortHandle::new_pair();
        let keepalive = TcpKeepalive::new()
            .with_time(std::time::Duration::from_secs(tcp_keepalive_timeout_secs));
        let tls = tls_config.setup()?.map(Arc::new);

        Ok(Box::new(MySQLHandler {
            abort_handle,
            abort_registration: Some(registration),
            join_handle: None,
            keepalive,
            tls,
            socket_handler,
        }))
    }

    #[async_backtrace::framed]
    async fn listener_tcp(listening: SocketAddr) -> Result<(TcpListenerStream, SocketAddr)> {
        let listener = tokio::net::TcpListener::bind(listening)
            .await
            .map_err(|e| {
                ErrorCode::TokioError(format!("{{{}:{}}} {}", listening.ip(), listening.port(), e))
            })?;
        let listener_addr = listener.local_addr()?;
        Ok((TcpListenerStream::new(listener), listener_addr))
    }

    fn listen_loop(
        &self,
        stream: ListeningStream,
        rt: Arc<Runtime>,
    ) -> impl Future<Output = ()> + use<> {
        let keepalive = self.keepalive.clone();
        let tls = self.tls.clone();
        let socket_handler = self.socket_handler.clone();

        stream.for_each(move |accept_socket| {
            let tls = tls.clone();
            let keepalive = keepalive.clone();
            let executor = rt.clone();
            let socket_handler = socket_handler.clone();
            async move {
                match accept_socket {
                    Err(error) => error!("Broken session connection: {}", error),
                    Ok(socket) => socket_handler(socket, keepalive, tls, executor),
                };
            }
        })
    }
}

#[async_trait::async_trait]
impl Server for MySQLHandler {
    #[async_backtrace::framed]
    async fn shutdown(&mut self, graceful: bool) {
        if !graceful {
            return;
        }

        self.abort_handle.abort();

        if let Some(join_handle) = self.join_handle.take() {
            if let Err(error) = join_handle.await {
                error!(
                    "Unexpected error during shutdown MySQLHandler. cause {}",
                    error
                );
            }
        }
    }

    #[async_backtrace::framed]
    async fn start(&mut self, listening: SocketAddr) -> Result<SocketAddr> {
        match self.abort_registration.take() {
            None => Err(ErrorCode::Internal("MySQLHandler already running.")),
            Some(registration) => {
                let rejected_rt = Arc::new(Runtime::with_worker_threads(
                    1,
                    Some("mysql-handler".to_string()),
                )?);
                let (stream, listener) = Self::listener_tcp(listening).await?;
                let stream = Abortable::new(stream, registration);
                self.join_handle = Some(databend_common_base::runtime::spawn(
                    self.listen_loop(stream, rejected_rt),
                ));
                Ok(listener)
            }
        }
    }
}
