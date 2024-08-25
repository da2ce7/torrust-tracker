use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;

use aquatic_udp_protocol::Response;
use tracing::{instrument, Level};

use super::bound_socket::BoundSocket;
use crate::core::Tracker;
use crate::servers::udp::{handlers, RawRequest};

pub struct Processor {
    socket: Arc<BoundSocket>,
    tracker: Arc<Tracker>,
}

impl Processor {
    pub fn new(socket: Arc<BoundSocket>, tracker: Arc<Tracker>) -> Self {
        Self { socket, tracker }
    }

    #[instrument(skip(self, request))]
    pub async fn process_request(self, request: RawRequest) {
        let from = request.from;
        let response = handlers::handle_packet(request, &self.tracker, self.socket.local_addr()).await;
        self.send_response(from, response).await;
    }

    #[instrument(skip(self, response))]
    async fn send_response(self, target: SocketAddr, response: Response) {
        let response_type = match &response {
            Response::Connect(_) => "Connect".to_string(),
            Response::AnnounceIpv4(_) => "AnnounceIpv4".to_string(),
            Response::AnnounceIpv6(_) => "AnnounceIpv6".to_string(),
            Response::Scrape(_) => "Scrape".to_string(),
            Response::Error(e) => format!("Error: {e:?}"),
        };

        let mut writer = Cursor::new(Vec::with_capacity(200));

        match response.write_bytes(&mut writer) {
            Ok(()) => {
                let total = writer.get_ref().len();
                let payload = writer.get_ref();

                let () = match self.send_packet(&target, payload).await {
                    Ok(sent) => {
                        if sent != total {
                            tracing::warn!(%total, %sent, ?payload, "Sent Incomplete {response_type} Response");
                        } else if tracing::event_enabled!(Level::TRACE) {
                            tracing::debug!(%total, %sent, ?payload, "Sent {response_type} Response");
                        } else {
                            tracing::debug!(%total, %sent, "Sent {response_type} Response");
                        }
                    }
                    Err(error) => tracing::warn!(%total, %error, ?payload, "failed to send"),
                };
            }
            Err(e) => {
                tracing::error!(%e, "error");
            }
        }
    }

    #[instrument(skip(self))]
    async fn send_packet(&self, target: &SocketAddr, payload: &[u8]) -> std::io::Result<usize> {
        // doesn't matter if it reaches or not
        self.socket.send_to(payload, target).await
    }
}
