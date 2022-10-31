use thiserror::Error;
use warp::reject::Reject;

use crate::settings::{CommonSettings, DatabaseSettings, GlobalSettings, ServiceSetting, TlsSettings, TrackerSettings};

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
    #[error("Bad Namespace: \".{field}\" {message}")]
    NamespaceError { message: String, field: String },

    // Todo: Expand this for Semantic Versioning 2.0.0
    #[error("Bad Version: \".{field}\" {message}")]
    VersionError { message: String, field: String },

    #[error("Tracker Settings Error: \".tracker.{field}\": {message}")]
    TrackerSettingsError {
        message: String,
        field: String,
        source: TrackerSettingsError,
    },

    #[error("Global Settings Error: \".tracker.global.{field}\": {message}")]
    GlobalSettingsError {
        message: String,
        field: String,
        source: GlobalSettingsError,
    },

    #[error("Common Settings Error: \".tracker.common.{field}\": {message}")]
    CommonSettingsError {
        message: String,
        field: String,
        source: CommonSettingsError,
    },

    #[error("Database Settings Error: \".tracker.database.{field}\": {message}")]
    DatabaseSettingsError {
        message: String,
        field: String,
        source: DatabaseSettingsError,
    },

    #[error("Service Settings Error: \".tracker.service.{id}.{field}\":{message}")]
    ServiceSettingsError {
        message: String,
        field: String,
        id: String,
        source: ServiceSettingsError,
    },
}

#[derive(Error, Debug, Clone)]
pub enum TrackerSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: TrackerSettings },
}

impl TrackerSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Debug, Clone)]
pub enum GlobalSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: GlobalSettings },
}

impl GlobalSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Debug, Clone)]
pub enum CommonSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: CommonSettings },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: CommonSettings },
}

impl CommonSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data } => field,
            Self::EmptyRequiredField { field, data } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Debug, Clone)]
pub enum DatabaseSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: DatabaseSettings },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: DatabaseSettings },
}

impl DatabaseSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data } => field,
            Self::EmptyRequiredField { field, data } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Debug, Clone)]
pub enum ServiceSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: ServiceSetting },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: ServiceSetting },

    #[error("Api Services Requires at least one Access Token!")]
    ApiRequiresAccessToken { field: String, data: ServiceSetting },

    #[error("TLS Services Requires TLS Settings!")]
    TlsRequiresTlsConfig { field: String, data: ServiceSetting },

    #[error("Bad TLS Configuration: {source}")]
    TlsSettingsError {
        field: String,
        source: TlsSettingsError,
        data: ServiceSetting,
    },
}

impl ServiceSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data } => field,
            Self::EmptyRequiredField { field, data } => field,
            Self::ApiRequiresAccessToken { field, data } => field,
            Self::TlsRequiresTlsConfig { field, data } => field,
            Self::TlsSettingsError { field, source, data } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Debug, Clone)]
pub enum TlsSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: TlsSettings },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: TlsSettings },

    #[error("Unable to find TLS Certificate File: {source}")]
    BadCertificateFilePath { field: String, source: FilePathError },

    #[error("Unable to find TLS Key File: {source}")]
    BadKeyFilePath { field: String, source: FilePathError },
}

impl TlsSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data } => field,
            Self::EmptyRequiredField { field, data } => field,
            Self::BadCertificateFilePath { field, source } => field,
            Self::BadKeyFilePath { field, source } => field,
        }
        .to_owned()
    }
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
