use std::collections::BTreeMap;

use serde::Deserialize;
use serde_with::serde_as;

#[derive(Deserialize, Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TrackerModeOld {
    Public,
    Listed,
    Private,
    PrivateListed,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum DatabaseDriversOld {
    Sqlite3,
    MySQL,
}

#[serde_as]
#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct UdpTrackerConfig {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub bind_address: Option<String>,
}

#[serde_as]
#[derive(Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct HttpTrackerConfig {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub bind_address: Option<String>,
    pub ssl_enabled: Option<bool>,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct HttpApiConfig {
    pub enabled: Option<bool>,
    pub bind_address: Option<String>,
    pub access_tokens: Option<BTreeMap<String, String>>,
}

#[serde_as]
#[derive(Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Settings {
    pub log_level: Option<String>,
    pub mode: Option<TrackerModeOld>,
    pub db_driver: Option<DatabaseDriversOld>,
    pub db_path: Option<String>,
    pub announce_interval: Option<u32>,
    pub min_announce_interval: Option<u32>,
    pub max_peer_timeout: Option<u32>,
    pub on_reverse_proxy: Option<bool>,
    pub external_ip: Option<String>,
    pub tracker_usage_statistics: Option<bool>,
    pub persistent_torrent_completed_stat: Option<bool>,
    pub inactive_peer_cleanup_interval: Option<u64>,
    pub remove_peerless_torrents: Option<bool>,
    pub udp_trackers: Option<Vec<UdpTrackerConfig>>,
    pub http_trackers: Option<Vec<HttpTrackerConfig>>,
    pub http_api: Option<HttpApiConfig>,
}

#[cfg(not)]
mod tests {

    use std::path::Path;
    use std::{env, fs};

    use uuid::Uuid;

    use crate::config_const::{CONFIG_FOLDER, CONFIG_LOCAL};
    use crate::settings::old_settings::Settings;

    #[test]
    fn default_settings_should_contain_an_external_ip() {
        let settings = Settings::default().unwrap();
        assert_eq!(settings.external_ip, Option::Some(String::from("0.0.0.0")));
    }

    #[test]
    fn settings_should_be_automatically_saved_into_local_config() {
        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL).with_extension("toml");

        let settings = Settings::new().unwrap();

        let contents = fs::read_to_string(&local_source).unwrap();

        assert_eq!(contents, toml::to_string(&settings).unwrap());
    }

    #[test]
    fn configuration_should_be_saved_in_a_toml_config_file() {
        let temp_config_path = env::temp_dir().as_path().join(format!("test_config_{}.toml", Uuid::new_v4()));

        let settings = Settings::default().unwrap();

        settings
            .write(temp_config_path.as_ref())
            .expect("Could not save configuration to file");

        let contents = fs::read_to_string(&temp_config_path).unwrap();

        assert_eq!(contents, toml::to_string(&settings).unwrap());
    }
}
