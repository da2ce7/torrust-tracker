use std::sync::Arc;

use log::info;
use tokio::task::JoinHandle;

use crate::api::server::{self, ApiServiceSettings};
use crate::tracker::tracker::TorrentTracker;

pub fn start_job(settings: &ApiServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    info!("Starting Torrust API server on: {}", settings.socket);

    let settings = settings.to_owned();

    tokio::spawn(async move {
        server::start(&settings, tracker).await;
    })
}
