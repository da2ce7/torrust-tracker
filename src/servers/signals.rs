//! This module contains functions to handle signals.
use std::time::{Duration, Instant};

use derive_more::Display;
use tokio::time::sleep;
use tracing::instrument;

/// This is the message that the "launcher" spawned task receives from the main
/// application process to notify the service to shutdown.
///
#[derive(Copy, Clone, Debug, Display)]
pub enum Halted {
    Normal,
}

/// Resolves on `ctrl_c` or the `terminate` signal.
///
/// # Panics
///
/// Will panic if the `ctrl_c` or `terminate` signal resolves with an error.
#[instrument(skip())]
pub async fn global_shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {tracing::warn!("caught interrupt signal (ctrl-c), halting...");},
        () = terminate => {tracing::warn!("caught interrupt signal (terminate), halting...");}
    }
}

/// Resolves when the `stop_receiver` or the `global_shutdown_signal()` resolves.
///
/// # Panics
///
/// Will panic if the `stop_receiver` resolves with an error.
#[instrument(skip(rx_halt))]
pub async fn shutdown_signal(rx_halt: tokio::sync::oneshot::Receiver<Halted>) {
    let halt = async {
        match rx_halt.await {
            Ok(signal) => signal,
            Err(err) => panic!("Failed to install stop signal: {err}"),
        }
    };

    tokio::select! {
        signal = halt => { tracing::debug!("Halt signal processed: {}", signal) },
        () = global_shutdown_signal() => { tracing::debug!("Global shutdown signal processed") }
    }
}

/// Same as `shutdown_signal()`, but shows a message when it resolves.
#[instrument(skip(rx_halt))]
pub async fn shutdown_signal_with_message(rx_halt: tokio::sync::oneshot::Receiver<Halted>, message: String) {
    shutdown_signal(rx_halt).await;

    tracing::info!("{message}");
}

#[instrument(skip(handle, rx_halt, message))]
pub async fn graceful_shutdown(
    handle: axum_server::Handle,
    rx_halt: tokio::sync::oneshot::Receiver<Halted>,
    message: String,
    timeout: Duration,
) {
    shutdown_signal_with_message(rx_halt, message).await;

    tracing::debug!("Sending graceful shutdown signal");
    handle.graceful_shutdown(Some(timeout));

    let now = Instant::now();

    if handle.connection_count() == 0 {
        tracing::debug!("no active connections... shutting down");
    } else {
        loop {
            if handle.connection_count() == 0 {
                tracing::debug!("no more connections... shutting down");
                break;
            }

            if now.elapsed() > timeout {
                tracing::warn!(remaining_alive_connections= %handle.connection_count(), "timed out... shutting down");
                break;
            }
            tracing::info!(remaining_alive_connections= %handle.connection_count(), elapsed= ?now.elapsed(), "waiting...");

            sleep(Duration::from_secs(1)).await;
        }
    }

    handle.shutdown();
}
