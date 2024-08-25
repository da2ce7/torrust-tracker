use std::net::SocketAddr;
use std::sync::Arc;

use derive_more::Constructor;
use futures_util::StreamExt;
use tokio::select;
use tokio::sync::oneshot;
use tracing::instrument;

use super::bound_socket::BoundSocket;
use super::request_buffer::ActiveRequests;
use crate::bootstrap::jobs::Started;
use crate::core::Tracker;
use crate::servers::logging::STARTED_ON;
use crate::servers::registar::ServiceHealthCheckJob;
use crate::servers::signals::{shutdown_signal_with_message, Halted};
use crate::servers::udp::server::processor::Processor;
use crate::servers::udp::server::receiver::Receiver;
use crate::servers::udp::UDP_TRACKER_LOG_TARGET;
use crate::shared::bit_torrent::tracker::udp::client::check;

/// A UDP server instance launcher.
#[derive(Constructor)]
pub struct Launcher;

impl Launcher {
    /// It starts the UDP server instance with graceful shutdown.
    ///
    /// # Panics
    ///
    /// It panics if unable to bind to udp socket, and get the address from the udp socket.
    /// It also panics if unable to send address of socket.
    #[instrument(skip(tracker, tx_start, rx_halt))]
    pub async fn run_with_graceful_shutdown(
        tracker: Arc<Tracker>,
        socket: BoundSocket,
        tx_start: oneshot::Sender<Started>,
        rx_halt: oneshot::Receiver<Halted>,
    ) {
        let address = socket.local_addr();
        let local_udp_url = socket.url().to_string();

        tracing::info!(target: UDP_TRACKER_LOG_TARGET, "{STARTED_ON}: {local_udp_url}");

        let receiver = Receiver::new(socket.into());

        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (spawning main loop)");

        let running = {
            let local_addr = local_udp_url.clone();
            tokio::spawn(async move {
                tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_addr, "Udp::run_with_graceful_shutdown::task (listening...)");
                let () = Self::run_udp_server_main(receiver, tracker.clone()).await;
            })
        };

        tx_start
            .send(Started { local_addr: address })
            .expect("the UDP Tracker service should not be dropped");

        tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (started)");

        let stop = running.abort_handle();

        let halt_task = tokio::spawn(shutdown_signal_with_message(
            rx_halt,
            format!("Halting UDP Service Bound to Socket: {address}"),
        ));

        select! {
            _ = running => { tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (stopped)"); },
            _ = halt_task => { tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (halting)"); }
        }
        stop.abort();

        tokio::task::yield_now().await; // lets allow the other threads to complete.
    }

    #[must_use]
    #[instrument(skip(binding))]
    pub fn check(binding: &SocketAddr) -> ServiceHealthCheckJob {
        let binding = *binding;
        let info = format!("checking the udp tracker health check at: {binding}");

        let job = tokio::spawn(async move { check(&binding).await });

        ServiceHealthCheckJob::new(binding, info, job)
    }

    #[instrument(skip(receiver, tracker))]
    async fn run_udp_server_main(mut receiver: Receiver, tracker: Arc<Tracker>) {
        let active_requests = &mut ActiveRequests::default();

        let local_addr = format!("udp://{}", receiver.local_addr());

        loop {
            let processor = Processor::new(receiver.socket.clone(), tracker.clone());

            if let Some(req) = {
                tracing::trace!(target: UDP_TRACKER_LOG_TARGET, local_addr, "Udp::run_udp_server (wait for request)");
                receiver.next().await
            } {
                tracing::trace!(target: UDP_TRACKER_LOG_TARGET, local_addr, "Udp::run_udp_server::loop (in)");

                let req = match req {
                    Ok(req) => req,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::Interrupted {
                            tracing::warn!(target: UDP_TRACKER_LOG_TARGET, local_addr, err = %e,  "Udp::run_udp_server::loop (interrupted)");
                            return;
                        }
                        tracing::error!(target: UDP_TRACKER_LOG_TARGET, local_addr, err = %e,  "Udp::run_udp_server::loop break: (got error)");
                        break;
                    }
                };

                // We spawn the new task even if there active requests buffer is
                // full. This could seem counterintuitive because we are accepting
                // more request and consuming more memory even if the server is
                // already busy. However, we "force_push" the new tasks in the
                // buffer. That means, in the worst scenario we will abort a
                // running task to make place for the new task.
                //
                // Once concern could be to reach an starvation point were we
                // are only adding and removing tasks without given them the
                // chance to finish. However, the buffer is yielding before
                // aborting one tasks, giving it the chance to finish.

                let abort_handle: tokio::task::AbortHandle = tokio::spawn(async move {
                    tracing::trace!(?req, "processing...");
                    processor.process_request(req).await;
                })
                .abort_handle();

                if abort_handle.is_finished() {
                    continue;
                }

                active_requests.force_push(abort_handle, &local_addr).await;
            } else {
                tokio::task::yield_now().await;

                // the request iterator returned `None`.
                tracing::error!(target: UDP_TRACKER_LOG_TARGET, local_addr, "Udp::run_udp_server breaking: (ran dry, should not happen in production!)");
                break;
            }
        }
    }
}
