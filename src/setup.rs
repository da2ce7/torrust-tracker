use std::collections::HashSet;
use std::sync::Arc;

use log::warn;
use tokio::task::JoinHandle;

use crate::api::server::ApiServiceSettings;
use crate::http::{HttpServiceSettings, TlsServiceSettings};
use crate::jobs::{http_tracker, torrent_cleanup, tracker_api, udp_tracker};
use crate::settings::{ServiceProtocol, Services};
use crate::tracker::tracker::TorrentTracker;
use crate::udp::UdpServiceSettings;

pub async fn setup(services: &Services, tracker: Arc<TorrentTracker>) -> Vec<JoinHandle<()>> {
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

    let mut udp_services: HashSet<UdpServiceSettings> = HashSet::new();
    let mut http_services: HashSet<HttpServiceSettings> = HashSet::new();
    let mut tls_services: HashSet<TlsServiceSettings> = HashSet::new();
    let mut api_services: HashSet<ApiServiceSettings> = HashSet::new();

    for service in services
        .into_iter()
        .filter(|service| service.1.enabled.filter(|t| t.to_owned()).is_some())
    {
        match service.1.service.unwrap() {
            ServiceProtocol::UDP => {
                // Todo: Handel Error.
                let service: UdpServiceSettings = service.try_into().unwrap();

                // Todo: Handel Error. (no duplicates)
                assert!(!udp_services.contains(&service));

                udp_services.insert(service);
            }
            ServiceProtocol::HTTP => {
                // Todo: Handel Error.
                let service: HttpServiceSettings = service.try_into().unwrap();

                // Todo: Handel Error. (no duplicates)
                assert!(!http_services.contains(&service));

                http_services.insert(service);
            }
            ServiceProtocol::TLS => {
                // Todo: Handel Error.
                let service: TlsServiceSettings = service.try_into().unwrap();

                // Todo: Handel Error. (no duplicates)
                assert!(!tls_services.contains(&service));

                tls_services.insert(service);
            }
            ServiceProtocol::API => {
                // Todo: Handel Error.
                let service: ApiServiceSettings = service.try_into().unwrap();

                // Todo: Handel Error. (no duplicates)
                assert!(!api_services.contains(&service));

                api_services.insert(service);
            }
        }
    }

    // Start the UDP blocks
    for service_settings in udp_services {
        if tracker.is_private() {
            warn!(
                "Could not start UDP tracker on: {} while in {:?}. UDP is not safe for private trackers!",
                service_settings.socket.to_owned(),
                tracker.mode
            );
        } else {
            jobs.push(udp_tracker::start_job(&service_settings, tracker.clone()))
        }
    }

    // Start the HTTP blocks
    for service_settings in http_services {
        jobs.push(http_tracker::start_http_job(&service_settings, tracker.clone()))
    }

    // Start the TLS blocks
    for service_settings in tls_services {
        jobs.push(http_tracker::start_tls_job(&service_settings, tracker.clone()))
    }

    // Start the API blocks
    for service_settings in api_services {
        jobs.push(tracker_api::start_job(&service_settings, tracker.clone()))
    }

    // Remove torrents without peers, every interval
    if let Some(interval) = tracker.common.cleanup_inactive_peers_interval_seconds {
        if interval > 0 {
            jobs.push(torrent_cleanup::start_job(interval, tracker.clone()));
        }
    }

    jobs
}
