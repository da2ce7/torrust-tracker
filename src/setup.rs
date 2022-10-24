use std::sync::Arc;

use log::warn;
use tokio::task::JoinHandle;

use crate::jobs::{http_tracker, torrent_cleanup, tracker_api, udp_tracker};
use crate::settings::Settings;
use crate::tracker::tracker::TorrentTracker;

pub async fn setup(settings: &Settings, tracker: Arc<TorrentTracker>) -> Vec<JoinHandle<()>> {
    let mut jobs: Vec<JoinHandle<()>> = Vec::new();

    // Load peer keys
    if tracker.is_private() {
        tracker.load_keys().await.expect("Could not retrieve keys from database.");
    }

    // Load whitelisted torrents
    if tracker.is_whitelisted() {
        tracker
            .load_whitelist()
            .await
            .expect("Could not load whitelist from database.");
    }

    // Start the UDP blocks
    for udp_tracker_settings in &settings.udp_trackers {
        if !udp_tracker_settings.enabled {
            continue;
        }

        if tracker.is_private() {
            warn!(
                "Could not start UDP tracker on: {} while in {:?}. UDP is not safe for private trackers!",
                udp_tracker_settings.bind_address, settings.mode
            );
        } else {
            jobs.push(udp_tracker::start_job(&udp_tracker_settings, tracker.clone()))
        }
    }

    // Start the HTTP blocks
    for http_tracker_settings in &settings.http_trackers {
        if !http_tracker_settings.enabled {
            continue;
        }
        jobs.push(http_tracker::start_job(&http_tracker_settings, tracker.clone()));
    }

    // Start HTTP API server
    if settings.http_api.enabled {
        jobs.push(tracker_api::start_job(&settings, tracker.clone()));
    }

    // Remove torrents without peers, every interval
    if settings.inactive_peer_cleanup_interval > 0 {
        jobs.push(torrent_cleanup::start_job(&settings, tracker.clone()));
    }

    jobs
}
