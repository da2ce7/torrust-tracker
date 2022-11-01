use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::error::SendError;
use tokio::sync::{RwLock, RwLockReadGuard};

use super::key;
use super::mode::TrackerMode;
use super::peer::TorrentPeer;
use super::statistics::{TrackerStatistics, TrackerStatisticsEvent, TrackerStatsService};
use crate::databases::database;
use crate::databases::database::Database;
use crate::protocol::common::InfoHash;
use crate::settings::{CommonSettings, GlobalSettings};
use crate::tracker::key::AuthKey;
use crate::tracker::torrent::{TorrentEntry, TorrentError, TorrentStats};

pub struct TorrentTracker {
    pub mode: TrackerMode,
    pub external_ip: Option<IpAddr>,
    pub on_reverse_proxy: bool,
    pub common: Arc<CommonSettings>,
    keys: RwLock<std::collections::HashMap<String, AuthKey>>,
    whitelist: RwLock<std::collections::HashSet<InfoHash>>,
    torrents: RwLock<std::collections::BTreeMap<InfoHash, TorrentEntry>>,
    stats_tracker: Box<dyn TrackerStatsService>,
    database: Box<dyn Database>,
}

impl TorrentTracker {
    pub fn new(
        global: &Arc<GlobalSettings>,
        common: &Arc<CommonSettings>,
        stats_tracker: Box<dyn TrackerStatsService>,
        database: Box<dyn Database>,
    ) -> Result<TorrentTracker, r2d2::Error> {
        Ok(TorrentTracker {
            mode: global.get_tracker_mode(),
            external_ip: global.get_external_ip_opt(),
            on_reverse_proxy: global.is_on_reverse_proxy().unwrap(),
            common: common.to_owned(),
            keys: RwLock::new(std::collections::HashMap::new()),
            whitelist: RwLock::new(std::collections::HashSet::new()),
            torrents: RwLock::new(std::collections::BTreeMap::new()),
            stats_tracker,
            database,
        })
    }

    pub fn is_public(&self) -> bool {
        self.mode == TrackerMode::Public
    }

    pub fn is_private(&self) -> bool {
        self.mode == TrackerMode::Private || self.mode == TrackerMode::PrivateListed
    }

    pub fn is_whitelisted(&self) -> bool {
        self.mode == TrackerMode::Listed || self.mode == TrackerMode::PrivateListed
    }

    pub async fn generate_auth_key(&self, lifetime: Duration) -> Result<AuthKey, database::Error> {
        let auth_key = key::generate_auth_key(lifetime);
        self.database.add_key_to_keys(&auth_key).await?;
        self.keys.write().await.insert(auth_key.key.clone(), auth_key.clone());
        Ok(auth_key)
    }

    pub async fn remove_auth_key(&self, key: &str) -> Result<(), database::Error> {
        self.database.remove_key_from_keys(&key).await?;
        self.keys.write().await.remove(key);
        Ok(())
    }

    pub async fn verify_auth_key(&self, auth_key: &AuthKey) -> Result<(), key::Error> {
        match self.keys.read().await.get(&auth_key.key) {
            None => Err(key::Error::KeyInvalid),
            Some(key) => key::verify_auth_key(key),
        }
    }

    pub async fn load_keys(&self) -> Result<(), database::Error> {
        let keys_from_database = self.database.load_keys().await?;
        let mut keys = self.keys.write().await;

        keys.clear();

        for key in keys_from_database {
            let _ = keys.insert(key.key.clone(), key);
        }

        Ok(())
    }

    // Adding torrents is not relevant to public trackers.
    pub async fn add_torrent_to_whitelist(&self, info_hash: &InfoHash) -> Result<(), database::Error> {
        self.add_torrent_to_database_whitelist(info_hash).await?;
        self.add_torrent_to_memory_whitelist(info_hash).await;
        Ok(())
    }

    async fn add_torrent_to_database_whitelist(&self, info_hash: &InfoHash) -> Result<(), database::Error> {
        self.database.add_info_hash_to_whitelist(*info_hash).await?;
        Ok(())
    }

    pub async fn add_torrent_to_memory_whitelist(&self, info_hash: &InfoHash) -> bool {
        self.whitelist.write().await.insert(*info_hash)
    }

    // Removing torrents is not relevant to public trackers.
    pub async fn remove_torrent_from_whitelist(&self, info_hash: &InfoHash) -> Result<(), database::Error> {
        self.database.remove_info_hash_from_whitelist(info_hash.clone()).await?;
        self.whitelist.write().await.remove(info_hash);
        Ok(())
    }

    pub async fn is_info_hash_whitelisted(&self, info_hash: &InfoHash) -> bool {
        self.whitelist.read().await.contains(info_hash)
    }

