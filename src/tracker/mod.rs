pub mod core;
pub mod key;
pub mod mode;
pub mod peer;
pub mod statistics;
pub mod torrent;

pub mod helpers {
    use std::net::IpAddr;
    use std::sync::Arc;

    use super::mode::TrackerMode;
    use super::statistics::{StatsTracker, TrackerStatsService};
    use crate::databases::database::{self, Database};
    use crate::settings::{
        CommonSettings, CommonSettingsBuilder, DatabaseSettingsBuilder, GlobalSettings, GlobalSettingsBuilder, LogFilterLevel,
    };

    pub struct TrackerArgs {
        pub global: Arc<GlobalSettings>,
        pub common: Arc<CommonSettings>,
        pub stats_tracker: fn() -> Box<dyn TrackerStatsService>,
        pub database: fn() -> Box<dyn Database>,
    }

    impl Default for TrackerArgs {
        fn default() -> Self {
            Self {
                global: Arc::new(GlobalSettingsBuilder::default().try_into().unwrap()),
                common: Arc::new(CommonSettingsBuilder::default().try_into().unwrap()),
                stats_tracker: || Box::new(StatsTracker::new_active_instance()),
                database: || database::connect_database(&DatabaseSettingsBuilder::default().try_into().unwrap()).unwrap(),
            }
        }
    }

    impl TrackerArgs {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn mode(self, tracker_mode: TrackerMode) -> Self {
            TrackerArgs {
                global: Arc::new(
                    GlobalSettingsBuilder::from(self.global)
                        .with_mode(tracker_mode)
                        .try_into()
                        .unwrap(),
                ),
                common: self.common,
                stats_tracker: self.stats_tracker,
                database: self.database,
            }
        }

        pub fn external_ip(self, external_ip: &IpAddr) -> Self {
            TrackerArgs {
                global: Arc::new(
                    GlobalSettingsBuilder::from(self.global)
                        .with_external_ip(external_ip)
                        .try_into()
                        .unwrap(),
                ),
                common: self.common,
                stats_tracker: self.stats_tracker,
                database: self.database,
            }
        }

        pub fn no_logs(self) -> Self {
            TrackerArgs {
                global: Arc::new(
                    GlobalSettingsBuilder::from(self.global)
                        .with_log_filter(&LogFilterLevel::Off)
                        .try_into()
                        .unwrap(),
                ),
                common: self.common,
                stats_tracker: self.stats_tracker,
                database: self.database,
            }
        }
    }
}
