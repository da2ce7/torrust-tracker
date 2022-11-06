use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;
use warp::reject::Reject;

pub mod settings;
pub mod settings_manager;
pub mod wrappers;

#[derive(Error, Clone, Copy, Debug, Eq, Hash, PartialEq)]
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

#[derive(Error, Clone, Debug, Eq, Hash, PartialEq)]
pub enum FilePathError {
    #[error("File Path failed to Canonicalize: {input} : {source}.")]
    FilePathIsUnresolvable { input: PathBuf, source: Arc<wrappers::IoError> },

    #[error("File Path destination is not a file: {input} : {source}.")]
    FilePathIsNotAvailable { input: PathBuf, source: Arc<wrappers::IoError> },
}
