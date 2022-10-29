use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use crate::http::routes;
use crate::tracker::tracker::TorrentTracker;

/// Server that listens on HTTP, needs a TorrentTracker
#[derive(Clone)]
pub struct HttpServer {
    tracker: Arc<TorrentTracker>,
}

#[derive(Debug)]
pub struct HttpServerSettings {
    pub name: String,
    pub socket: SocketAddr,
    pub tls: Option<TlsSettings>,
}

#[derive(Debug, Clone)]
pub struct TlsSettings {
    pub cert_file_path: PathBuf,
    pub key_file_path: PathBuf,
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
