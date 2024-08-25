use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio::task::JoinSet;
use torrust_tracker::bootstrap::jobs::Started;
use torrust_tracker::servers::health_check_api::{server, HEALTH_CHECK_API_LOG_TARGET};
use torrust_tracker::servers::registar::Registar;
use torrust_tracker::servers::signals::{self, Halted};
use torrust_tracker_configuration::HealthCheckApi;
use tracing::instrument;

#[derive(Debug)]
pub enum Error {
    #[allow(dead_code)]
    Error(String),
}

pub struct Running {
    pub local_addr: SocketAddr,
    pub halt_task: oneshot::Sender<signals::Halted>,
    pub tasks: JoinSet<Result<(), std::io::Error>>,
    pub bind_to: SocketAddr,
}

pub struct Stopped {
    pub bind_to: SocketAddr,
}

pub struct Environment<S> {
    pub registar: Registar,
    pub state: S,
}

impl Environment<Stopped> {
    pub fn new(config: &Arc<HealthCheckApi>, registar: Registar) -> Self {
        let bind_to = config.bind_address;

        Self {
            registar,
            state: Stopped { bind_to },
        }
    }

    /// Start the test environment for the Health Check API.
    /// It runs the API server.
    #[instrument(skip(self))]
    pub async fn start(self) -> Environment<Running> {
        let (tx_start, rx_start) = oneshot::channel::<Started>();
        let (tx_halt, rx_halt) = tokio::sync::oneshot::channel::<Halted>();

        let register = self.registar.entries();
        let bind_to = self.state.bind_to;

        let mut tasks = JoinSet::new();
        server::start(bind_to, tx_start, rx_halt, register, &mut tasks).expect("it should start the health check service");

        tracing::debug!(target: HEALTH_CHECK_API_LOG_TARGET, "Server started. Sending the binding {bind_to} ...");

        tracing::debug!(target: HEALTH_CHECK_API_LOG_TARGET, "Waiting for spawning task to send the binding ...");

        let local_addr = rx_start.await.expect("it should send service binding").local_addr;

        tracing::info!(%local_addr, "started");

        Environment {
            registar: self.registar.clone(),
            state: Running {
                tasks,
                halt_task: tx_halt,
                local_addr,
                bind_to,
            },
        }
    }
}

impl Environment<Running> {
    pub async fn new(config: &Arc<HealthCheckApi>, registar: Registar) -> Self {
        Environment::<Stopped>::new(config, registar).start().await
    }

    pub async fn stop(mut self) -> Result<Environment<Stopped>, Error> {
        self.state
            .halt_task
            .send(Halted::Normal)
            .map_err(|e| Error::Error(e.to_string()))?;

        while let Some(task) = self.state.tasks.join_next().await {
            match task {
                Ok(Ok(())) => (),
                Ok(Err(e)) => {
                    tracing::error!(%e, "task flailed with error");
                    panic!("task flailed with error")
                }
                Err(e) => {
                    tracing::error!(%e, "failed to cleanly join task");
                    panic!("failed to cleanly join task")
                }
            }
        }

        Ok(Environment {
            registar: self.registar.clone(),
            state: Stopped {
                bind_to: self.state.bind_to,
            },
        })
    }
}
