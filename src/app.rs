//! Torrust Tracker application.
//!
//! The tracker application has a global configuration for multiple jobs.
//! It's basically a container for other services.
//! It also check constraint and dependencies between services. For example:
//! It's not safe to run a UDP tracker on top of a core public tracker, as UDP trackers
//! do not allow private access to the tracker data.
//!
//! The application is responsible for:
//!
//! - Loading data from the database when it's needed.
//! - Starting some jobs depending on the configuration.
//!
//! Jobs executed always:
//!
//! - Health Check API
//!
//! Optional jobs:
//!
//! - Torrent cleaner: it removes inactive peers and (optionally) peerless torrents.
//! - UDP trackers: the user can enable multiple UDP tracker on several ports.
//! - HTTP trackers: the user can enable multiple HTTP tracker on several ports.
//! - Tracker REST API: the tracker API can be enabled/disabled.
use std::sync::Arc;

use tokio::task::JoinSet;
use torrust_tracker_configuration::Configuration;
use tracing::instrument;

use crate::bootstrap::jobs::{health_check_api, http_tracker, torrent_cleanup, tracker_apis, udp_tracker};
use crate::servers::registar::Registar;
use crate::{core, servers};

/// # Panics
///
/// Will panic if:
///
/// - Can't retrieve tracker keys from database.
/// - Can't load whitelist from database.
#[instrument(skip(config, tracker))]
pub async fn start(config: &Configuration, tracker: Arc<core::Tracker>) -> JoinSet<()> {
    if config.http_api.is_none()
        && (config.udp_trackers.is_none() || config.udp_trackers.as_ref().map_or(true, std::vec::Vec::is_empty))
        && (config.http_trackers.is_none() || config.http_trackers.as_ref().map_or(true, std::vec::Vec::is_empty))
    {
        tracing::warn!("No services enabled in configuration");
    }

    let mut jobs = JoinSet::new();

    let registar = Registar::default();

    // Load peer keys
    if tracker.is_private() {
        tracker
            .load_keys_from_database()
            .await
            .expect("Could not retrieve keys from database.");
    }

    // Load whitelisted torrents
    if tracker.is_listed() {
        tracker
            .load_whitelist_from_database()
            .await
            .expect("Could not load whitelist from database.");
    }

    // Start the UDP blocks
    if let Some(udp_trackers) = &config.udp_trackers {
        for udp_tracker_config in udp_trackers {
            if tracker.is_private() {
                tracing::warn!(
                    "Could not start UDP tracker on: {} while in private mode. UDP is not safe for private trackers!",
                    udp_tracker_config.bind_address
                );
            } else {
                jobs.spawn(udp_tracker::run_job(
                    *udp_tracker_config,
                    tracker.clone(),
                    registar.give_form(),
                ));
            }
        }
    } else {
        tracing::info!("No UDP blocks in configuration");
    }

    // Start the HTTP blocks
    if let Some(http_trackers) = &config.http_trackers {
        for http_tracker_config in http_trackers {
            if let Some(job) = http_tracker::start_job(
                http_tracker_config,
                tracker.clone(),
                registar.give_form(),
                servers::http::Version::V1,
            )
            .await
            {
                jobs.spawn(async move {
                    match job.await {
                        Ok(()) => (),
                        Err(e) => tracing::error!(%e, "failed to shutdown http service"),
                    }
                });
            };
        }
    } else {
        tracing::info!("No HTTP blocks in configuration");
    }

    // Start HTTP API
    if let Some(http_api_config) = &config.http_api {
        if let Some(job) = tracker_apis::start_job(
            http_api_config,
            tracker.clone(),
            registar.give_form(),
            servers::apis::Version::V1,
        )
        .await
        {
            jobs.spawn(async move {
                match job.await {
                    Ok(()) => (),
                    Err(e) => tracing::error!(%e, "failed to shutdown http api service"),
                }
            });
        };
    } else {
        tracing::info!("No API block in configuration");
    }

    // Start runners to remove torrents without peers, every interval
    if config.core.inactive_peer_cleanup_interval > 0 {
        let task = torrent_cleanup::start_job(&config.core, &tracker);
        jobs.spawn(async move {
            match task.await {
                Ok(()) => (),
                Err(e) => tracing::error!(%e, "failed to shutdown cleanup service"),
            }
        });
    }

    // Start Health Check API
    let task = health_check_api::start_job(&config.health_check_api, registar.entries()).await;
    jobs.spawn(async move {
        match task.await {
            Ok(()) => (),
            Err(e) => tracing::error!(%e, "failed to shutdown health check service"),
        }
    });

    jobs
}
