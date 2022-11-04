use std::io::Cursor;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use aquatic_udp_protocol::Response;
use log::{debug, info};
use tokio::net::UdpSocket;

use crate::errors::ServiceSettingsError;
use crate::settings::{Service, ServiceProtocol};
use crate::tracker::tracker::TorrentTracker;
use crate::udp::{handle_packet, MAX_PACKET_SIZE};
use crate::{check_field_is_not_empty, check_field_is_not_none};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct UdpServiceSettings {
    pub id: String,
    pub enabled: bool,
    pub display_name: String,
    pub socket: SocketAddr,
}

impl Default for UdpServiceSettings {
    fn default() -> Self {
        Self {
            id: "default_udp".to_string(),
            enabled: false,
            display_name: "UDP (default)".to_string(),
            socket: SocketAddr::from_str("0.0.0.0:6969").unwrap(),
        }
    }
}

impl TryFrom<(&String, &Service)> for UdpServiceSettings {
    type Error = ServiceSettingsError;

    fn try_from(value: (&String, &Service)) -> Result<Self, Self::Error> {
        check_field_is_not_none!(value.1 => ServiceSettingsError;
            enabled, service);

        if value.1.service.unwrap() != ServiceProtocol::Udp {
            return Err(ServiceSettingsError::WrongService {
                field: "service".to_string(),
                expected: ServiceProtocol::Udp,
                found: value.1.service.unwrap(),
                data: value.1.into(),
            });
        }

        check_field_is_not_empty!(value.1 => ServiceSettingsError;
                display_name: String);

        Ok(Self {
            id: value.0.to_owned(),
            enabled: value.1.enabled.unwrap(),
            display_name: value.1.display_name.to_owned().unwrap(),
            socket: value.1.get_socket()?,
        })
    }
}

pub struct UdpServer {
    socket: Arc<UdpSocket>,
    tracker: Arc<TorrentTracker>,
}

impl UdpServer {
    pub async fn new(tracker: Arc<TorrentTracker>, socket_addr: &SocketAddr) -> tokio::io::Result<UdpServer> {
        let socket = UdpSocket::bind(socket_addr).await?;

        Ok(UdpServer {
            socket: Arc::new(socket),
            tracker,
        })
    }

    pub async fn start(&self) {
        loop {
            let mut data = [0; MAX_PACKET_SIZE];
            let socket = self.socket.clone();
            let tracker = self.tracker.clone();

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Stopping UDP server: {}..", socket.local_addr().unwrap());
                    break;
                }
                Ok((valid_bytes, remote_addr)) = socket.recv_from(&mut data) => {
                    let payload = data[..valid_bytes].to_vec();

                    debug!("Received {} bytes from {}", payload.len(), remote_addr);
                    debug!("{:?}", payload);

                    let response = handle_packet(remote_addr, payload, tracker).await;
                    UdpServer::send_response(socket, remote_addr, response).await;
                }
            }
        }
    }

    async fn send_response(socket: Arc<UdpSocket>, remote_addr: SocketAddr, response: Response) {
        debug!("sending response to: {:?}", &remote_addr);

        let buffer = vec![0u8; MAX_PACKET_SIZE];
        let mut cursor = Cursor::new(buffer);

        match response.write(&mut cursor) {
            Ok(_) => {
                let position = cursor.position() as usize;
                let inner = cursor.get_ref();

                debug!("{:?}", &inner[..position]);
                UdpServer::send_packet(socket, &remote_addr, &inner[..position]).await;
            }
            Err(_) => {
                debug!("could not write response to bytes.");
            }
        }
    }

    async fn send_packet(socket: Arc<UdpSocket>, remote_addr: &SocketAddr, payload: &[u8]) {
        // doesn't matter if it reaches or not
        let _ = socket.send_to(payload, remote_addr).await;
    }
}
