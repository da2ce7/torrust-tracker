use std::sync::Arc;

use log::info;
use torrust_tracker::databases::database;
use torrust_tracker::settings::{TrackerSettings, TrackerSettingsBuilder};
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
    let settings: TrackerSettings = match TrackerSettingsBuilder::default().try_into() {
        Ok(settings) => settings,
        Err(error) => {
            panic!("{error:?}")
        }
    };

    // Initialize stats tracker
    let stats_tracker = StatsTracker::new_instance(settings.common.as_ref().unwrap().enable_tracker_usage_statistics.unwrap());

    // Initialize database
    let database = match database::connect_database(&settings.database.unwrap()) {
        Ok(database) => database,
        Err(error) => {
            panic!("{}", error)
        }
    };

    // Initialize Torrust tracker
    let tracker = match TorrentTracker::new(
        &Arc::new(settings.global.as_ref().unwrap().to_owned()),
        &Arc::new(settings.common.as_ref().unwrap().to_owned()),
        Box::new(stats_tracker),
        database,
    ) {
        Ok(tracker) => Arc::new(tracker),
        Err(error) => {
            panic!("{}", error)
        }
    };

    // Initialize logging
    logging::setup_logging(&settings.global.as_ref().unwrap());

    // Run jobs
    let jobs = setup::setup(&settings.service.unwrap(), tracker.clone()).await;

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
