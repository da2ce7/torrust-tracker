use std::net::SocketAddr;

use thiserror::Error;
use torrust_tracker_services::registration::Registration;

use crate::servers::signals::Halted;

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
    FailedToReceiveStartedMessage(tokio::sync::oneshot::error::RecvError),
    #[error("Failed to register service")]
    FailedToRegisterService(Registration),
    #[error("Failed to deregister service")]
    FailedToDeregisterService(SocketAddr),
    #[error("Tasks should not panic")]
    TaskPanicked(tokio::task::JoinError),
    #[error("Already tried to stop service")]
    NotStarted,
    #[error("Failed to send halt signal")]
    FailedToSendStop(Halted),
    #[error("It should have a task when running.")]
    UnexpectedEmptyTasks,
    #[error("It should not panic when running.")]
    TaskUnexpectedlyPanicked(tokio::task::JoinError),
    #[error("It should run until asked to stop.")]
    TaskUnexpectedlyQuit,
    #[error("It should be able to bind to a socket")]
    FailedToBindToSocket(std::io::Error),
    #[error("It should be able to obtain the local address of the bound socket")]
    FailedToObtainLocalAddress(std::io::Error),
}
