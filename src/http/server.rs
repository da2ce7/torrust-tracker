use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::errors::settings::ServiceSettingsError;
use crate::http::routes;
use crate::settings::{Service, ServiceProtocol};
use crate::tracker::core::TorrentTracker;
use crate::{check_field_is_not_empty, check_field_is_not_none};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct HttpServiceSettings {
    pub id: String,
    pub enabled: bool,
    pub display_name: String,
    pub socket: SocketAddr,
}

impl Default for HttpServiceSettings {
    fn default() -> Self {
        Self {
            id: "default_http".to_string(),
            enabled: false,
            display_name: "HTTP (default)".to_string(),
            socket: SocketAddr::from_str("0.0.0.0:6969").unwrap(),
        }
    }
}

impl TryFrom<(&String, &Service)> for HttpServiceSettings {
    type Error = ServiceSettingsError;

    fn try_from(value: (&String, &Service)) -> Result<Self, Self::Error> {
        check_field_is_not_none!(value.1 => ServiceSettingsError;
            enabled, service);

        if value.1.service.unwrap() != ServiceProtocol::Http {
            return Err(ServiceSettingsError::WrongService {
                field: "service".to_string(),
                expected: ServiceProtocol::Http,
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TlsServiceSettings {
    pub id: String,
    pub enabled: bool,
    pub display_name: String,
    pub socket: SocketAddr,
    pub certificate_file_path: PathBuf,
    pub key_file_path: PathBuf,
}

impl Default for TlsServiceSettings {
    fn default() -> Self {
        Self {
            id: "default_http".to_string(),
            enabled: false,
            display_name: "HTTP (default)".to_string(),
            socket: SocketAddr::from_str("0.0.0.0:6969").unwrap(),
            certificate_file_path: Default::default(),
            key_file_path: Default::default(),
        }
    }
}

impl TryFrom<(&String, &Service)> for TlsServiceSettings {
    type Error = ServiceSettingsError;

    fn try_from(value: (&String, &Service)) -> Result<Self, Self::Error> {
        check_field_is_not_none!(value.1 => ServiceSettingsError;
            enabled, service, tls);

        if value.1.service.unwrap() != ServiceProtocol::Tls {
            return Err(ServiceSettingsError::WrongService {
                field: "service".to_string(),
                expected: ServiceProtocol::Tls,
                found: value.1.service.unwrap(),
                data: value.1.into(),
            });
        }

        check_field_is_not_empty!(value.1 => ServiceSettingsError;
                display_name: String);

        let tls = value.1.tls.to_owned().unwrap();

        Ok(Self {
            id: value.0.to_owned(),
            enabled: value.1.enabled.unwrap(),
            display_name: value.1.display_name.to_owned().unwrap(),
            socket: value.1.get_socket()?,

            certificate_file_path: tls
                .get_certificate_file_path()
                .map_err(|err| ServiceSettingsError::TlsSettingsError {
                    field: value.0.to_owned(),
                    source: err,
                    data: value.1.into(),
                })?,

            key_file_path: tls
                .get_key_file_path()
                .map_err(|err| ServiceSettingsError::TlsSettingsError {
                    field: value.0.to_owned(),
                    source: err,
                    data: value.1.into(),
                })?,
        })
    }
}

/// Server that listens on HTTP, needs a TorrentTracker
#[derive(Clone)]
pub struct HttpServer {
    tracker: Arc<TorrentTracker>,
}

impl HttpServer {
    pub fn new(tracker: Arc<TorrentTracker>) -> HttpServer {
        HttpServer { tracker }
    }

    /// Start the HttpServer
    pub fn start(&self, socket_addr: SocketAddr) -> impl warp::Future<Output = ()> {
        let (_addr, server) = warp::serve(routes(self.tracker.clone())).bind_with_graceful_shutdown(socket_addr, async move {
            tokio::signal::ctrl_c().await.expect("Failed to listen to shutdown signal.");
        });

        server
    }

    /// Start the HttpServer in TLS mode
    pub fn start_tls(
        &self,
        socket_addr: SocketAddr,
        ssl_cert_path: PathBuf,
        ssl_key_path: PathBuf,
    ) -> impl warp::Future<Output = ()> {
        let (_addr, server) = warp::serve(routes(self.tracker.clone()))
            .tls()
            .cert_path(ssl_cert_path)
            .key_path(ssl_key_path)
            .bind_with_graceful_shutdown(socket_addr, async move {
                tokio::signal::ctrl_c().await.expect("Failed to listen to shutdown signal.");
            });

        server
    }
}
