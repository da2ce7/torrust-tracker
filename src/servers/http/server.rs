//! Module to handle the HTTP server instances.
use std::future::Future;
use std::net::SocketAddr;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
use derive_more::derive::Display;
use derive_more::Constructor;
use futures::FutureExt;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::task::{JoinError, JoinSet};
use tracing::instrument;

use super::v1::routes::router;
use crate::bootstrap::jobs::Started;
use crate::core::Tracker;
use crate::servers::custom_axum_server::{self, TimeoutAcceptor};
use crate::servers::http::HTTP_TRACKER_LOG_TARGET;
use crate::servers::logging::STARTED_ON;
use crate::servers::registar::{ServiceHealthCheckJob, ServiceRegistration, ServiceRegistrationForm};
use crate::servers::signals::{graceful_shutdown, Halted};

/// Error that can occur when starting or stopping the HTTP server.
///
/// Some errors triggered while starting the server are:
///
/// - The spawned server cannot send its `SocketAddr` back to the main thread.
/// - The launcher cannot receive the `SocketAddr` from the spawned server.
///
/// Some errors triggered while stopping the server are:
///
/// - The channel to send the shutdown signal to the server is closed.
/// - The task to shutdown the server on the spawned server failed to execute to
///   completion.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to start service")]
    FailedToStart(std::io::Error),
    #[error("Failed to receive started message")]
    FailedToReceiveStartedMessage(oneshot::error::RecvError),
    #[error("Failed to register service")]
    FailedToRegisterService(ServiceRegistration),
    #[error("Failed to join to service")]
    Join(#[from] JoinError),
    #[error("Already tried to stop service")]
    AlreadyStopping,
    #[error("Failed to send halt signal")]
    FailedToSendStop(Halted),
}

#[derive(Debug)]
enum Tasks {
    Active(JoinSet<Finished>),
    Shutdown(JoinSet<Finished>),
}

impl Tasks {
    fn to_shutdown(&mut self) {
        let active = match std::mem::replace(self, Tasks::Shutdown(JoinSet::new())) {
            Tasks::Active(active) => active,
            Tasks::Shutdown(_) => panic!("should only be used on active to shutdown"),
        };

        *self = Tasks::Shutdown(active);
    }
}

#[derive(Debug, Display)]
pub enum Finished {
    #[display("Main Task")]
    Main(std::io::Result<()>),
    #[display("Main with Tls Task")]
    MainTls(std::io::Result<()>),
    #[display("Shutdown Task")]
    Shutdown(()),
}

#[derive(Constructor, Clone)]
pub struct Launcher {
    tracker: Arc<Tracker>,
    tls: Option<RustlsConfig>,
    bind_to: SocketAddr,
}

impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tls = if self.tls.is_some() { "enabled" } else { "disabled" };

        f.debug_struct("Launcher")
            .field("bind_to", &self.bind_to)
            .field("tls", &tls)
            .field("tracker", &"..")
            .finish()
    }
}

impl std::fmt::Display for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tls = if self.tls.is_some() { "enabled" } else { "disabled" };

        f.write_fmt(format_args!("Launcher with tls {tls}, binding to: {}", self.bind_to))
    }
}

impl Launcher {
    #[instrument(skip(self, tx_start, rx_halt))]
    fn start_with_graceful_shutdown(
        &self,
        tx_start: mpsc::SyncSender<Started>,
        rx_halt: oneshot::Receiver<Halted>,
    ) -> std::io::Result<JoinSet<Finished>> {
        let socket = std::net::TcpListener::bind(self.bind_to).expect("Could not bind tcp_listener to address.");

        let protocol = if self.tls.is_some() { "https" } else { "http" };

        let local_addr = socket.local_addr().expect("Could not get local_addr from tcp_listener.");
        let local_udp_url = format!("{protocol}://{local_addr}");

        let mut tasks = JoinSet::new();

        let handle = Handle::new();

        tasks.spawn(
            graceful_shutdown(
                handle.clone(),
                rx_halt,
                format!("Shutting down HTTP server on socket address: {local_udp_url}"),
                Duration::from_secs(90),
            )
            .map(Finished::Shutdown),
        );

        tracing::info!(target: HTTP_TRACKER_LOG_TARGET, "Starting on: {local_udp_url}");

        let app = router(self.tracker.clone(), local_addr);

        let _abort_handle = match &self.tls {
            Some(tls) => tasks.spawn(
                custom_axum_server::from_tcp_rustls_with_timeouts(socket, tls.clone())
                    .handle(handle)
                    // The TimeoutAcceptor is commented because TSL does not work with it.
                    // See: https://github.com/torrust/torrust-index/issues/204#issuecomment-2115529214
                    //.acceptor(TimeoutAcceptor)
                    .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                    .map(Finished::MainTls),
            ),
            None => tasks.spawn(
                custom_axum_server::from_tcp_with_timeouts(socket)
                    .handle(handle)
                    .acceptor(TimeoutAcceptor)
                    .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                    .map(Finished::Main),
            ),
        };

        tracing::info!(target: HTTP_TRACKER_LOG_TARGET, "{STARTED_ON}: {local_udp_url}");

        let () = tx_start.send(Started { local_addr }).map_err(|msg| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("failed to send start message: {msg:?}"),
            )
        })?;

        Ok(tasks)
    }
}

