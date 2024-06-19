//! Core tracker domain services.
//!
//! There are two services:
//!
//! - [`get_torrent_info`]: it returns all the data about one torrent.
//! - [`get_torrents`]: it returns data about some torrent in bulk excluding the peer list.
use std::sync::Arc;

use torrust_tracker_primitives::info_hash::InfoHash;
use torrust_tracker_primitives::pagination::Pagination;
use torrust_tracker_primitives::peer;
use torrust_tracker_torrent_repository::entry::EntrySync;
use torrust_tracker_torrent_repository::repository::Repository;

use crate::core::Tracker;

/// It contains all the information the tracker has about a torrent
#[derive(Debug, PartialEq)]
pub struct Info {
    /// The infohash of the torrent this data is related to
    pub info_hash: InfoHash,
    /// The total number of seeders for this torrent. Peer that actively serving a full copy of the torrent data
    pub seeders: u64,
    /// The total number of peers that have ever complete downloading this torrent
    pub completed: u64,
    /// The total number of leechers for this torrent. Peers that actively downloading this torrent
    pub leechers: u64,
    /// The swarm: the list of peers that are actively trying to download or serving this torrent
    pub peers: Option<Vec<peer::Peer>>,
}

/// It contains only part of the information the tracker has about a torrent
///
/// It contains the same data as [Info] but without the list of peers in the swarm.
#[derive(Debug, PartialEq, Clone)]
pub struct BasicInfo {
    /// The infohash of the torrent this data is related to
    pub info_hash: InfoHash,
    /// The total number of seeders for this torrent. Peer that actively serving a full copy of the torrent data
    pub seeders: u64,
    /// The total number of peers that have ever complete downloading this torrent
    pub completed: u64,
    /// The total number of leechers for this torrent. Peers that actively downloading this torrent
    pub leechers: u64,
}

/// It returns all the information the tracker has about one torrent in a [Info] struct.
pub async fn get_torrent_info(tracker: Arc<Tracker>, info_hash: &InfoHash) -> Option<Info> {
    let torrent_entry_option = tracker.torrents.get(info_hash);

    let torrent_entry = torrent_entry_option?;

    let stats = torrent_entry.get_swarm_metadata();

    let peers = torrent_entry.get_peers(None);

    let peers = Some(peers.iter().map(|peer| (**peer)).collect());

    Some(Info {
        info_hash: *info_hash,
        seeders: u64::from(stats.complete),
        completed: u64::from(stats.downloaded),
        leechers: u64::from(stats.incomplete),
        peers,
    })
}

/// It returns all the information the tracker has about multiple torrents in a [`BasicInfo`] struct, excluding the peer list.
pub async fn get_torrents_page(tracker: Arc<Tracker>, pagination: Option<&Pagination>) -> Vec<BasicInfo> {
    let mut basic_infos: Vec<BasicInfo> = vec![];

    for (info_hash, torrent_entry) in tracker.torrents.get_paginated(pagination) {
        let stats = torrent_entry.get_swarm_metadata();

        basic_infos.push(BasicInfo {
            info_hash,
            seeders: u64::from(stats.complete),
            completed: u64::from(stats.downloaded),
            leechers: u64::from(stats.incomplete),
        });
    }

    basic_infos
}

