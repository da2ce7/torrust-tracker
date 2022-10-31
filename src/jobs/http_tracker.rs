use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use log::{error, info, warn};
use tokio::task::JoinHandle;

use crate::errors::{FilePathError, ServiceSettingsError, TlsSettingsError};
use crate::tracker::tracker::TorrentTracker;
use crate::{HttpServer, HttpServiceSettings, TlsServiceSettings};

pub fn start_http_job(settings: &HttpServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        info!("Starting HTTP Server \"{}\" on: {}", settings.display_name, settings.socket);
        http_tracker.start(settings.socket).await;
    })
}

pub fn start_tls_job(settings: &TlsServiceSettings, tracker: Arc<TorrentTracker>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let http_tracker = HttpServer::new(tracker);

        info!(
            "Starting HTTP Server \"{}\" on: {} (TLS)",
            settings.display_name, settings.socket
        );
        http_tracker
            .start_tls(settings.socket, settings.certificate_file_path, settings.key_file_path)
            .await;
    })
}

fn get_socket(bind_addr: &String) -> Result<SocketAddr, ServiceSettingsError> {
    if !bind_addr.is_empty() {
        match bind_addr.parse::<SocketAddr>() {
            Ok(socket) => Ok(socket),
            Err(source) => Err(ServiceSettingsError::BindingAddressBadSyntax {
                input: bind_addr.to_string(),
                source,
            }),
        }
    } else {
        Err(ServiceSettingsError::BindingAddressIsEmpty)
    }
}

fn get_tls_config(tls_cert_path: &String, tls_key_path: &String) -> Result<TlsSettings, TlsSettingsError> {
    let cert_file_path = match get_path(tls_cert_path) {
        Ok(path) => path,
        Err(source) => return Err(TlsSettingsError::BadCertificateFilePath { source }),
    };

    let key_file_path = match get_path(tls_key_path) {
        Ok(path) => path,
        Err(source) => return Err(TlsSettingsError::BadKeyFilePath { source }),
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
