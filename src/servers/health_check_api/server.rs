//! Logic to run the Health Check HTTP API server.
//!
//! This API is intended to be used by the container infrastructure to check if
//! the whole application is healthy.
use std::net::SocketAddr;
use std::time::Duration;

use axum_server::Handle;
use futures::FutureExt;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio::task::JoinSet;
use tracing::instrument;

use crate::bootstrap::jobs::Started;
use crate::servers::health_check_api::routes::router;
use crate::servers::health_check_api::HEALTH_CHECK_API_LOG_TARGET;
use crate::servers::registar::ServiceRegistry;
use crate::servers::signals::{graceful_shutdown, Halted};

/// Starts Health Check API server.
///
/// # Errors
///
/// It would return an error if unable to bind socket.
/// It would return an error if unable to get local address.
/// It would return an error if the service returns in a error.
#[instrument(skip(bind_to, tx, rx_halt, register, tasks))]
pub fn start(
    bind_to: SocketAddr,
    tx: Sender<Started>,
    rx_halt: Receiver<Halted>,
    register: ServiceRegistry,
    tasks: &mut JoinSet<Result<(), std::io::Error>>,
) -> Result<(), std::io::Error> {
    let socket = std::net::TcpListener::bind(bind_to)?;
    let local_addr = socket.local_addr()?;

    let handle = Handle::new();

    tracing::debug!(target: HEALTH_CHECK_API_LOG_TARGET, "Starting service with graceful shutdown in a spawned task ...");

    tasks.spawn(
        graceful_shutdown(
            handle.clone(),
            rx_halt,
            format!("Shutting down health check api on socket address: {local_addr}"),
            Duration::from_secs(90),
        )
        .map(Ok),
    );

    let router = router(register);

    tasks.spawn(
        axum_server::from_tcp(socket)
            .handle(handle)
            .serve(router.into_make_service_with_connect_info::<SocketAddr>()),
    );

    tx.send(Started { local_addr }).map_err(|message| {
        std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            format!("it could not send message: {message:?}"),
        )
    })
}
