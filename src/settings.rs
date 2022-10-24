use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use config::{Config, ConfigError, File};
use log::info;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};

use crate::config_const::{CONFIG_DEFAULT, CONFIG_FOLDER, CONFIG_LOCAL, CONFIG_OLD_LOCAL, CONFIG_OVERRIDE};
use crate::databases::database::DatabaseDrivers;
use crate::mode::TrackerMode;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct UdpTrackerConfig {
    pub enabled: bool,
    pub bind_address: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HttpTrackerConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl_enabled: bool,
    #[serde_as(as = "NoneAsEmptyString")]
    pub ssl_cert_path: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub ssl_key_path: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HttpApiConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub access_tokens: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Settings {
    pub log_level: Option<String>,
    pub mode: TrackerMode,
    pub db_driver: DatabaseDrivers,
    pub db_path: String,
    pub announce_interval: u32,
    pub min_announce_interval: u32,
    pub max_peer_timeout: u32,
    pub on_reverse_proxy: bool,
    pub external_ip: Option<String>,
    pub tracker_usage_statistics: bool,
    pub persistent_torrent_completed_stat: bool,
    pub inactive_peer_cleanup_interval: u64,
    pub remove_peerless_torrents: bool,
    pub udp_trackers: Vec<UdpTrackerConfig>,
    pub http_trackers: Vec<HttpTrackerConfig>,
    pub http_api: HttpApiConfig,
}

#[derive(Debug)]
pub enum ConfigurationError {
    IOError { error: std::io::Error },
    ParseError { error: toml::de::Error },
    EncodeError { error: toml::ser::Error },
    DecodeError { error: ConfigError },
    TrackerModeIncompatible,
    MissingConfigurationError { error: String },
    RenameFailedError { error: String },
}

impl Settings {
    pub fn default() -> Result<Self, ConfigurationError> {
        let default_source = Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT);
        let mut sources: Vec<PathBuf> = Vec::new();
        Self::check_source(&default_source).map(|_| sources.push(default_source))?;
        let settings = Self::load(&sources)?;
        Ok(settings)
    }

    pub fn new() -> Result<Self, ConfigurationError> {
        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL);

        Self::migrate_old_config()?;

        let sources = Self::get_sources()?;
        let settings = Self::load(&sources)?;

        settings.write(&local_source)?;

        Ok(settings)
    }

    pub fn migrate_old_config() -> Result<(), ConfigurationError> {
        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL);
        let old_local_source = Path::new(CONFIG_FOLDER).join(CONFIG_OLD_LOCAL);

        let mut sources: Vec<PathBuf> = Vec::new();

        if match Self::check_source(&old_local_source) {
            Ok(_) => true,
            Err(ConfigurationError::MissingConfigurationError { error: e }) => {
                info!("No old configuration was found... skipping: {e:?}");
                return Ok(());
            }
            Err(ConfigurationError::DecodeError { error: e }) => {
                eprintln!("Old Configuration was not properly decoded... skipping: {e:?}");
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        } {
            sources.push(old_local_source.clone())
        }

        let settings = Self::load(&sources)?;
        settings.write(&local_source)?;

        match fs::rename(
            old_local_source.with_extension("toml"),
            old_local_source.with_extension("toml.old"),
        ) {
            Ok(_) => Ok(()),
            Err(e) => Err(ConfigurationError::RenameFailedError { error: format!("{e:?}") }),
        }
    }

    fn check_source(source: &Path) -> Result<(), ConfigurationError> {
        if !source.with_extension("toml").exists() {
            let source_display = source.display();
            return Err(ConfigurationError::MissingConfigurationError {
                error: format!("No Configuration File Found at: {source_display}"),
            });
        }

        match Config::builder().add_source(File::from(source)).build() {
            Ok(_) => Ok(()),
            Err(e) => Err(ConfigurationError::DecodeError { error: e }),
        }
    }

    fn get_sources() -> Result<Vec<PathBuf>, ConfigurationError> {
        let default_source = Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT);
        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL);
        let override_source = Path::new(CONFIG_FOLDER).join(CONFIG_OVERRIDE);

        let mut sources: Vec<PathBuf> = Vec::new();

        Self::check_source(&default_source).map(|_| sources.push(default_source))?;

        if match Self::check_source(&local_source) {
            Ok(_) => true,
            Err(ConfigurationError::MissingConfigurationError { error: _ }) => false,
            Err(e) => return Err(e),
        } {
            sources.push(local_source)
        }

        if match Self::check_source(&override_source) {
            Ok(_) => true,
            Err(ConfigurationError::MissingConfigurationError { error: _ }) => false,
            Err(e) => return Err(e),
        } {
            sources.push(override_source)
        }

        Ok(sources)
    }

    fn load(sources: &Vec<PathBuf>) -> Result<Self, ConfigurationError> {
        let mut config_builder = Config::builder();

        for source in sources {
            config_builder = config_builder.add_source(File::from(source.clone()));
        }

        let setting = match config_builder.build() {
            Ok(s) => s,
            Err(e) => return Err(ConfigurationError::DecodeError { error: e }),
        };

        match setting.try_deserialize() {
            Ok(s) => Ok(s),
            Err(e) => Err(ConfigurationError::DecodeError { error: e }),
        }
    }

    fn write(&self, destination: &Path) -> Result<(), ConfigurationError> {
        let toml_string = match toml::to_string(self) {
            Ok(s) => s,
            Err(e) => return Err(ConfigurationError::EncodeError { error: e }),
        };

        match fs::write(destination.with_extension("toml"), toml_string) {
            Ok(_) => Ok(()),
            Err(e) => Err(ConfigurationError::IOError { error: e }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use uuid::Uuid;

    use crate::settings::Settings;

    #[test]
    fn configuration_should_contain_the_external_ip() {
        let settings = Settings::default().unwrap();
        assert_eq!(settings.external_ip, Option::Some(String::from("0.0.0.0")));
    }

    #[test]
    fn configuration_should_be_saved_in_a_toml_config_file() {
        let temp_config_path = env::temp_dir().as_path().join(format!("test_config_{}.toml", Uuid::new_v4()));

        let settings = Settings::new().unwrap();

        settings
            .write(&temp_config_path)
            .expect("Could not save configuration to file");

        let contents = fs::read_to_string(&temp_config_path).unwrap();

        assert_eq!(contents, toml::to_string(&settings).unwrap());
    }
}
