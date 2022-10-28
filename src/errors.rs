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

    #[error("bad server configuration")]
    ConfigurationError { source: ServerConfigError },
}

impl Reject for ServerError {}

#[derive(Error, Debug)]
pub enum ServerConfigError {
    #[error("server is unamed")]
    UnnamedServer,

    #[error("empty binding address")]
    BindingAddressIsEmpty,

    #[error("failed to parse binding address: {input}")]
    BindingAddressBadSyntax {
        input: String,
        source: std::net::AddrParseError,
    },

    #[error("bad tls configuration")]
    BadTlsConfig { source: HttpTlsConfigError },
}

#[derive(Error, Debug)]
pub enum HttpTlsConfigError {
    #[error("unable to find certificate file")]
    BadCertificateFilePath { source: FilePathError },

    #[error("unable to find key file")]
    BadKeyFilePath { source: FilePathError },
}

#[derive(Error, Debug)]
pub enum FilePathError {
    #[error("empty path")]
    FilePathIsEmpty,

    #[error("failed to canonicalize path: {input}, {message}")]
    FilePathIsUnresolvable { input: String, message: String },

    #[error("failed to locate path: {input}")]
    FilePathDoseNotExist { input: String },

    #[error("path is not a file: {input}")]
    FilePathIsNotAFile { input: String },
}
