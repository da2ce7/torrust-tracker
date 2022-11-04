use std::io;
use std::path::PathBuf;

use thiserror::Error;
use warp::reject::Reject;

use crate::databases::database::DatabaseDrivers;
use crate::settings::{
    CommonSettings, DatabaseSettings, GlobalSettings, ServiceNoSecrets, ServiceProtocol, TlsSettings, TrackerSettings,
};

#[derive(Error, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
}

impl Reject for ServerError {}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsManagerError {
    #[error("Unable find existing configuration: \".{source}\"")]
    NoExistingConfigFile { source: FilePathError },
    #[error("File Exists!: \"{at}\"")]
    ExistingFile { at: PathBuf },
    #[error("Path is not a Directory!: \"{at}\"")]
    NotDirectory { at: PathBuf },

    #[error("Path is not a Directory at: \"{at}\" : {kind}: {message}!")]
    FailedToCreateConfigDirectory {
        at: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },

    #[error("Path is not a Directory at: \"{at}\" : {kind}: {message}!")]
    FailedToResolveDirectory {
        at: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },

    #[error("Unable to create new file at: \"{at}\" : {source}!")]
    FailedToCreateNewFile { at: PathBuf, source: FilePathError },
    #[error("Unable to open file: \"{at}\" : {kind}: {message}.")]
    FailedToOpenFile {
        at: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    #[error("Unable to read file: \"{from}\" : {kind}: {message}.")]
    FailedToReadFile {
        from: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    #[error("Unable to write file:  \"{to}\" : {kind}: {message}.")]
    FailedToWriteFile {
        to: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    #[error("Unable to import old settings from: \"{from}\" : \"{source}\"")]
    FailedToImportOldSettings { from: PathBuf, source: Box<SettingsError> },
    #[error("Unable to move successfully imported old settings from: {from} to: {to} \"{kind}: {message}\"")]
    FailedToMoveOldSettingsFile {
        from: PathBuf,
        to: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },

    #[error("Unable to read in json: \"{from}\" : {message}")]
    FailedToReadIn { from: PathBuf, message: String },
    #[error("Unable to read json: {message}")]
    FailedToReadBuffer { message: String },
    #[error("Unable to write out json: \"{to}\" : {message}")]
    FailedToWriteOut { to: PathBuf, message: String },
    #[error("Unable to write json: {message}")]
    FailedToWriteBuffer { message: String },
    #[error("Unable to parse in old settings from: \"{from}\" : {message}.")]
    FailedToParseInOld { from: PathBuf, message: String },
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

    #[error("Service Settings Error: \".tracker.service.{id}.{field}\": {message}")]
    ServiceSettingsError {
        message: String,
        field: String,
        id: String,
        source: ServiceSettingsError,
    },
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TrackerSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: TrackerSettings },
}

impl TrackerSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data: _ } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GlobalSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: GlobalSettings },

    #[error("Bad Socket String: \"{input}\", {message}")]
    ExternalIpBadSyntax {
        field: String,
        input: String,
        message: String,
        data: GlobalSettings,
    },
}

impl GlobalSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data: _ } => field,
            Self::ExternalIpBadSyntax {
                field,
                input: _,
                message: _,
                data: _,
            } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CommonSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: CommonSettings },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: CommonSettings },
}

impl CommonSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data: _ } => field,
            Self::EmptyRequiredField { field, data: _ } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DatabaseSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: DatabaseSettings },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: DatabaseSettings },

    #[error("Want {expected}, but have {actual}!")]
    WrongDriver {
        field: String,
        expected: DatabaseDrivers,
        actual: DatabaseDrivers,
        data: DatabaseSettings,
    },
}

impl DatabaseSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data: _ } => field,
            Self::EmptyRequiredField { field, data: _ } => field,
            Self::WrongDriver {
                field,
                expected: _,
                actual: _,
                data: _,
            } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ServiceSettingsError {
    #[error("Required Field is missing (null)!")]
    MissingRequiredField { field: String, data: ServiceNoSecrets },

    #[error("Required Field is empty (0 or \"\")!")]
    EmptyRequiredField { field: String, data: ServiceNoSecrets },

    #[error("Api Services Requires at least one Access Token!")]
    ApiRequiresAccessToken { field: String, data: ServiceNoSecrets },

    #[error("TLS Services Requires TLS Settings!")]
    TlsRequiresTlsConfig { field: String, data: ServiceNoSecrets },

    #[error("Bad TLS Configuration: {source}.")]
    TlsSettingsError {
        field: String,
        source: TlsSettingsError,
        data: ServiceNoSecrets,
    },

    #[error("Bad Socket String: \"{input}\".")]
    BindingAddressBadSyntax {
        field: String,
        input: String,
        message: String,
        data: ServiceNoSecrets,
    },
    #[error("Unexpected Service. Expected: {expected}, Found {found}.")]
    WrongService {
        field: String,
        expected: ServiceProtocol,
        found: ServiceProtocol,
        data: ServiceNoSecrets,
    },
}

impl ServiceSettingsError {
    pub fn get_field(&self) -> String {
        match self {
            Self::MissingRequiredField { field, data: _ } => field,
            Self::EmptyRequiredField { field, data: _ } => field,
            Self::ApiRequiresAccessToken { field, data: _ } => field,
            Self::TlsRequiresTlsConfig { field, data: _ } => field,
            Self::TlsSettingsError {
                field,
                source: _,
                data: _,
            } => field,
            Self::BindingAddressBadSyntax {
                field,
                input: _,
                message: _,
                data: _,
            } => field,

            Self::WrongService {
                field,
                expected: _,
                found: _,
                data: _,
            } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
            Self::MissingRequiredField { field, data: _ } => field,
            Self::EmptyRequiredField { field, data: _ } => field,
            Self::BadCertificateFilePath { field, source: _ } => field,
            Self::BadKeyFilePath { field, source: _ } => field,
        }
        .to_owned()
    }
}

#[derive(Error, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FilePathError {
    #[error("File Path failed to Canonicalize: {input}, {kind}: {message}.")]
    FilePathIsUnresolvable {
        input: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },

    #[error("File Path destination is not a file: {input}, {kind}: {message}.")]
    FilePathIsNotAvailable {
        input: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
}

pub mod helpers {
    use std::fs::{File, OpenOptions};
    use std::path::{Path, PathBuf};

    use crate::errors::FilePathError;

    pub fn get_file_at(at: &PathBuf, mode: &OpenOptions) -> Result<(File, PathBuf), FilePathError> {
        let file = mode.open(at).map_err(|error| FilePathError::FilePathIsNotAvailable {
            input: at.to_owned(),
            kind: error.kind(),
            message: error.to_string(),
        })?;

        let at = Path::new(at)
            .canonicalize()
            .map_err(|error| FilePathError::FilePathIsUnresolvable {
                input: at.to_owned(),
                kind: error.kind(),
                message: error.to_string(),
            })?;

        Ok((file, at))
    }
}
