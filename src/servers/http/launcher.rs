use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
use derive_more::derive::{Constructor, Display};
use futures::TryFutureExt;
use tokio::task::JoinSet;
use tracing::instrument;

use super::error::Error;
use crate::core::Tracker;
use crate::servers::custom_axum_server::{self, TimeoutAcceptor};
use crate::servers::http::server::Running;
use crate::servers::http::v1::routes::router;

#[derive(Debug, Display, Clone)]
pub enum ProtocolTls {
    Without(String),
    With(String),
}

impl ProtocolTls {
    fn new(tls: bool) -> Self {
        if tls {
            Self::With("https".to_string())
        } else {
            Self::Without("http".to_string())
        }
    }
}

#[derive(Constructor, Clone)]
pub struct Launcher {
    tracker: Arc<Tracker>,
    tls: Option<RustlsConfig>,
    bind_to: SocketAddr,
}

impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tls = if self.tls.is_some() { "enabled" } else { "disabled" };

        f.debug_struct("Launcher")
            .field("bind_to", &self.bind_to)
            .field("tls", &tls)
            .field("tracker", &"..")
            .finish()
    }
}

impl std::fmt::Display for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tls = if self.tls.is_some() { "enabled" } else { "disabled" };

        f.write_fmt(format_args!("Launcher with tls {tls}, binding to: {}", self.bind_to))
    }
}

impl Launcher {
    #[instrument(skip(self))]
    pub(super) fn start<S>(&self) -> Result<Running<S>, <S as torrust_tracker_services::service::Service>::Error>
    where
        S: torrust_tracker_services::service::Service<Error = Error>,
    {
        let mut tasks = JoinSet::new();
        let handle = Handle::new();
        let socket = std::net::TcpListener::bind(self.bind_to).map_err(Error::FailedToBindToSocket)?;
        let local_addr = socket.local_addr().map_err(Error::FailedToObtainLocalAddress)?;
        let app = router(self.tracker.clone(), local_addr);

        drop(match &self.tls {
            Some(tls) => tasks.spawn(
                custom_axum_server::from_tcp_rustls_with_timeouts(socket, tls.clone())
                    .handle(handle.clone())
                    // The TimeoutAcceptor is commented because TSL does not work with it.
                    // See: https://github.com/torrust/torrust-index/issues/204#issuecomment-2115529214
                    //.acceptor(TimeoutAcceptor)
                    .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                    .map_err(Error::FailedToStart),
            ),
            None => tasks.spawn(
                custom_axum_server::from_tcp_with_timeouts(socket)
                    .handle(handle.clone())
                    .acceptor(TimeoutAcceptor)
                    .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
                    .map_err(Error::FailedToStart),
            ),
        });

        Ok(Running::new(
            tasks,
            handle,
            Some(Duration::from_secs(90)),
            local_addr,
            ProtocolTls::new(self.tls.is_some()),
        ))
    }
}
