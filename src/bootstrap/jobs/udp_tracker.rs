//! UDP tracker job starter.
//!
//! The [`udp_tracker::start_job`](crate::bootstrap::jobs::udp_tracker::start_job)
//! function starts a new UDP tracker server.
//!
//! > **NOTICE**: that the application can launch more than one UDP tracker
//! > on different ports. Refer to the [configuration documentation](https://docs.rs/torrust-tracker-configuration)
//! > for the configuration options.
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use tokio::sync::oneshot;
use tokio::task::JoinError;
use torrust_tracker_configuration::UdpTracker;
use tracing::instrument;

use crate::core;
use crate::servers::registar::ServiceRegistrationForm;
use crate::servers::signals::Halted;
use crate::servers::udp::server::launcher::Launcher;
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
#[instrument(skip(config, tracker, form))]
pub fn start_job<'a>(
    config: UdpTracker,
    tracker: Arc<core::Tracker>,
    form: ServiceRegistrationForm,
) -> (BoxFuture<'a, Result<(), JoinError>>, oneshot::Sender<Halted>) {
    let stopped = Server::new(Launcher::new(tracker, config.bind_address));

    let mut running = match stopped.start(form) {
        Ok(running) => running,
        Err(e) => {
            tracing::error!(%e, "failed to start service");
            panic!("failed to start service")
        }
    };

    let halt = running.state.halt_task.take().expect("it should have halt channel");
    let fut = running.map_ok(|_| ()).boxed();

    (fut, halt)
}
