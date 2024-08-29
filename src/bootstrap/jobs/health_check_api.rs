//! Health Check API job starter.
//!
//! The [`health_check_api::start_job`](crate::bootstrap::jobs::health_check_api::start_job)
//! function starts the Health Check REST API.
//!
//! The [`health_check_api::start_job`](crate::bootstrap::jobs::health_check_api::start_job)  
//! function spawns a new asynchronous task, that tasks is the "**launcher**".
//! The "**launcher**" starts the actual server and sends a message back
//! to the main application.
//!
//! The "**launcher**" is an intermediary thread that decouples the Health Check
//! API server from the process that handles it.
//!
//! Refer to the [configuration documentation](https://docs.rs/torrust-tracker-configuration)
//! for the API configuration options.

use tokio::sync::oneshot;
use tokio::task::JoinSet;
use torrust_tracker_configuration::HealthCheckApi;
use tracing::instrument;

use super::Started;
use crate::registry::Registry;
use crate::servers::health_check_api::{server, HEALTH_CHECK_API_LOG_TARGET};
use crate::servers::logging::STARTED_ON;
use crate::servers::signals::Halted;

/// This function starts a new Health Check API server with the provided
/// configuration.
///
/// The functions starts a new concurrent task that will run the API server.
/// This task will send a message to the main application process to notify
/// that the API server was successfully started.
///
/// # Panics
///
/// It would panic if unable to send the  `ApiServerJobStarted` notice.
#[instrument(skip(config, register))]
pub async fn run_job(config: HealthCheckApi, register: Registry) {
    let bind_addr = config.bind_address;

    let (tx_start, rx_start) = oneshot::channel::<Started>();
    let (tx_halt, rx_halt) = tokio::sync::oneshot::channel::<Halted>();

    tracing::info!(target: HEALTH_CHECK_API_LOG_TARGET, "Starting on: http://{}", bind_addr);

    let mut tasks = JoinSet::new();

    // Run the API server
    let () = match server::start(bind_addr, tx_start, rx_halt, register, &mut tasks) {
        Ok(()) => (),
        Err(e) => {
            tracing::error!(%e, "failed to start service");
            panic!("failed to start health check api service")
        }
    };

    // Wait until the server sends the started message
    let local_addr = match rx_start.await {
        Ok(started) => started.local_addr,
        Err(e) => panic!("the Health Check API server was dropped: {e}"),
    };

    tracing::info!(target: HEALTH_CHECK_API_LOG_TARGET, "{STARTED_ON}: http://{local_addr}");

    assert!(!tx_halt.is_closed(), "Halt channel for Health Check API should be open");

    while let Some(task) = tasks.join_next().await {
        match task {
            Ok(Ok(())) => (),
            Ok(Err(e)) => {
                tracing::error!(%e, %local_addr, "task flailed with error");
                panic!("task flailed with error")
            }
            Err(e) => {
                tracing::error!(%e, %local_addr, "failed to cleanly join task");
                panic!("failed to cleanly join task")
            }
        }
    }

    tracing::info!(target: HEALTH_CHECK_API_LOG_TARGET, "Stopped server running on: http:://{local_addr}");
}
