use std::sync::Arc;

use log::{error, info, warn};
use tokio::task::JoinHandle;

use crate::settings::UdpTrackerConfig;
use crate::tracker::tracker::TorrentTracker;
use crate::UdpServer;

pub fn start_job(settings: &UdpTrackerConfig, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let bind_addr = settings.bind_address.clone();

    tokio::spawn(async move {
        match UdpServer::new(tracker, &bind_addr.as_ref().unwrap()).await {
            Ok(udp_server) => {
                info!("Starting UDP server on: {}", bind_addr.as_ref().clone().unwrap());
                udp_server.start().await;
            }
            Err(e) => {
                warn!("Could not start UDP tracker on: {}", bind_addr.as_ref().unwrap());
                error!("{}", e);
            }
        }
    })
}
