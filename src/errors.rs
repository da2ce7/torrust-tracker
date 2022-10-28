use thiserror::Error;
use warp::reject::Reject;

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
    ConfigurationError { message: String, source: ServerConfigError },
}

impl Reject for ServerError {}

#[derive(Error, Debug, Clone)]
pub enum ServerConfigError {
    #[error("This Server is Unamed!")]
    UnnamedServer,

    #[error("Binding Address is Empty!")]
    BindingAddressIsEmpty,

    #[error("Binding Address: \"{input}\" has Bad Syntax: {source}")]
    BindingAddressBadSyntax {
        input: String,
        source: std::net::AddrParseError,
    },

    #[error("Bad TLS Configuration: {source}")]
    BadHttpTlsConfig { source: HttpTlsConfigError },
}

#[derive(Error, Debug, Clone)]
pub enum HttpTlsConfigError {
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
