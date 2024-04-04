//! This module contains functions to handle signals.

use derive_more::Display;
use tracing::info;

/// This is the message that the "launcher" spawned task receives from the main
/// application process to notify the service to shutdown.
///
#[derive(Copy, Clone, Debug, Display, PartialEq, Eq)]
pub enum Halted {
    Normal,
    Dropped,
}

/// Resolves on `ctrl_c` or the `terminate` signal.
///
/// # Panics
///
/// Will panic if the `ctrl_c` or `terminate` signal resolves with an error.
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
        () = ctrl_c => {},
        () = terminate => {}
    }
}

/// Resolves when the `stop_receiver` or the `global_shutdown_signal()` resolves.
///
/// # Panics
///
/// Will panic if the `stop_receiver` resolves with an error.
pub async fn shutdown_signal(rx_halt: tokio::sync::oneshot::Receiver<Halted>) {
    let halt = async {
        match rx_halt.await {
            Ok(signal) => signal,
            Err(err) => panic!("Failed to install stop signal: {err}"),
        }
    };

    tokio::select! {
        signal = halt => { info!("Halt signal processed: {}", signal) },
        () = global_shutdown_signal() => { info!("Global shutdown signal processed") }
    }
}

/// Same as `shutdown_signal()`, but shows a message when it resolves.
pub async fn shutdown_signal_with_message(rx_halt: tokio::sync::oneshot::Receiver<Halted>, message: String) {
    shutdown_signal(rx_halt).await;

    info!("{message}");
}
