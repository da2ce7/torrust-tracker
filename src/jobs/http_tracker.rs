use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{debug, error, info, warn};
use tokio::task::JoinHandle;

use crate::errors::{FilePathError, HttpTlsConfigError, ServerConfigError, ServerError};
use crate::settings::HttpTrackerConfig;
use crate::tracker::tracker::TorrentTracker;
use crate::{HttpServer, HttpServerSettings, HttpServerSettingsError};

pub fn start_job(config: &HttpTrackerConfig, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let bind_addr = config.bind_address.unwrap().parse::<SocketAddr>().unwrap();
    let ssl_enabled = config.ssl_enabled;
    let ssl_cert_path = config.ssl_cert_path.clone();
    let ssl_key_path = config.ssl_key_path.clone();

    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        if !ssl_enabled.unwrap() {
            info!("Starting HTTP server on: {}", bind_addr);
            http_tracker.start(bind_addr).await;
        } else if ssl_enabled.unwrap() && ssl_cert_path.is_some() && ssl_key_path.is_some() {
            info!("Starting HTTPS server on: {} (TLS)", bind_addr);
            http_tracker
                .start_tls(bind_addr, ssl_cert_path.unwrap(), ssl_key_path.unwrap())
                .await;
        } else {
            warn!("Could not start HTTP tracker on: {}, missing SSL Cert or Key!", bind_addr);
        }
    })
}

fn get_tracker_settings(config: &HttpTrackerConfig) -> Result<Option<HttpServerSettings>, ServerError> {
    let name = config.name.unwrap_or_default();
    let is_enabled = config.enabled.unwrap_or_default();
    let bind_addr = config.bind_address.unwrap_or_default();
    let ssl_enabled = config.ssl_enabled.unwrap_or_default();
    let ssl_cert_path = config.ssl_cert_path.unwrap_or_default();
    let ssl_key_path = config.ssl_key_path.unwrap_or_default();

    const HTTP_SERVER: string = "HTTP Server";
    const HTTP_SERVER_TLS: string = "HTTP TLS Server";

    check_name(HTTP_SERVER, name, is_enabled);

    check_binding(HTTP_SERVER, bind_addr, is_enabled);

    if ssl_enabled {
        check_path(HTTP_SERVER_TLS, ssl_cert_path, is_enabled);

        if cert_path.is_some() {}
    }

    if !is_enabled {
        info!("Will not load HTTP server: \"{server_name}\", disabled in config.");
        return Ok(None);
    }

    Err(HttpServerSettingsError::NoName)
}

fn handel_server_config_error(
    error: ServerConfigError,
    server_type: String,
    server_name: String,
    is_enabled: bool,
) -> Result<(), ServerError> {
    if is_enabled {
        match error {
            ServerConfigError::UnnamedServer => {
                error!(
                    "Warning: \"{}\" has error: {}",
                    server_type,
                    ServerConfigError::UnnamedServer.to_string()
                );
                Err(ServerError::ConfigurationError { source: error })
            }
            ServerConfigError::BindingAddressIsEmpty => {
                error!(
                    "Warning: \"{}\", \"{}\" has error: {}",
                    server_type,
                    server_name,
                    ServerConfigError::UnnamedServer.to_string()
                );
                Err(ServerError::ConfigurationError { source: error })
            }
            ServerConfigError::BindingAddressBadSyntax { input, source } => {
                warn!("");
                Ok(())
            }

            // Bad Tls Config Should be handled elsewhere.
            ServerConfigError::BadTlsConfig { source } => Err(ServerError::InternalServerError),
        }
    } else {
        Ok(())
    }
}

fn handel_tls_config_error(error: HttpTlsConfigError, is_enabled: bool) -> Result<(), ServerConfigError> {
    if is_enabled {
        match error {
            HttpTlsConfigError::BadCertificateFilePath { source } => {
                error!("{}", ServerConfigError::UnnamedServer.to_string());
                Err(ServerConfigError::BadTlsConfig { source: error })
            }
            HttpTlsConfigError::BadKeyFilePath { source } => {
                warn!("{}", ServerConfigError::BindingAddressIsEmpty.to_string());
                Err(ServerConfigError::BadTlsConfig { source: error })
            }
        }
    } else {
        Ok(())
    }
}

fn handel_file_path_error(error: FilePathError, is_enabled: bool) -> Result<(), ()> {
    if is_enabled {
        match error {
            FilePathError::FilePathIsEmpty => {
                error!("{}", FilePathError::FilePathIsEmpty.to_string());
                Err(())
            }
            FilePathError::FilePathIsUnresolvable { input, message } => {
                warn!("{}", FilePathError::FilePathIsEmpty.to_string());
                Err(())
            }
            FilePathError::FilePathDoseNotExist { input } => {
                warn!("{}", FilePathError::FilePathIsEmpty.to_string());
                Err(())
            }

            FilePathError::FilePathIsNotAFile { input } => {
                warn!("{}", FilePathError::FilePathIsEmpty.to_string());
                Err(())
            }
        }
    } else {
        Ok(())
    }
}

fn check_name(server_type: String, name: String, is_enabled: bool) -> Result<(), ServerConfigError> {
    if name.is_empty() {
        Err(ServerConfigError::UnnamedServer)
    } else {
        Ok(())
    }
}

fn check_binding(
    server_type: String,
    server_name: String,
    bind_addr: String,
    is_enabled: bool,
) -> Result<SocketAddr, ServerConfigError> {
    if bind_addr.is_empty() {
        Err(ServerConfigError::BindingAddressIsEmpty)
    } else {
        match bind_addr.parse::<SocketAddr>() {
            Ok(socket) => Ok(socket),
            Err(e) => Err(ServerConfigError::BindingAddressBadSyntax {
                input: bind_addr,
                error: e,
            }),
        }
    }
}

fn check_path(
    server_type: String,
    server_name: String,
    path_type: String,
    path: String,
    is_enabled: bool,
) -> Result<PathBuf, ServerConfigError> {
    if path.is_empty() {
        Err(ServerConfigError::FilePathIsEmpty)
    } else {
        match Path::new(&path).canonicalize() {
            Ok(path_resolved) => {
                if path_resolved.exists() {
                    if path_resolved.is_file() {
                        Ok(path_resolved)
                    } else {
                        Err(ServerConfigError::FilePathIsNotAFile {
                            input: path_resolved.display().to_string(),
                        })
                    }
                } else {
                    Err(ServerConfigError::FilePathDoseNotExist {
                        input: path_resolved.display().to_string(),
                    })
                }
            }
            Err(e) => Err(ServerConfigError::FilePathIsUnresolvable {
                input: path,
                error: e.to_string(),
            }),
        }
    }
}
