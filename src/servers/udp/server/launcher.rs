use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use aquatic_udp_protocol::Response;
use derive_more::Constructor;
use futures_util::StreamExt;
use tokio::select;
use tokio::sync::oneshot;

use super::request_buffer::ActiveRequests;
use super::RawRequest;
use crate::bootstrap::jobs::Started;
use crate::core::Tracker;
use crate::servers::logging::STARTED_ON;
use crate::servers::registar::ServiceHealthCheckJob;
use crate::servers::signals::{shutdown_signal_with_message, Halted};
use crate::servers::udp::server::bound_socket::BoundSocket;
use crate::servers::udp::server::receiver::Receiver;
use crate::servers::udp::{handlers, UDP_TRACKER_LOG_TARGET};
use crate::shared::bit_torrent::tracker::udp::client::check;
use crate::shared::bit_torrent::tracker::udp::MAX_PACKET_SIZE;

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
    pub async fn run_with_graceful_shutdown(
        tracker: Arc<Tracker>,
        bind_to: SocketAddr,
        tx_start: oneshot::Sender<Started>,
        rx_halt: oneshot::Receiver<Halted>,
    ) {
        let halt_task = tokio::task::spawn(shutdown_signal_with_message(
            rx_halt,
            format!("Halting UDP Service Bound to Socket: {bind_to}"),
        ));

        tracing::info!(target: UDP_TRACKER_LOG_TARGET, "Starting on: {bind_to}");

        let socket = tokio::time::timeout(Duration::from_millis(5000), BoundSocket::new(bind_to))
            .await
            .expect("it should bind to the socket within five seconds");

        let bound_socket = match socket {
            Ok(socket) => socket,
            Err(e) => {
                tracing::error!(target: UDP_TRACKER_LOG_TARGET, addr = %bind_to, err = %e, "Udp::run_with_graceful_shutdown panic! (error when building socket)" );
                panic!("could not bind to socket!");
            }
        };

        let address = bound_socket.address();
        let local_udp_url = bound_socket.url().to_string();

        tracing::info!(target: UDP_TRACKER_LOG_TARGET, "{STARTED_ON}: {local_udp_url}");

        let receiver = Receiver::new(bound_socket.into());

        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (spawning main loop)");

        let running = {
            let local_addr = local_udp_url.clone();
            tokio::task::spawn(async move {
                tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_addr, "Udp::run_with_graceful_shutdown::task (listening...)");
                let () = Self::run_udp_server_main(receiver, tracker.clone()).await;
            })
        };

        tx_start
            .send(Started { address })
            .expect("the UDP Tracker service should not be dropped");

        tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (started)");

        let stop = running.abort_handle();

        select! {
            _ = running => { tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (stopped)"); },
            _ = halt_task => { tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_udp_url, "Udp::run_with_graceful_shutdown (halting)"); }
        }
        stop.abort();

        tokio::task::yield_now().await; // lets allow the other threads to complete.
    }

    #[must_use]
    pub fn check(binding: &SocketAddr) -> ServiceHealthCheckJob {
        let binding = *binding;
        let info = format!("checking the udp tracker health check at: {binding}");

        let job = tokio::spawn(async move { check(&binding).await });

        ServiceHealthCheckJob::new(binding, info, job)
    }

    async fn run_udp_server_main(mut receiver: Receiver, tracker: Arc<Tracker>) {
        let reqs = &mut ActiveRequests::default();

        let addr = receiver.bound_socket_address();
        let local_addr = format!("udp://{addr}");

        loop {
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

                /* code-review:

                  Does it make sense to spawn a new request processor task when
                  the ActiveRequests buffer is full?

                  We could store the UDP request in a secondary buffer and wait
                  until active tasks are finished. When a active request is finished
                  we can move a new UDP request from the pending to process requests
                  buffer to the active requests buffer.

                  This forces us to define an explicit timeout for active requests.

                  In the current solution the timeout is dynamic, it depends on
                  the system load. With high load we can remove tasks without
                  giving them enough time to be processed. With low load we could
                  keep processing running longer than a reasonable time for
                  the client to receive the response.

                */

                let abort_handle =
                    tokio::task::spawn(Launcher::process_request(req, tracker.clone(), receiver.bound_socket.clone()))
                        .abort_handle();

                if abort_handle.is_finished() {
                    continue;
                }

                reqs.force_push(abort_handle, &local_addr).await;
            } else {
                tokio::task::yield_now().await;
                // the request iterator returned `None`.
                tracing::error!(target: UDP_TRACKER_LOG_TARGET, local_addr, "Udp::run_udp_server breaking: (ran dry, should not happen in production!)");
                break;
            }
        }
    }

    async fn process_request(request: RawRequest, tracker: Arc<Tracker>, socket: Arc<BoundSocket>) {
        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, request = %request.from, "Udp::process_request (receiving)");
        Self::process_valid_request(tracker, socket, request).await;
    }

    async fn process_valid_request(tracker: Arc<Tracker>, socket: Arc<BoundSocket>, udp_request: RawRequest) {
        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, "Udp::process_valid_request. Making Response to {udp_request:?}");
        let from = udp_request.from;
        let response = handlers::handle_packet(udp_request, &tracker.clone(), socket.address()).await;
        Self::send_response(&socket.clone(), from, response).await;
    }

    async fn send_response(bound_socket: &Arc<BoundSocket>, to: SocketAddr, response: Response) {
        let response_type = match &response {
            Response::Connect(_) => "Connect".to_string(),
            Response::AnnounceIpv4(_) => "AnnounceIpv4".to_string(),
            Response::AnnounceIpv6(_) => "AnnounceIpv6".to_string(),
            Response::Scrape(_) => "Scrape".to_string(),
            Response::Error(e) => format!("Error: {e:?}"),
        };

        tracing::debug!(target: UDP_TRACKER_LOG_TARGET, target = ?to, response_type,  "Udp::send_response (sending)");

        let buffer = vec![0u8; MAX_PACKET_SIZE];
        let mut cursor = Cursor::new(buffer);

        match response.write_bytes(&mut cursor) {
            Ok(()) => {
                #[allow(clippy::cast_possible_truncation)]
                let position = cursor.position() as usize;
                let inner = cursor.get_ref();

                tracing::debug!(target: UDP_TRACKER_LOG_TARGET, ?to, bytes_count = &inner[..position].len(), "Udp::send_response (sending...)" );
                tracing::trace!(target: UDP_TRACKER_LOG_TARGET, ?to, bytes_count = &inner[..position].len(), payload = ?&inner[..position], "Udp::send_response (sending...)");

                Self::send_packet(bound_socket, &to, &inner[..position]).await;

                tracing::trace!(target:UDP_TRACKER_LOG_TARGET, ?to, bytes_count = &inner[..position].len(), "Udp::send_response (sent)");
            }
            Err(e) => {
                tracing::error!(target: UDP_TRACKER_LOG_TARGET, ?to, response_type, err = %e, "Udp::send_response (error)");
            }
        }
    }

    async fn send_packet(bound_socket: &Arc<BoundSocket>, remote_addr: &SocketAddr, payload: &[u8]) {
        tracing::trace!(target: UDP_TRACKER_LOG_TARGET, to = %remote_addr, ?payload, "Udp::send_response (sending)");

        // doesn't matter if it reaches or not
        drop(bound_socket.send_to(payload, remote_addr).await);
    }
}