use std::sync::Arc;

use log::{error, info, warn};
use tokio::task::JoinHandle;

use crate::tracker::tracker::TorrentTracker;
use crate::udp::{UdpServer, UdpServiceSettings};

pub fn start_job(settings: &UdpServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let settings = settings.to_owned();

    tokio::spawn(async move {
        match UdpServer::new(tracker, &settings.socket).await {
            Ok(udp_server) => {
                info!("Starting UDP server on: {}", settings.socket);
                udp_server.start().await;
            }
            Err(e) => {
                warn!("Could not start UDP tracker on: {}", settings.socket);
                error!("{}", e);
            }
        }
    })
}
