use std::sync::Arc;

use log::info;
use tokio::task::JoinHandle;

use crate::http::{HttpServer, HttpServiceSettings};
use crate::tracker::core::TorrentTracker;

pub fn start_http_job(settings: &HttpServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let settings = settings.to_owned();

    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        info!("Starting HTTP Server \"{}\" on: {}", settings.display_name, settings.socket);
        http_tracker.start(settings.socket).await;
    })
}
