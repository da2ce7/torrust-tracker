use thiserror::Error;
use warp::reject::Reject;

use crate::settings::{CommonSettings, DatabaseSettings, GlobalSettings, ServiceSetting, TrackerSettings};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("internal server error")]
    InternalServerError,

    #[error("info_hash is either missing or invalid")]
    InvalidInfoHash,

    #[error("peer_id is either missing or invalid")]
    InvalidPeerId,

    #[error("could not find remote address")]
    AddressNotFound,

    #[error("torrent has no peers")]
    NoPeersFound,

    #[error("torrent not on whitelist")]
    TorrentNotWhitelisted,

    #[error("peer not authenticated")]
    PeerNotAuthenticated,

    #[error("invalid authentication key")]
    PeerKeyNotValid,

    #[error("exceeded info_hash limit")]
    ExceededInfoHashLimit,

    #[error("bad request")]
    BadRequest,

    #[error("connection cookie is not valid")]
    InvalidConnectionCookie,

    #[error("bad server configuration")]
    ServiceSettingsError { message: String, source: ServiceSettingsError },
}

impl Reject for ServerError {}

#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("Bad Namespace: \".namespace\" {message}")]
    NamespaceError { message: String },

    // Todo: Expand this for Semantic Versioning 2.0.0
    #[error("Bad Version: \".namespace\" {message}")]
    VersionError { message: String },

    #[error("Tracker Settings Error: \"settings.{message}")]
    TrackerSettingsError { message: String, source: TrackerSettingsError },

    #[error("Global Settings Error: \"settings.global.{message}")]
    GlobalSettingsError { message: String, source: GlobalSettingsError },

    #[error("Common Settings Error: \"settings.common.{message}")]
    CommonSettingsError { message: String, source: CommonSettingsError },

    #[error("Database Settings Error: \"settings.database.{message}")]
    DatabaseSettingsError { message: String, source: DatabaseSettingsError },

    #[error("Service Settings Error: \"settings.service.{id}.{message}")]
    ServiceSettingsError {
        id: String,
        message: String,
        source: ServiceSettingsError,
    },
}

#[derive(Error, Debug, Clone)]
pub enum TrackerSettingsError {
    #[error("\"{field}\": Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: TrackerSettings },
}

#[derive(Error, Debug, Clone)]
pub enum GlobalSettingsError {
    #[error("\"{field}\": Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: GlobalSettings },
}

#[derive(Error, Debug, Clone)]
pub enum CommonSettingsError {
    #[error("\"{field}\": Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: CommonSettings },

    #[error("\"{field}\": Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: CommonSettings },
}

#[derive(Error, Debug, Clone)]
pub enum DatabaseSettingsError {
    #[error("\"{field}\": Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: DatabaseSettings },

    #[error("\"{field}\": Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: DatabaseSettings },
}

#[derive(Error, Debug, Clone)]
pub enum ServiceSettingsError {
    #[error("{field}\": Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: ServiceSetting },

    #[error("{field}\": Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: ServiceSetting },

    #[error("Service {id} is without Display Name!")]
    UnnamedService { id: String, data: ServiceSetting },

    #[error("Binding Address: \"{input}\" has Bad Syntax: {source}")]
    BindingAddressBadSyntax {
        id: String,
        input: String,
        source: std::net::AddrParseError,
        data: ServiceSetting,
    },

    #[error("Bad TLS Configuration: {source}")]
    BadHttpTlsConfig {
        id: String,
        source: TlsConfigError,
        data: ServiceSetting,
    },
}

#[derive(Error, Debug, Clone)]
pub enum TlsConfigError {
    #[error("Unable to find TLS Certificate File: {source}")]
    BadCertificateFilePath { source: FilePathError },

    #[error("Unable to find TLS Key File: {source}")]
    BadKeyFilePath { source: FilePathError },
}

#[derive(Error, Debug, Clone)]
pub enum FilePathError {
    #[error("File Path Supplied is Empty!")]
    FilePathIsEmpty,

    #[error("File Path failed to Canonicalize: {input}, {message}")]
    FilePathIsUnresolvable { input: String, message: String },

    #[error("File Path destination dose not exist: {input}")]
    FilePathDoseNotExist { input: String },

    #[error("File Path destination is not a file: {input}")]
    FilePathIsNotAFile { input: String },
}
