use std::sync::Arc;

use log::info;
use tokio::task::JoinHandle;

use crate::http::{HttpServer, HttpServiceSettings, TlsServiceSettings};
use crate::tracker::core::TorrentTracker;

pub fn start_http_job(settings: &HttpServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let settings = settings.to_owned();

    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        info!("Starting HTTP Server \"{}\" on: {}", settings.display_name, settings.socket);
        http_tracker.start(settings.socket).await;
    })
}

pub fn start_tls_job(settings: &TlsServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let settings = settings.to_owned();

    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        info!(
            "Starting HTTP Server \"{}\" on: {} (TLS)",
            settings.display_name, settings.socket
        );
        http_tracker
            .start_tls(settings.socket, settings.certificate_file_path, settings.key_file_path)
            .await;
    })
}
