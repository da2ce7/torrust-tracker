use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use log::{debug, error, info, warn};
use tokio::task::JoinHandle;

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

fn get_tracker_settings(config: &HttpTrackerConfig) -> Result<Option<HttpServerSettings>, HttpServerSettingsError> {
    let name = config.name.unwrap_or_default();
    let is_enabled = config.enabled.unwrap_or_default();
    let bind_addr = config.bind_address.unwrap_or_default();
    let ssl_enabled = config.ssl_enabled.unwrap_or_default();
    let ssl_cert_path = config.ssl_cert_path.unwrap_or_default();
    let ssl_key_path = config.ssl_key_path.unwrap_or_default();

    if name.is_empty() {
        warn!("Warning: Unamed HTTP server in configuration, please add a Name!");
        if is_enabled {
            error!("Error: Enabled HTTP server is UNAMED!");
        } else {
            warn!("Warning: Disabled HTTP server is UNAMED!");
        }
    };

    if bind_addr.is_empty() {
        if is_enabled {
            error!("Error: Enabled HTTP server: \"{name}\" must have binding address!");
        } else {
            warn!("Warning: Disabled HTTP server: \"{name}\" should have binding address!");
        }
    } else {
        match bind_addr.parse::<SocketAddr>() {
            Ok(port) => {
                if is_enabled {
                    info!("Info: Enabled HTTP server: \"{name}\" binds to: \"{bind_addr:?}\".");
                } else {
                    info!("Info: Disabled HTTP server: \"{name}\" would bind to: \"{bind_addr:?}\".");
                }
            }
            Err(e) => {
                if is_enabled {
                    error!("Error: Enabled HTTP server: \"{name}\" has a invalid binding address!");
                } else {
                    warn!("Warning: Disabled HTTP server: \"{name}\" has a invalid binding address!");
                }
            }
        }
    };

    if ssl_enabled {
        if ssl_cert_path.is_empty() {
            if is_enabled {
                error!("Error: Enabled HTTP TLS server: \"{name}\" must have a certificate path!");
            } else {
                warn!("Warning: Disabled HTTP TLS server: \"{name}\" should have a certificate path!");
            }
        }

        let cert_path = match Path::new(&ssl_cert_path).canonicalize() {
            Ok(path) => {
                debug!(
                    "Info: Enabled HTTP TLS server: \"{name}\" will use path: \"{}\" as certificate.",
                    path.display()
                );
            }
            Err(_) => {
                if is_enabled {
                    error!(
                        "Error: Enabled HTTP TLS server: \"{name}\" has an invalid certificate path: \"{}\"!",
                        ssl_cert_path
                    );
                } else {
                    warn!(
                        "Warning: Disabled HTTP TLS server: \"{name}\"has an invalid certificate path: \"{}\"!",
                        ssl_cert_path
                    );
                }
            }
        };

        if Path::from(cert_path).exists() {
            if cert_path.is_file() {
                if is_enabled {
                    info!(
                        "Info: Enabled HTTP TLS server: \"{name}\" will use file at: \"{}\" as certificate.",
                        cert_path.display()
                    );
                } else {
                    info!(
                        "Info: Disabled HTTP TLS server: \"{name}\" would use file at: \"{}\" as certificate.",
                        cert_path.display()
                    );
                }
            }
            if is_enabled {
                error!("Error: Enabled HTTP TLS server: \"{name}\" must have a certificate path!");
            } else {
                error!("Warning: Disabled HTTP TLS server: \"{name}\" should have a certificate path!");
            }
        }

        if ssl_key_path.is_empty() {
            if is_enabled {
                error!("Error: Enabled HTTP TLS server: \"{name}\" must have a key path!");
            } else {
                error!("Warning: Disabled HTTP TLS server: \"{name}\" should have a key path!");
            }
        }
    }

    if !is_enabled {
        info!("Will not load HTTP server: \"{name}\", disabled in config.");
        return Ok(None);
    }

    Err(HttpServerSettingsError::NoName)
}