/// A HTTP server instance controller with no HTTP instance running.
#[allow(clippy::module_name_repetitions)]
pub type StoppedHttpServer = Server<Stopped>;

/// A HTTP server instance controller with a running HTTP instance.
#[allow(clippy::module_name_repetitions)]
pub type RunningHttpServer = Server<Running>;

/// A HTTP server instance controller.
///
/// It's responsible for:
///
/// - Keeping the initial configuration of the server.
/// - Starting and stopping the server.
/// - Keeping the state of the server: `running` or `stopped`.
///
/// It's an state machine. Configurations cannot be changed. This struct
/// represents concrete configuration and state. It allows to start and stop the
/// server but always keeping the same configuration.
///
/// > **NOTICE**: if the configurations changes after running the server it will
/// > reset to the initial value after stopping the server. This struct is not
/// > intended to persist configurations between runs.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Display)]
pub struct Server<S> {
    /// The state of the server: `running` or `stopped`.
    pub state: S,
}

/// A stopped HTTP server state.
#[derive(Debug, Display)]
#[display("Stopped: {launcher}")]
pub struct Stopped {
    launcher: Launcher,
}

/// A running HTTP server state.
#[derive(Debug, Display, Constructor)]
#[display("Running (with local address): {local_addr}")]
pub struct Running {
    /// The address where the server is bound.
    pub local_addr: SocketAddr,
    pub halt_task: Option<tokio::sync::oneshot::Sender<Halted>>,
    tasks: Tasks,
    pub launcher: Launcher,
}

impl Server<Stopped> {
    /// It creates a new `HttpServer` controller in `stopped` state.
    #[must_use]
    pub fn new(launcher: Launcher) -> Self {
        Self {
            state: Stopped { launcher },
        }
    }

    /// It starts the server and returns a `HttpServer` controller in `running`
    /// state.
    ///
    /// # Errors
    ///
    /// It would return an error if no `SocketAddr` is returned after launching the server.
    ///
    /// # Panics
    ///
    /// It would panic spawned HTTP server launcher cannot send the bound `SocketAddr`
    /// back to the main thread.
    pub fn start(self, form: ServiceRegistrationForm) -> std::io::Result<Server<Running>> {
        let (tx_start, rx_start) = std::sync::mpsc::sync_channel::<Started>(0);
        let (tx_halt, rx_halt) = tokio::sync::oneshot::channel::<Halted>();

        let launcher = self.state.launcher;

        let tasks = launcher.start_with_graceful_shutdown(tx_start, rx_halt)?;

        let local_addr = rx_start.recv().expect("it should be able to start the service").local_addr;

        form.send(ServiceRegistration::new(local_addr, check_fn))
            .expect("it should be able to send service registration");

        Ok(Server {
            state: Running {
                local_addr,
                halt_task: Some(tx_halt),
                tasks: Tasks::Active(tasks),
                launcher,
            },
        })
    }
}

impl Server<Running> {
    /// It sends a stop signal to the server.
    /// state.
    ///
    /// # Errors
    ///
    /// It would return an error if the channel for the task killer signal was closed.
    #[instrument(skip(self), err)]
    pub fn stop(&mut self) -> Result<(), Error> {
        self.state
            .halt_task
            .take()
            .ok_or(Error::AlreadyStopping)?
            .send(Halted::Normal)
            .map_err(Error::FailedToSendStop)
    }
}