/// It returns all the information the tracker has about multiple torrents in a [`BasicInfo`] struct, excluding the peer list.
pub async fn get_torrents(tracker: Arc<Tracker>, info_hashes: &[InfoHash]) -> Vec<BasicInfo> {
    let mut basic_infos: Vec<BasicInfo> = vec![];

    for info_hash in info_hashes {
        if let Some(stats) = tracker.torrents.get(info_hash).map(|t| t.get_swarm_metadata()) {
            basic_infos.push(BasicInfo {
                info_hash: *info_hash,
                seeders: u64::from(stats.complete),
                completed: u64::from(stats.downloaded),
                leechers: u64::from(stats.incomplete),
            });
        }
    }

    basic_infos
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use torrust_tracker_primitives::announce_event::AnnounceEvent;
    use torrust_tracker_primitives::{peer, DurationSinceUnixEpoch, NumberOfBytes};

    fn sample_peer() -> peer::Peer {
        peer::Peer {
            peer_id: peer::Id(*b"-qB00000000000000000"),
            peer_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(126, 0, 0, 1)), 8080),
            updated: DurationSinceUnixEpoch::new(1_669_397_478_934, 0),
            uploaded: NumberOfBytes(0),
            downloaded: NumberOfBytes(0),
            left: NumberOfBytes(0),
            event: AnnounceEvent::Started,
        }
    }

    mod getting_a_torrent_info {

        use std::str::FromStr;
        use std::sync::Arc;

        use torrust_tracker_configuration::Configuration;
        use torrust_tracker_primitives::info_hash::InfoHash;
        use torrust_tracker_test_helpers::configuration;

        use crate::core::services::torrent::tests::sample_peer;
        use crate::core::services::torrent::{get_torrent_info, Info};
        use crate::core::services::tracker_factory;

        pub fn tracker_configuration() -> Configuration {
            configuration::ephemeral()
        }

        #[tokio::test]
        async fn should_return_none_if_the_tracker_does_not_have_the_torrent() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let torrent_info = get_torrent_info(
                tracker.clone(),
                &InfoHash::from_str("0b3aea4adc213ce32295be85d3883a63bca25446").unwrap(),
            )
            .await;

            assert!(torrent_info.is_none());
        }

        #[tokio::test]
        async fn should_return_the_torrent_info_if_the_tracker_has_the_torrent() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
            let info_hash = InfoHash::from_str(&hash).unwrap();
            tracker.upsert_peer_and_get_stats(&info_hash, &sample_peer()).await;

            let torrent_info = get_torrent_info(tracker.clone(), &info_hash).await.unwrap();

            assert_eq!(
                torrent_info,
                Info {
                    info_hash: InfoHash::from_str(&hash).unwrap(),
                    seeders: 1,
                    completed: 0,
                    leechers: 0,
                    peers: Some(vec![sample_peer()]),
                }
            );
        }
    }

    mod searching_for_torrents {

        use std::str::FromStr;
        use std::sync::Arc;

        use torrust_tracker_configuration::Configuration;
        use torrust_tracker_primitives::info_hash::InfoHash;
        use torrust_tracker_test_helpers::configuration;

        use crate::core::services::torrent::tests::sample_peer;
        use crate::core::services::torrent::{get_torrents_page, BasicInfo, Pagination};
        use crate::core::services::tracker_factory;

        pub fn tracker_configuration() -> Configuration {
            configuration::ephemeral()
        }

        #[tokio::test]
        async fn should_return_an_empty_result_if_the_tracker_does_not_have_any_torrent() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let torrents = get_torrents_page(tracker.clone(), Some(&Pagination::default())).await;

            assert_eq!(torrents, vec![]);
        }

        #[tokio::test]
        async fn should_return_a_summarized_info_for_all_torrents() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
            let info_hash = InfoHash::from_str(&hash).unwrap();

            tracker.upsert_peer_and_get_stats(&info_hash, &sample_peer()).await;

            let torrents = get_torrents_page(tracker.clone(), Some(&Pagination::default())).await;

            assert_eq!(
                torrents,
                vec![BasicInfo {
                    info_hash: InfoHash::from_str(&hash).unwrap(),
                    seeders: 1,
                    completed: 0,
                    leechers: 0,
                }]
            );
        }

        #[tokio::test]
        async fn should_allow_limiting_the_number_of_torrents_in_the_result() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let hash1 = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
            let info_hash1 = InfoHash::from_str(&hash1).unwrap();
            let hash2 = "03840548643af2a7b63a9f5cbca348bc7150ca3a".to_owned();
            let info_hash2 = InfoHash::from_str(&hash2).unwrap();

            tracker.upsert_peer_and_get_stats(&info_hash1, &sample_peer()).await;
            tracker.upsert_peer_and_get_stats(&info_hash2, &sample_peer()).await;

            let offset = 0;
            let limit = 1;

            let torrents = get_torrents_page(tracker.clone(), Some(&Pagination::new(offset, limit))).await;

            assert_eq!(torrents.len(), 1);
        }

        #[tokio::test]
        async fn should_allow_using_pagination_in_the_result() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let hash1 = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
            let info_hash1 = InfoHash::from_str(&hash1).unwrap();
            let hash2 = "03840548643af2a7b63a9f5cbca348bc7150ca3a".to_owned();
            let info_hash2 = InfoHash::from_str(&hash2).unwrap();

            tracker.upsert_peer_and_get_stats(&info_hash1, &sample_peer()).await;
            tracker.upsert_peer_and_get_stats(&info_hash2, &sample_peer()).await;

            let offset = 1;
            let limit = 4000;

            let torrents = get_torrents_page(tracker.clone(), Some(&Pagination::new(offset, limit))).await;

            assert_eq!(torrents.len(), 1);
            assert_eq!(
                torrents,
                vec![BasicInfo {
                    info_hash: InfoHash::from_str(&hash1).unwrap(),
                    seeders: 1,
                    completed: 0,
                    leechers: 0,
                }]
            );
        }

        #[tokio::test]
        async fn should_return_torrents_ordered_by_info_hash() {
            let tracker = Arc::new(tracker_factory(&tracker_configuration()));

            let hash1 = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
            let info_hash1 = InfoHash::from_str(&hash1).unwrap();
            tracker.upsert_peer_and_get_stats(&info_hash1, &sample_peer()).await;

            let hash2 = "03840548643af2a7b63a9f5cbca348bc7150ca3a".to_owned();
            let info_hash2 = InfoHash::from_str(&hash2).unwrap();
            tracker.upsert_peer_and_get_stats(&info_hash2, &sample_peer()).await;

            let torrents = get_torrents_page(tracker.clone(), Some(&Pagination::default())).await;

            assert_eq!(
                torrents,
                vec![
                    BasicInfo {
                        info_hash: InfoHash::from_str(&hash2).unwrap(),
                        seeders: 1,
                        completed: 0,
                        leechers: 0,
                    },
                    BasicInfo {
                        info_hash: InfoHash::from_str(&hash1).unwrap(),
                        seeders: 1,
                        completed: 0,
                        leechers: 0,
                    }
                ]
            );
        }
    }
}