    pub async fn load_whitelist(&self) -> Result<(), database::Error> {
        let whitelisted_torrents_from_database = self.database.load_whitelist().await?;
        let mut whitelist = self.whitelist.write().await;

        whitelist.clear();

        for info_hash in whitelisted_torrents_from_database {
            let _ = whitelist.insert(info_hash);
        }

        Ok(())
    }

    pub async fn authenticate_request(&self, info_hash: &InfoHash, key: &Option<AuthKey>) -> Result<(), TorrentError> {
        // no authentication needed in public mode
        if self.is_public() {
            return Ok(());
        }

        // check if auth_key is set and valid
        if self.is_private() {
            match key {
                Some(key) => {
                    if self.verify_auth_key(key).await.is_err() {
                        return Err(TorrentError::PeerKeyNotValid);
                    }
                }
                None => {
                    return Err(TorrentError::PeerNotAuthenticated);
                }
            }
        }

        // check if info_hash is whitelisted
        if self.is_whitelisted() {
            if !self.is_info_hash_whitelisted(info_hash).await {
                return Err(TorrentError::TorrentNotWhitelisted);
            }
        }

        Ok(())
    }

    // Loading the torrents from database into memory
    pub async fn load_persistent_torrents(&self) -> Result<(), database::Error> {
        let persistent_torrents = self.database.load_persistent_torrents().await?;
        let mut torrents = self.torrents.write().await;

        for (info_hash, completed) in persistent_torrents {
            // Skip if torrent entry already exists
            if torrents.contains_key(&info_hash) {
                continue;
            }

            let torrent_entry = TorrentEntry {
                peers: Default::default(),
                completed,
            };

            torrents.insert(info_hash.clone(), torrent_entry);
        }

        Ok(())
    }

    /// Get all torrent peers for a given torrent filtering out the peer with the client address
    pub async fn get_torrent_peers(&self, info_hash: &InfoHash, client_addr: &SocketAddr) -> Vec<TorrentPeer> {
        let read_lock = self.torrents.read().await;

        match read_lock.get(info_hash) {
            None => vec![],
            Some(entry) => entry.get_peers(Some(client_addr)).into_iter().cloned().collect(),
        }
    }

    /// Get all torrent peers for a given torrent
    pub async fn get_all_torrent_peers(&self, info_hash: &InfoHash) -> Vec<TorrentPeer> {
        let read_lock = self.torrents.read().await;

        match read_lock.get(info_hash) {
            None => vec![],
            Some(entry) => entry.get_peers(None).into_iter().cloned().collect(),
        }
    }

    pub async fn update_torrent_with_peer_and_get_stats(&self, info_hash: &InfoHash, peer: &TorrentPeer) -> TorrentStats {
        let mut torrents = self.torrents.write().await;

        let torrent_entry = match torrents.entry(info_hash.clone()) {
            Entry::Vacant(vacant) => vacant.insert(TorrentEntry::new()),
            Entry::Occupied(entry) => entry.into_mut(),
        };

        let stats_updated = torrent_entry.update_peer(peer);

        // todo: move this action to a separate worker
        if self.common.enable_persistent_statistics.unwrap() && stats_updated {
            let _ = self
                .database
                .save_persistent_torrent(&info_hash, torrent_entry.completed)
                .await;
        }

        let (seeders, completed, leechers) = torrent_entry.get_stats();

        TorrentStats {
            seeders,
            leechers,
            completed,
        }
    }

    pub async fn get_torrents(&self) -> RwLockReadGuard<'_, BTreeMap<InfoHash, TorrentEntry>> {
        self.torrents.read().await
    }

    pub async fn get_stats(&self) -> RwLockReadGuard<'_, TrackerStatistics> {
        self.stats_tracker.get_stats().await
    }

    pub async fn send_stats_event(&self, event: TrackerStatisticsEvent) -> Option<Result<(), SendError<TrackerStatisticsEvent>>> {
        self.stats_tracker.send_event(event).await
    }

    // Remove inactive peers and (optionally) peerless torrents
    pub async fn cleanup_torrents(&self) {
        let mut torrents_lock = self.torrents.write().await;

        // If we don't need to remove torrents we will use the faster iter
        if self.common.enable_peerless_torrent_pruning.unwrap() {
            torrents_lock.retain(|_, torrent_entry| {
                torrent_entry.remove_inactive_peers(self.common.peer_timeout_seconds_maximum.unwrap());

                match self.common.enable_persistent_statistics.unwrap() {
                    true => torrent_entry.completed > 0 || torrent_entry.peers.len() > 0,
                    false => torrent_entry.peers.len() > 0,
                }
            });
        } else {
            for (_, torrent_entry) in torrents_lock.iter_mut() {
                torrent_entry.remove_inactive_peers(self.common.peer_timeout_seconds_maximum.unwrap());
            }
        }
    }
}