/// Checks the Health by connecting to the HTTP tracker endpoint.
///
/// # Errors
///
/// This function will return an error if unable to connect.
/// Or if the request returns an error.
#[must_use]
pub fn check_fn(binding: &SocketAddr) -> ServiceHealthCheckJob {
    let url = format!("http://{binding}/health_check"); // DevSkim: ignore DS137138

    let info = format!("checking http tracker health check at: {url}");

    let job = tokio::spawn(async move {
        match reqwest::get(url).await {
            Ok(response) => Ok(response.status().to_string()),
            Err(err) => Err(err.to_string()),
        }
    });

    ServiceHealthCheckJob::new(*binding, info, job)
}

impl Future for Server<Running> {
    type Output = Result<Server<Stopped>, JoinError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let () = match &mut self.state.tasks {
            Tasks::Active(ref mut active) => {
                // lets poll until one of the active tasks finishes...

                let () = match active.poll_join_next(cx) {
                    std::task::Poll::Pending => return std::task::Poll::Pending,
                    std::task::Poll::Ready(Some(Err(e))) => return std::task::Poll::Ready(Err(e)),

                    std::task::Poll::Ready(None) => panic!("it should have at least a single task"),

                    std::task::Poll::Ready(Some(Ok(Finished::Main(Ok(()))))) => tracing::warn!("main task unexpectedly exited!"),
                    std::task::Poll::Ready(Some(Ok(Finished::Main(Err(e))))) => {
                        tracing::warn!(%e, "main task unexpectedly exited with error!");
                    }

                    std::task::Poll::Ready(Some(Ok(Finished::MainTls(Ok(()))))) => {
                        tracing::warn!("main tls task unexpectedly exited!");
                    }
                    std::task::Poll::Ready(Some(Ok(Finished::MainTls(Err(e))))) => {
                        tracing::warn!(%e, "main tls task unexpectedly exited with error!");
                    }

                    std::task::Poll::Ready(Some(Ok(Finished::Shutdown(())))) => tracing::debug!("shutting down"),
                };

                active.abort_all();
                self.state.tasks.to_shutdown();
                cx.waker().wake_by_ref();
                return std::task::Poll::Pending;
            }
            Tasks::Shutdown(ref mut shutdown) => {
                // lets clean up the tasks for shutdown...

                let () = match shutdown.poll_join_next(cx) {
                    std::task::Poll::Pending => return std::task::Poll::Pending,

                    std::task::Poll::Ready(Some(Ok(finished))) => {
                        tracing::trace!(%finished, "task finished when shutting down");
                        cx.waker().wake_by_ref();
                        return std::task::Poll::Pending;
                    }

                    std::task::Poll::Ready(Some(Err(e))) => {
                        tracing::warn!(%e, "task failed to join successfully when shutting down");
                        cx.waker().wake_by_ref();
                        return std::task::Poll::Pending;
                    }
                    std::task::Poll::Ready(None) => {
                        tracing::debug!("finished cleaning up tasks...");
                        return std::task::Poll::Ready(Ok(Server::new(self.state.launcher.clone())));
                    }
                };
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use torrust_tracker_test_helpers::configuration::ephemeral_public;

    use crate::bootstrap::app::initialize_with_configuration;
    use crate::bootstrap::jobs::make_rust_tls;
    use crate::servers::http::server::{Launcher, Server};
    use crate::servers::registar::Registar;

    #[tokio::test]
    async fn it_should_be_able_to_start_and_stop() {
        let cfg = Arc::new(ephemeral_public());
        let tracker = initialize_with_configuration(&cfg);
        let http_trackers = cfg.http_trackers.clone().expect("missing HTTP trackers configuration");
        let config = &http_trackers[0];

        let bind_to = config.bind_address;

        let tls = make_rust_tls(&config.tsl_config)
            .await
            .map(|tls| tls.expect("tls config failed"));

        let register = &Registar::default();

        let stopped = Server::new(Launcher::new(tracker, tls, bind_to));
        let mut started = stopped.start(register.give_form()).expect("it should start the server");
        let () = started.stop().expect("it should stop the server");

        let stopped = started.await.expect("it should not shutdown with an error");

        assert_eq!(stopped.state.launcher.bind_to, bind_to);
    }
}
