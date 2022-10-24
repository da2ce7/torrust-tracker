use std::sync::Arc;

use log::info;
use tokio::task::JoinHandle;

use crate::api::server;
use crate::settings::Settings;
use crate::tracker::tracker::TorrentTracker;

pub fn start_job(settings: &Settings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let bind_addr = settings
        .http_api
        .bind_address
        .parse::<std::net::SocketAddr>()
        .expect("Tracker API bind_address invalid.");
    info!("Starting Torrust API server on: {}", bind_addr);

    tokio::spawn(async move {
        server::start(bind_addr, tracker).await;
    })
}
