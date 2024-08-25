use std::fmt::Debug;
use std::net::SocketAddr;
use std::ops::Deref;

use url::Url;

use crate::servers::udp::UDP_TRACKER_LOG_TARGET;

/// Wrapper for Tokio [`UdpSocket`][`tokio::net::UdpSocket`] that is bound to a particular socket.
pub struct BoundSocket {
    socket: tokio::net::UdpSocket,
}

impl BoundSocket {
    /// # Errors
    ///
    /// Will return an error if the socket can't be bound the the provided address.
    pub fn new(addr: SocketAddr) -> std::io::Result<Self> {
        let bind_addr = format!("udp://{addr}");
        tracing::debug!(target: UDP_TRACKER_LOG_TARGET, bind_addr, "UdpSocket::new (binding)");

        let socket = std::net::UdpSocket::bind(addr)?;

        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = format!("udp://{}", socket.local_addr()?);
        tracing::debug!(target: UDP_TRACKER_LOG_TARGET, local_addr, "UdpSocket::new (bound)");

        Ok(Self { socket })
    }

    /// # Panics
    ///
    /// Will panic if the socket can't get the address it was bound to.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr().expect("it should get local address")
    }

    /// # Panics
    ///
    /// Will panic if the address the socket was bound to is not a valid address
    /// to be used in a URL.
    #[must_use]
    pub fn url(&self) -> Url {
        Url::parse(&format!("udp://{}", self.local_addr())).expect("UDP socket address should be valid")
    }
}

impl Deref for BoundSocket {
    type Target = tokio::net::UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.socket
    }
}

impl Debug for BoundSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let local_addr = match self.socket.local_addr() {
            Ok(socket) => format!("Receiving From: {socket}"),
            Err(err) => format!("Socket Broken: {err}"),
        };

        f.debug_struct("UdpSocket").field("addr", &local_addr).finish_non_exhaustive()
    }
}
