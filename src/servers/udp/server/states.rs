use std::fmt::Debug;
use std::future::Future;
use std::net::SocketAddr;

use derive_more::derive::Display;
use derive_more::Constructor;
use tokio::task::{JoinError, JoinSet};
use tracing::{instrument, Level};

use super::{Server, UdpError};
use crate::bootstrap::jobs::Started;
use crate::servers::registar::{ServiceRegistration, ServiceRegistrationForm};
use crate::servers::signals::Halted;
use crate::servers::udp::server::launcher::{Finished, Launcher};
use crate::servers::udp::UDP_TRACKER_LOG_TARGET;

/// A UDP server instance controller with no UDP instance running.
#[allow(clippy::module_name_repetitions)]
pub type StoppedUdpServer = Server<Stopped>;

/// A UDP server instance controller with a running UDP instance.
#[allow(clippy::module_name_repetitions)]
pub type RunningUdpServer = Server<Running>;

#[derive(Debug)]
enum Tasks {
    Active(JoinSet<Finished>),
    Shutdown(JoinSet<Finished>),
}

impl Tasks {
    fn make_shutdown(&mut self) {
        let active = match std::mem::replace(self, Tasks::Shutdown(JoinSet::new())) {
            Tasks::Active(active) => active,
            Tasks::Shutdown(_) => panic!("should only be used on active to shutdown"),
        };

        *self = Tasks::Shutdown(active);
    }
}

/// A stopped UDP server state.
#[derive(Debug, Display)]
#[display("Stopped: {launcher}")]
pub struct Stopped {
    pub launcher: Launcher,
}

/// A running UDP server state.
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
    /// Creates a new `UdpServer` instance in `stopped`state.
    #[must_use]
    pub fn new(launcher: Launcher) -> Self {
        Self {
            state: Stopped { launcher },
        }
    }

    /// It starts the server and returns a `UdpServer` controller in `running`
    /// state.
    ///
    /// # Errors
    ///
    /// Will return `Err` if UDP can't bind to given bind address.
    ///
    /// # Panics
    ///
    /// It panics if unable to receive the bound socket address from service.
    ///
    #[instrument(skip(self,  form), err, ret(Display, level = Level::INFO))]
    pub fn start(self, form: ServiceRegistrationForm) -> Result<Server<Running>, UdpError> {
        let (tx_start, rx_start) = std::sync::mpsc::sync_channel::<Started>(0);
        let (tx_halt, rx_halt) = tokio::sync::oneshot::channel::<Halted>();

        assert!(!tx_halt.is_closed(), "Halt channel for UDP tracker should be open");

        let tasks = self
            .state
            .launcher
            .start_with_graceful_shutdown(tx_start, rx_halt)
            .map_err(UdpError::FailedToStart)?;

        let local_addr = rx_start.recv().map_err(UdpError::FailedToReceiveStartedMessage)?.local_addr;

        let local_addr_url = format!("udp://{local_addr}");
        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, local_addr_url, "UdpServer<Stopped>::start (running)");

        form.send(ServiceRegistration::new(local_addr, Launcher::check))
            .map_err(UdpError::FailedToRegisterService)?;

        let running_udp_server: Server<Running> = Server {
            state: Running {
                local_addr,
                halt_task: Some(tx_halt),
                tasks: Tasks::Active(tasks),
                launcher: self.state.launcher,
            },
        };

        Ok(running_udp_server)
    }
}

impl Server<Running> {
    /// It stops the server and returns a `UdpServer` controller in `stopped`
    /// state.
    ///     
    /// # Errors
    ///
    /// Will return [`UdpError::AlreadyStopping`] if the oneshot channel to send the stop signal
    /// has already been called once.
    ///
    /// # Panics
    ///
    /// It panics if unable to shutdown service.
    #[instrument(skip(self), err)]
    pub fn stop(&mut self) -> Result<(), UdpError> {
        self.state
            .halt_task
            .take()
            .ok_or(UdpError::AlreadyStopping)?
            .send(Halted::Normal)
            .map_err(UdpError::FailedToSendStop)
    }
}

impl Future for Server<Running> {
    type Output = Result<Server<Stopped>, JoinError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let () = match &mut self.state.tasks {
            Tasks::Active(ref mut active) => {
                // lets poll until one of the active tasks finishes...

                let () = match active.poll_join_next(cx) {
                    std::task::Poll::Pending => return std::task::Poll::Pending,
                    std::task::Poll::Ready(None) => panic!("it should have at least a single task"),

                    std::task::Poll::Ready(Some(Err(e))) => return std::task::Poll::Ready(Err(e)),

                    std::task::Poll::Ready(Some(Ok(Finished::Main(())))) => tracing::warn!("main task unexpectedly exited!"),
                    std::task::Poll::Ready(Some(Ok(Finished::Shutdown(())))) => tracing::debug!("shutting down"),
                };

                active.abort_all();
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

        self.state.tasks.make_shutdown();
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    }
}

impl Drop for Running {
    #[instrument(fields(%self))]
    fn drop(&mut self) {
        tracing::trace!("dropped");
    }
}
