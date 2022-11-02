use derive_more::Display;
use serde;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Debug, Hash, Display)]
#[serde(rename_all = "snake_case")]
pub enum TrackerMode {
    // Will track every new info hash and serve every peer.
    Public,

    // Will only track whitelisted info hashes.
    Listed,

    // Will only serve authenticated peers
    Private,

    // Will only track whitelisted info hashes and serve authenticated peers
    PrivateListed,
}

impl Default for TrackerMode {
    fn default() -> Self {
        TrackerMode::Listed
    }
}
