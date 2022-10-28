use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{error, info, warn};
use tokio::task::JoinHandle;

use crate::errors::{FilePathError, HttpTlsConfigError, ServerConfigError, ServerError};
use crate::settings::HttpTrackerConfig;
use crate::tracker::tracker::TorrentTracker;
use crate::{HttpServer, HttpServerSettings, HttpServerTlsSettings};

pub fn start_job(config: &HttpTrackerConfig, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    let settings = get_tracker_settings(config).unwrap().unwrap();

    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        match settings.tls {
            Some(tls) => {
                info!("Starting HTTP Server \"{}\" on: {} (TLS)", settings.name, settings.socket);
                http_tracker
                    .start_tls(settings.socket, tls.cert_file_path, tls.key_file_path)
                    .await;
            }
            None => {
                info!("Starting HTTP Server \"{}\" on: {}", settings.name, settings.socket);
                http_tracker.start(settings.socket).await;
            }
        }
    })
}

fn get_tracker_settings(config: &HttpTrackerConfig) -> Result<Option<HttpServerSettings>, ServerError> {
    let empty_string = &"".to_string();
    let is_enabled = config.enabled.unwrap_or_default();

    let http_server: String = "HTTP Server".to_string();

    let name = match get_name(config.name.as_ref().unwrap_or(empty_string)) {
        Ok(name) => {
            info!("Info: Loading Config for HTTP Server: \"{name}\".");
            Some(name)
        }
        Err(error) => {
            let server_error = handel_server_config_error(&error, &http_server, &None);

            if !is_enabled {
                warn!("Warning: {}.", server_error.to_string());
                None
            } else {
                error!("Error: {}!", server_error.to_string());
                return Err(server_error);
            }
        }
    };

    // Not going to continue without a name.
    if name.is_none() {
        return Ok(None);
    };

    let socket = match get_socket(config.bind_address.as_ref().unwrap_or(empty_string)) {
        Ok(socket) => {
            info!("Info: HTTP Server \"{}\" uses socket: \"{socket}\".", name.clone().unwrap());
            Some(socket)
        }
        Err(error) => {
            let server_error = handel_server_config_error(&error, &http_server, &name);

            if !is_enabled {
                warn!("Warning: {}.", server_error.to_string());
                None
            } else {
                error!("Error: {}!", server_error.to_string());
                return Err(server_error);
            }
        }
    };

    let tls_config = if config.ssl_enabled.unwrap_or_default() {
        match get_tls_config(
            config.ssl_cert_path.as_ref().unwrap_or(empty_string),
            config.ssl_key_path.as_ref().unwrap_or(empty_string),
        ) {
            Ok(tls_config) => {
                info!(
                    "Info: HTTP Server \"{}\" uses TLS Certificate: \"{}\".",
                    name.clone().unwrap(),
                    tls_config.cert_file_path.display().to_string(),
                );
                info!(
                    "Info: HTTP Server \"{}\" uses TLS Key: \"{}\".",
                    name.clone().unwrap(),
                    tls_config.cert_file_path.display().to_string(),
                );
                Some(tls_config)
            }
            Err(error) => {
                let server_error = handel_server_config_error(&handel_http_tls_config_error(&error), &http_server, &name);

                if !is_enabled {
                    warn!("Warning: {}.", server_error.to_string());
                    None
                } else {
                    error!("Error: {}!", server_error.to_string());
                    return Err(server_error);
                }
            }
        }
    } else {
        None
    };

    if is_enabled {
        Ok(Some(HttpServerSettings {
            name: name.unwrap(),
            socket: socket.unwrap(),
            tls: tls_config,
        }))
    } else {
        Ok(None)
    }
}

fn handel_server_config_error(error: &ServerConfigError, server_type: &String, server_name: &Option<String>) -> ServerError {
    let unnamed = &"UNNAMED".to_string();

    match error {
        ServerConfigError::UnnamedServer => {
            let message = format!("\"{}\", {}", server_type, ServerConfigError::UnnamedServer);
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }
        ServerConfigError::BindingAddressIsEmpty => {
            let message = format!(
                "\"{}\", \"{}\", {}",
                server_type,
                server_name.as_ref().unwrap_or(unnamed),
                ServerConfigError::BindingAddressIsEmpty
            );
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }
        ServerConfigError::BindingAddressBadSyntax { input, source } => {
            let message = format!(
                "Error: \"{}\", \"{}\", {}.",
                server_type,
                server_name.as_ref().unwrap_or(unnamed),
                ServerConfigError::BindingAddressBadSyntax {
                    input: input.clone(),
                    source: source.clone()
                },
            );
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }

        ServerConfigError::BadHttpTlsConfig { source } => {
            let message = format!(
                "Error: \"{}\", \"{}\", {}.",
                server_type,
                server_name.as_ref().unwrap_or(unnamed),
                ServerConfigError::BadHttpTlsConfig { source: source.clone() },
            );
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }
    }
}

fn handel_http_tls_config_error(error: &HttpTlsConfigError) -> ServerConfigError {
    match error {
        HttpTlsConfigError::BadCertificateFilePath { source } => ServerConfigError::BadHttpTlsConfig {
            source: HttpTlsConfigError::BadCertificateFilePath { source: source.clone() },
        },
        HttpTlsConfigError::BadKeyFilePath { source } => ServerConfigError::BadHttpTlsConfig {
            source: HttpTlsConfigError::BadKeyFilePath { source: source.clone() },
        },
    }
}

fn get_name(name: &String) -> Result<String, ServerConfigError> {
    if !name.is_empty() {
        Ok(name.clone())
    } else {
        Err(ServerConfigError::UnnamedServer)
    }
}

fn get_socket(bind_addr: &String) -> Result<SocketAddr, ServerConfigError> {
    if !bind_addr.is_empty() {
        match bind_addr.parse::<SocketAddr>() {
            Ok(socket) => Ok(socket),
            Err(source) => Err(ServerConfigError::BindingAddressBadSyntax {
                input: bind_addr.to_string(),
                source,
            }),
        }
    } else {
        Err(ServerConfigError::BindingAddressIsEmpty)
    }
}

fn get_tls_config(tls_cert_path: &String, tls_key_path: &String) -> Result<HttpServerTlsSettings, HttpTlsConfigError> {
    let cert_file_path = match get_path(tls_cert_path) {
        Ok(path) => path,
        Err(source) => return Err(HttpTlsConfigError::BadCertificateFilePath { source }),
    };

    let key_file_path = match get_path(tls_key_path) {
        Ok(path) => path,
        Err(source) => return Err(HttpTlsConfigError::BadKeyFilePath { source }),
    };

    Ok(HttpServerTlsSettings {
        cert_file_path,
        key_file_path,
    })
}

fn get_path(path: &String) -> Result<PathBuf, FilePathError> {
    if !path.is_empty() {
        match Path::new(&path).canonicalize() {
            Ok(path) => {
                if path.exists() {
                    if path.is_file() {
                        Ok(path)
                    } else {
                        Err(FilePathError::FilePathIsNotAFile {
                            input: path.display().to_string(),
                        })
                    }
                } else {
                    Err(FilePathError::FilePathDoseNotExist {
                        input: path.display().to_string(),
                    })
                }
            }
            Err(e) => Err(FilePathError::FilePathDoseNotExist { input: path.clone() }),
        }
    } else {
        Err(FilePathError::FilePathIsEmpty)
    }
}
