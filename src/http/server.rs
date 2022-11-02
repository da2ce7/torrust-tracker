use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use crate::errors::ServiceSettingsError;
use crate::http::routes;
use crate::settings::ServiceSettings;
use crate::tracker::tracker::TorrentTracker;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct HttpServiceSettings {
    pub id: String,
    pub display_name: String,
    pub socket: SocketAddr,
}

impl TryFrom<(&String, &ServiceSettings)> for HttpServiceSettings {
    type Error = ServiceSettingsError;

    fn try_from(_value: (&String, &ServiceSettings)) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TlsServiceSettings {
    pub id: String,
    pub display_name: String,
    pub socket: SocketAddr,
    pub certificate_file_path: PathBuf,
    pub key_file_path: PathBuf,
}

impl TryFrom<(&String, &ServiceSettings)> for TlsServiceSettings {
    type Error = ServiceSettingsError;

    fn try_from(_value: (&String, &ServiceSettings)) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// Server that listens on HTTP, needs a TorrentTracker
#[derive(Clone)]
pub struct HttpServer {
    tracker: Arc<TorrentTracker>,
}

impl HttpServer {
    pub fn new(tracker: Arc<TorrentTracker>) -> HttpServer {
        HttpServer { tracker }
    }

    /// Start the HttpServer
    pub fn start(&self, socket_addr: SocketAddr) -> impl warp::Future<Output = ()> {
        let (_addr, server) = warp::serve(routes(self.tracker.clone())).bind_with_graceful_shutdown(socket_addr, async move {
            tokio::signal::ctrl_c().await.expect("Failed to listen to shutdown signal.");
        });

        server
    }

    /// Start the HttpServer in TLS mode
    pub fn start_tls(
        &self,
        socket_addr: SocketAddr,
        ssl_cert_path: PathBuf,
        ssl_key_path: PathBuf,
    ) -> impl warp::Future<Output = ()> {
        let (_addr, server) = warp::serve(routes(self.tracker.clone()))
            .tls()
            .cert_path(ssl_cert_path)
            .key_path(ssl_key_path)
            .bind_with_graceful_shutdown(socket_addr, async move {
                tokio::signal::ctrl_c().await.expect("Failed to listen to shutdown signal.");
            });

        server
    }
}
