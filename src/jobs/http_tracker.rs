use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{error, info, warn};
use tokio::task::JoinHandle;

use crate::errors::{FilePathError, ServerError, ServiceConfigError, TlsConfigError};
use crate::settings::old_settings::HttpTrackerConfig;
use crate::tracker::tracker::TorrentTracker;
use crate::{HttpServer, HttpServerSettings, TlsSettings};

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

    let name = match get_name(config.display_name.as_ref().unwrap_or(empty_string)) {
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

fn handel_server_config_error(error: &ServiceConfigError, server_type: &String, server_name: &Option<String>) -> ServerError {
    let unnamed = &"UNNAMED".to_string();

    match error {
        ServiceConfigError::UnnamedServer => {
            let message = format!("\"{}\", {}", server_type, ServiceConfigError::UnnamedServer);
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }
        ServiceConfigError::BindingAddressIsEmpty => {
            let message = format!(
                "\"{}\", \"{}\", {}",
                server_type,
                server_name.as_ref().unwrap_or(unnamed),
                ServiceConfigError::BindingAddressIsEmpty
            );
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }
        ServiceConfigError::BindingAddressBadSyntax { input, source } => {
            let message = format!(
                "Error: \"{}\", \"{}\", {}.",
                server_type,
                server_name.as_ref().unwrap_or(unnamed),
                ServiceConfigError::BindingAddressBadSyntax {
                    input: input.clone(),
                    source: source.clone()
                },
            );
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }

        ServiceConfigError::BadHttpTlsConfig { source } => {
            let message = format!(
                "Error: \"{}\", \"{}\", {}.",
                server_type,
                server_name.as_ref().unwrap_or(unnamed),
                ServiceConfigError::BadHttpTlsConfig { source: source.clone() },
            );
            ServerError::ConfigurationError {
                message,
                source: error.clone(),
            }
        }
    }
}

fn handel_http_tls_config_error(error: &TlsConfigError) -> ServiceConfigError {
    match error {
        TlsConfigError::BadCertificateFilePath { source } => ServiceConfigError::BadHttpTlsConfig {
            source: TlsConfigError::BadCertificateFilePath { source: source.clone() },
        },
        TlsConfigError::BadKeyFilePath { source } => ServiceConfigError::BadHttpTlsConfig {
            source: TlsConfigError::BadKeyFilePath { source: source.clone() },
        },
    }
}

fn get_name(name: &String) -> Result<String, ServiceConfigError> {
    if !name.is_empty() {
        Ok(name.clone())
    } else {
        Err(ServiceConfigError::UnnamedServer)
    }
}

fn get_socket(bind_addr: &String) -> Result<SocketAddr, ServiceConfigError> {
    if !bind_addr.is_empty() {
        match bind_addr.parse::<SocketAddr>() {
            Ok(socket) => Ok(socket),
            Err(source) => Err(ServiceConfigError::BindingAddressBadSyntax {
                input: bind_addr.to_string(),
                source,
            }),
        }
    } else {
        Err(ServiceConfigError::BindingAddressIsEmpty)
    }
}

fn get_tls_config(tls_cert_path: &String, tls_key_path: &String) -> Result<TlsSettings, TlsConfigError> {
    let cert_file_path = match get_path(tls_cert_path) {
        Ok(path) => path,
        Err(source) => return Err(TlsConfigError::BadCertificateFilePath { source }),
    };

    let key_file_path = match get_path(tls_key_path) {
        Ok(path) => path,
        Err(source) => return Err(TlsConfigError::BadKeyFilePath { source }),
    };

    Ok(TlsSettings {
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
            Err(error) => Err(FilePathError::FilePathIsUnresolvable {
                input: path.clone(),
                message: error.to_string(),
            }),
        }
    } else {
        Err(FilePathError::FilePathIsEmpty)
    }
}
