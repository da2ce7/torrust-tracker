use thiserror::Error;

use super::FilePathError;
use crate::databases::database::DatabaseDrivers;
use crate::settings::{
    CommonSettings, DatabaseSettings, GlobalSettings, ServiceNoSecrets, ServiceProtocol, TlsSettings, TrackerSettings,
};

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
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
