use std::sync::Arc;

use log::info;
use torrust_tracker::settings::old_settings::Settings;
use torrust_tracker::tracker::statistics::StatsTracker;
use torrust_tracker::tracker::tracker::TorrentTracker;
use torrust_tracker::{ephemeral_instance_keys, logging, setup, static_time};

#[tokio::main]
async fn main() {
    // Set the time of Torrust app starting
    lazy_static::initialize(&static_time::TIME_AT_APP_START);

    // Initialize the Ephemeral Instance Random Seed
    lazy_static::initialize(&ephemeral_instance_keys::RANDOM_SEED);

    // Initialize Torrust Settings
    let settings = match Settings::new() {
        Ok(settings) => Arc::new(settings),
        Err(error) => {
            panic!("{error:?}")
        }
    };

    // Initialize stats tracker
    let stats_tracker = StatsTracker::new_instance(settings.tracker_usage_statistics.unwrap());

    // Initialize Torrust tracker
    let tracker = match TorrentTracker::new(settings.clone(), Box::new(stats_tracker)) {
        Ok(tracker) => Arc::new(tracker),
        Err(error) => {
            panic!("{}", error)
        }
    };

    // Initialize logging
    logging::setup_logging(&settings);

    // Run jobs
    let jobs = setup::setup(&settings, tracker.clone()).await;

    // handle the signals here
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Torrust shutting down..");

            // Await for all jobs to shutdown
            futures::future::join_all(jobs).await;
            info!("Torrust successfully shutdown.");
        }
    }
}
