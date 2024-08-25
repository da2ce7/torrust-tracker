//! UDP tracker job starter.
//!
//! The [`udp_tracker::start_job`](crate::bootstrap::jobs::udp_tracker::start_job)
//! function starts a new UDP tracker server.
//!
//! > **NOTICE**: that the application can launch more than one UDP tracker
//! > on different ports. Refer to the [configuration documentation](https://docs.rs/torrust-tracker-configuration)
//! > for the configuration options.
use std::sync::Arc;

use torrust_tracker_configuration::UdpTracker;
use tracing::instrument;

use crate::core;
use crate::servers::registar::ServiceRegistrationForm;
use crate::servers::udp::server::spawner::Spawner;
use crate::servers::udp::server::Server;

/// It starts a new UDP server with the provided configuration.
///
/// It spawns a new asynchronous task for the new UDP server.
///
/// # Panics
///
/// It will panic if the API binding address is not a valid socket.
/// It will panic if it is unable to start the UDP service.
/// It will panic if the task did not finish successfully.
#[must_use]
#[allow(clippy::async_yields_async)]
#[instrument(skip(config, tracker, form))]
pub async fn run_job(config: UdpTracker, tracker: Arc<core::Tracker>, form: ServiceRegistrationForm) {
    let stopped = Server::new(Spawner::new(config.bind_address));

    let started = stopped.start(tracker, form).await;

    match started.await {
        Ok(_stopped) => (),
        Err(e) => {
            tracing::error!(%e, "failed to cleanly stop service");
            panic!("failed to cleanly stop service")
        }
    }
}
