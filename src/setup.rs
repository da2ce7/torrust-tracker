use std::sync::Arc;

use log::warn;
use tokio::task::JoinHandle;

use crate::jobs::{http_tracker, torrent_cleanup, tracker_api, udp_tracker};
use crate::settings::old_settings::Settings;
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
    if let Some(trackers) = settings.udp_trackers.as_ref() {
        for udp_tracker_settings in trackers {
            if !udp_tracker_settings.enabled.unwrap_or_default() {
                continue;
            }

            if tracker.is_private() {
                warn!(
                    "Could not start UDP tracker on: {} while in {:?}. UDP is not safe for private trackers!",
                    udp_tracker_settings.bind_address.clone().unwrap(),
                    settings.mode
                );
            } else {
                jobs.push(udp_tracker::start_job(udp_tracker_settings, tracker.clone()))
            }
        }
    }

    // Start the HTTP blocks
    if let Some(trackers) = settings.http_trackers.as_ref() {
        for http_tracker_settings in trackers {
            if !http_tracker_settings.enabled.unwrap_or_default() {
                continue;
            }
            jobs.push(http_tracker::start_job(http_tracker_settings, tracker.clone()));
        }
    }

    if let Some(api) = settings.http_api.as_ref() {
        if api.enabled.unwrap() {
            jobs.push(tracker_api::start_job(settings, tracker.clone()));
        }
    }

    // Start HTTP API server

    // Remove torrents without peers, every interval
    if settings.inactive_peer_cleanup_interval.unwrap() > 0 {
        jobs.push(torrent_cleanup::start_job(settings, tracker.clone()));
    }

    jobs
}
