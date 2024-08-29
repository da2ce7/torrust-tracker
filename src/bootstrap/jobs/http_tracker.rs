//! HTTP tracker job starter.
//!
//! The function [`http_tracker::start_job`](crate::bootstrap::jobs::http_tracker::start_job) starts a new HTTP tracker server.
//!
//! > **NOTICE**: the application can launch more than one HTTP tracker on different ports.
//! > Refer to the [configuration documentation](https://docs.rs/torrust-tracker-configuration) for the configuration options.
//!
//! The [`http_tracker::start_job`](crate::bootstrap::jobs::http_tracker::start_job) function spawns a new asynchronous task,
//! that tasks is the "**launcher**". The "**launcher**" starts the actual server and sends a message back to the main application.
//!
//! The "**launcher**" is an intermediary thread that decouples the HTTP servers from the process that handles it. The HTTP could be used independently in the future.
//! In that case it would not need to notify a parent process.
use std::net::SocketAddr;
use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use futures::future::BoxFuture;
use futures::{FutureExt as _, TryFutureExt as _};
use tokio::sync::oneshot;
use tokio::task::JoinError;
use torrust_tracker_configuration::HttpTracker;
use tracing::instrument;

use super::make_rust_tls;
use crate::core;
use crate::servers::http::server::{Launcher, Server};
use crate::servers::http::Version;
use crate::servers::registar::ServiceRegistrationForm;
use crate::servers::signals::Halted;

/// It starts a new HTTP server with the provided configuration and version.
///
/// Right now there is only one version but in the future we could support more than one HTTP tracker version at the same time.
/// This feature allows supporting breaking changes on `BitTorrent` BEPs.
///
/// # Panics
///
/// It would panic if the `config::HttpTracker` struct would contain inappropriate values.
///
#[instrument(skip(config, tracker, form))]
pub async fn start_job<'a>(
    config: &HttpTracker,
    tracker: Arc<core::Tracker>,
    form: ServiceRegistrationForm,
    version: Version,
) -> (BoxFuture<'a, Result<(), JoinError>>, oneshot::Sender<Halted>) {
    let socket = config.bind_address;

    let tls = make_rust_tls(&config.tsl_config)
        .await
        .map(|tls| tls.expect("it should have a valid http tracker tls configuration"));

    match version {
        Version::V1 => start_v1(socket, tls, tracker.clone(), form),
    }
}

#[allow(clippy::async_yields_async)]
#[instrument(skip(socket, tls, tracker, form))]
fn start_v1<'a>(
    socket: SocketAddr,
    tls: Option<RustlsConfig>,
    tracker: Arc<core::Tracker>,
    form: ServiceRegistrationForm,
) -> (BoxFuture<'a, Result<(), JoinError>>, oneshot::Sender<Halted>) {
    let stopped = Server::new(Launcher::new(tracker, tls, socket));

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use torrust_tracker_test_helpers::configuration::ephemeral_public;

    use crate::bootstrap::app::initialize_with_configuration;
    use crate::bootstrap::jobs::http_tracker::start_job;
    use crate::registry::registar::Registar;
    use crate::servers::http::Version;

    #[tokio::test]
    async fn it_should_start_http_tracker() {
        let cfg = Arc::new(ephemeral_public());
        let http_tracker = cfg.http_trackers.clone().expect("missing HTTP tracker configuration");
        let config = &http_tracker[0];
        let tracker = initialize_with_configuration(&cfg);
        let version = Version::V1;

        start_job(config, tracker, Registar::default().give_form(), version).await;
    }
}
