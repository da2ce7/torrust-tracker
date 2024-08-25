use std::fmt::Debug;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use derive_more::derive::Display;
use derive_more::Constructor;
use tokio::task::JoinSet;
use tracing::{instrument, Level};

use super::spawner::Spawner;
use super::{Server, UdpError};
use crate::bootstrap::jobs::Started;
use crate::core::Tracker;
use crate::servers::registar::{ServiceRegistration, ServiceRegistrationForm};
use crate::servers::signals::Halted;
use crate::servers::udp::server::launcher::Launcher;
use crate::servers::udp::UDP_TRACKER_LOG_TARGET;

/// A UDP server instance controller with no UDP instance running.
#[allow(clippy::module_name_repetitions)]
pub type StoppedUdpServer = Server<Stopped>;

/// A UDP server instance controller with a running UDP instance.
#[allow(clippy::module_name_repetitions)]
pub type RunningUdpServer = Server<Running>;

/// A stopped UDP server state.
#[derive(Debug, Display)]
#[display("Stopped: {spawner}")]
pub struct Stopped {
    pub spawner: Spawner,
}

/// A running UDP server state.
#[derive(Debug, Display, Constructor)]
#[display("Running (with local address): {local_addr}")]
pub struct Running {
    /// The address where the server is bound.
    pub local_addr: SocketAddr,
    pub halt_task: Option<tokio::sync::oneshot::Sender<Halted>>,
    pub task: JoinSet<Spawner>,
}

impl Server<Stopped> {
    /// Creates a new `UdpServer` instance in `stopped`state.
    #[must_use]
    pub fn new(spawner: Spawner) -> Self {
        Self {
            state: Stopped { spawner },
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
    #[allow(clippy::async_yields_async)]
    #[instrument(skip(self, tracker, form), ret(Display, level = Level::INFO))]
    pub async fn start(self, tracker: Arc<Tracker>, form: ServiceRegistrationForm) -> Server<Running> {
        let (tx_start, rx_start) = tokio::sync::oneshot::channel::<Started>();
        let (tx_halt, rx_halt) = tokio::sync::oneshot::channel::<Halted>();

        assert!(!tx_halt.is_closed(), "Halt channel for UDP tracker should be open");

        let mut task = JoinSet::new();

        let _abort_handle = self.state.spawner.spawn_launcher(tracker, tx_start, rx_halt, &mut task);

        let local_addr = rx_start.await.expect("it should be able to start the service").local_addr;

        form.send(ServiceRegistration::new(local_addr, Launcher::check))
            .expect("it should be able to send service registration");

        let running_udp_server: Server<Running> = Server {
            state: Running {
                local_addr,
                halt_task: Some(tx_halt),
                task,
            },
        };

        let local_addr = format!("udp://{local_addr}");
        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, local_addr, "UdpServer<Stopped>::start (running)");

        running_udp_server
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
    type Output = Result<Server<Stopped>, UdpError>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let spawner = match self.state.task.poll_join_next(cx) {
            std::task::Poll::Ready(Some(Ok(spawner))) => spawner,
            std::task::Poll::Ready(Some(Err(e))) => return std::task::Poll::Ready(Err(e.into())),
            std::task::Poll::Ready(None) => panic!("it should not be polled once finished"),
            std::task::Poll::Pending => return std::task::Poll::Pending,
        };

        match self.state.task.poll_join_next(cx) {
            std::task::Poll::Ready(None) => {}
            _ => unreachable!("it should only have a single task"),
        };

        let stopped_api_server: Server<Stopped> = Server {
            state: Stopped { spawner },
        };

        std::task::Poll::Ready(Ok(stopped_api_server))
    }
}
