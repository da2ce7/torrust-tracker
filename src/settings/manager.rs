use std::collections::hash_map::DefaultHasher;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use log::{info, warn};

use super::{
    CommonSettings, CommonSettingsBuilder, DatabaseSettings, DatabaseSettingsBuilder, GlobalSettings, GlobalSettingsBuilder,
    ServicesBuilder, Settings, TrackerSettings, TrackerSettingsBuilder,
};
use crate::config_const::{CONFIG_DEFAULT, CONFIG_FOLDER, CONFIG_LOCAL, CONFIG_OLD};
use crate::errors::helpers::get_existing_file_path;
use crate::errors::SettingsManagerError;
use crate::settings::old_settings;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct SettingsManager {
    settings: Settings,
}

impl Default for SettingsManager {
    fn default() -> Self {
        let settings: Settings = Default::default();
        Self {
            settings: Settings {
                namespace: settings.namespace,
                version: settings.version,
                tracker: TrackerSettingsBuilder::default().try_into().unwrap(),
            },
        }
    }
}

impl SettingsManager {
    pub fn empty() -> Self {
        let settings: Settings = Default::default();

        Self {
            settings: Settings {
                namespace: settings.namespace,
                version: settings.version,
                tracker: TrackerSettingsBuilder::empty().tracker_settings,
            },
        }
    }

    pub fn setup() -> Result<Self, SettingsManagerError> {
        let config = Path::new(CONFIG_FOLDER);
        let default = &Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT).with_extension("json");
        let old = &Path::new(CONFIG_FOLDER).join(CONFIG_OLD).with_extension("toml");
        let local = &Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL).with_extension("json");

        Self::make_folder(&config.to_path_buf())?;

        let sm = Self::load(default, old, local)?;

        sm.save(local)?;

        Ok(sm)
    }

    pub fn make_folder(config: &PathBuf) -> Result<(), SettingsManagerError> {
        if let Ok(path) = config.canonicalize() {
            if path.is_dir() {
                return Ok(());
            } else {
                return Err(SettingsManagerError::NotDirectory { path });
            }
        }
        match fs::create_dir(config) {
            Ok(_) => Ok(()),
            Err(source) => Err(SettingsManagerError::FailedToCreateConfigDirectory {
                path: config.to_owned(),
                source,
            }),
        }
    }

    pub fn load(default: &PathBuf, old: &PathBuf, local: &PathBuf) -> Result<Self, SettingsManagerError> {
        Self::write_default(default)?;

        let old_settings = match Self::import_old(old) {
            Ok(settings) => Some(settings),
            Err(error) => match error {
                SettingsManagerError::NoExistingConfigFile { source } => {
                    info!("No Old Configuration To Load: {source}");
                    None
                }
                error => {
                    return Err(error);
                }
            },
        };

        if let Some(res) = old_settings {
            return Ok(res);
        };

        // If no old settings, lets try the local settings.
        let local_settings = match Self::read(local) {
            Ok(settings) => Some(settings),
            Err(error) => match error {
                SettingsManagerError::NoExistingConfigFile { source } => {
                    info!("No Configuration To Load: {source}");
                    None
                }
                error => {
                    return Err(error);
                }
            },
        };

        if let Some(res) = local_settings {
            return Ok(res);
        };

        // if nothing else, lets load the default.
        Ok(Self::default())
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), SettingsManagerError> {
        let path_to = match get_existing_file_path(path).map_err(|source| SettingsManagerError::NoExistingConfigFile { source }) {
            Ok(path) => Ok(path),
            Err(source) => match source {
                SettingsManagerError::NoExistingConfigFile { source: _ } => Err(path),
                source => {
                    return Err(source);
                }
            },
        };

        // lets backup the previous configuration, if we have any...
        if let Ok(path_from) = path_to {
            let hash = match OpenOptions::new().read(true).write(false).create(false).open(&path_from) {
                Ok(rdr) => {
                    let mut data = Vec::<u8>::new();
                    BufReader::new(rdr)
                        .read_to_end(&mut data)
                        .map_err(|source| SettingsManagerError::FailedToReadOld {
                            path_from: path_from.to_owned(),
                            source,
                        })?;

                    let mut hasher = DefaultHasher::new();
                    data.hash(&mut hasher);
                    hasher.finish()
                }
                Err(source) => {
                    return Err(SettingsManagerError::FailedToOpenFile { path: path_from, source });
                }
            };

            let path_backup = path_from.with_extension(format!("{}-{}", "json.backup", hash));

            // if import was success, lets rename the extension to ".toml.old".
            match fs::rename(&path_from, &path_backup) {
                Ok(_) => {
                    info!(
                        "\nPrevious Settings Was Successfully Backed Up!\nFrom: \"{}\", and moved to: \"{}\".\n",
                        path_from.display(),
                        path_backup.display()
                    );
                }
                Err(source) => {
                    return Err(SettingsManagerError::FailedToMoveOldSettingsFile {
                        path_from,
                        path_to: path_backup,
                        source,
                    });
                }
            }
        };

        self.write(path)
    }

    pub fn write_default(path: &PathBuf) -> Result<(), SettingsManagerError> {
        let path_or = get_existing_file_path(path).map_err(|source| SettingsManagerError::NoExistingConfigFile { source });
        let path_to = match path_or {
            Ok(path) => Ok(path),
            Err(error) => match error {
                SettingsManagerError::NoExistingConfigFile { source: _ } => Err(path.to_owned()),
                error => {
                    return Err(error);
                }
            },
        }
        .map_or_else(|p| p, |f| f);

        match OpenOptions::new().read(false).write(true).create(true).open(&path_to) {
            Ok(writer) => Self::default()
                .write_json(writer)
                .map_err(|source| SettingsManagerError::FailedToWriteOut {
                    path_to: path_to.to_owned(),
                    source,
                }),
            Err(source) => Err(SettingsManagerError::FailedToOpenFile { path: path_to, source }),
        }
    }

    pub fn read_json<R>(rdr: R) -> Result<Self, serde_json::Error>
    where
        R: io::Read,
    {
        match serde_json::from_reader(rdr) {
            Ok(settings) => Ok(Self { settings }),
            Err(error) => Err(error),
        }
    }

    pub fn read(path: &PathBuf) -> Result<Self, SettingsManagerError> {
        let path_from = get_existing_file_path(path).map_err(|source| SettingsManagerError::NoExistingConfigFile { source })?;

        match OpenOptions::new().read(true).write(false).create(false).open(&path_from) {
            Ok(rdr) => Self::read_json(rdr).map_err(|source| SettingsManagerError::FailedToReadIn { path_from, source }),
            Err(source) => Err(SettingsManagerError::FailedToOpenFile { path: path_from, source }),
        }
    }

    pub fn write_json<W>(&self, writer: W) -> Result<(), serde_json::Error>
    where
        W: io::Write,
    {
        serde_json::to_writer_pretty(writer, &self.settings)
    }

    pub fn write(&self, path: &PathBuf) -> Result<(), SettingsManagerError> {
        let path_to = match get_existing_file_path(path).map_err(|source| SettingsManagerError::NoExistingConfigFile { source }) {
            Ok(path) => {
                return Err(SettingsManagerError::ExistingFile { path });
            }
            Err(source) => match source {
                SettingsManagerError::NoExistingConfigFile { source: _ } => path,
                source => {
                    return Err(source);
                }
            },
        };

        match OpenOptions::new().read(false).write(true).create(true).open(path_to) {
            Ok(writer) => self
                .write_json(&writer)
                .map_err(|source| SettingsManagerError::FailedToWriteOut {
                    path_to: path_to.to_owned(),
                    source,
                }),
            Err(source) => Err(SettingsManagerError::FailedToOpenFile {
                path: path_to.to_owned(),
                source,
            }),
        }
    }

    pub fn import_old(path: &PathBuf) -> Result<Self, SettingsManagerError> {
        let path_from = get_existing_file_path(path).map_err(|source| SettingsManagerError::NoExistingConfigFile { source })?;

        let mut old_settings_data = Vec::<u8>::new();
        let old_settings: old_settings::Settings = match OpenOptions::new().read(true).write(false).create(false).open(&path_from)
        {
            Ok(rdr) => {
                BufReader::new(rdr).read_to_end(&mut old_settings_data).map_err(|source| {
                    SettingsManagerError::FailedToReadOld {
                        path_from: path_from.to_owned(),
                        source,
                    }
                })?;
                match toml::de::from_slice(old_settings_data.as_slice()) {
                    Ok(old) => old,
                    Err(source) => {
                        return Err(SettingsManagerError::FailedToParseInOld { path_from, source });
                    }
                }
            }
            Err(source) => {
                return Err(SettingsManagerError::FailedToOpenFile { path: path_from, source });
            }
        };

        let mut builder = TrackerSettingsBuilder::empty();

        let import_try = builder.to_owned().import_old(&old_settings);

        if let Err(error) = TryInto::<TrackerSettings>::try_into(import_try.to_owned()) {
            let broken_path = path.with_file_name("config-import").with_extension("fail.json");
            warn!(
                "Failed to successfully import settings. Import attempt is saved at: {}\nWith Error: {}",
                broken_path.to_string_lossy(),
                error
            );
            let empty = Self::empty();
            let broken = Self {
                settings: Settings {
                    namespace: empty.settings.namespace,
                    version: error.to_string(),
                    tracker: import_try.to_owned().tracker_settings,
                },
            };

            broken.save(&broken_path)?;

            let defaults: TrackerSettings = TrackerSettingsBuilder::default().try_into().unwrap();

            if TryInto::<GlobalSettings>::try_into(GlobalSettingsBuilder::from(&import_try.tracker_settings.global.unwrap()))
                .is_err()
            {
                builder.tracker_settings.global = defaults.global;
            }

            if TryInto::<CommonSettings>::try_into(CommonSettingsBuilder::from(&import_try.tracker_settings.common.unwrap()))
                .is_err()
            {
                builder.tracker_settings.common = defaults.common;
            }

            if TryInto::<DatabaseSettings>::try_into(DatabaseSettingsBuilder::from(
                &import_try.tracker_settings.database.unwrap(),
            ))
            .is_err()
            {
                builder.tracker_settings.database = defaults.database;
            }
        }

        builder = builder.to_owned().import_old(&old_settings);

        if let Err(error) = TryInto::<TrackerSettings>::try_into(builder.to_owned()) {
            let broken_path = path.with_file_name("config-import-with-defaults").with_extension("fail.json");
            warn!(
                "Failed to successfully import settings. Import attempt is saved at: {}\nWith Error: {}",
                broken_path.to_string_lossy(),
                error
            );
            let empty = Self::empty();
            let broken = Self {
                settings: Settings {
                    namespace: empty.settings.namespace,
                    version: error.to_string(),
                    tracker: builder.to_owned().tracker_settings,
                },
            };

            broken.save(&broken_path)?;

            let defaults: TrackerSettings = TrackerSettingsBuilder::default().try_into().unwrap();

            if TryInto::<GlobalSettings>::try_into(GlobalSettingsBuilder::from(builder.tracker_settings.global.as_ref().unwrap()))
                .is_err()
            {
                builder.tracker_settings.global = defaults.global;
            }

            if TryInto::<CommonSettings>::try_into(CommonSettingsBuilder::from(builder.tracker_settings.common.as_ref().unwrap()))
                .is_err()
            {
                builder.tracker_settings.common = defaults.common;
            }

            if TryInto::<DatabaseSettings>::try_into(DatabaseSettingsBuilder::from(
                builder.tracker_settings.database.as_ref().unwrap(),
            ))
            .is_err()
            {
                builder.tracker_settings.database = defaults.database;
            }

            let mut service_builder = ServicesBuilder::from(&builder.tracker_settings.services.unwrap());
            service_builder.remove_check_fail();
            builder.tracker_settings.services = Some(service_builder.try_into().unwrap());
        }

        let imported = match TryInto::<TrackerSettings>::try_into(builder) {
            Ok(tracker) => {
                let settings: Settings = Default::default();
                Self {
                    settings: Settings {
                        namespace: settings.namespace,
                        version: settings.version,
                        tracker,
                    },
                }
            }
            Err(source) => {
                return Err(SettingsManagerError::FailedToImportOldSettings {
                    path_from,
                    source: Box::new(source),
                });
            }
        };

        let mut hasher = DefaultHasher::new();
        old_settings_data.hash(&mut hasher);

        let path_backup = path_from.with_extension(format!("{}-{}", "toml.old", hasher.finish()));

        // if import was success, lets rename the extension to ".toml.old".
        match fs::rename(&path_from, &path_backup) {
            Ok(_) => {
                info!(
                    "\nOld Settings Was Successfully Imported!\n And moved from: \"{}\", to: \"{}\".\n",
                    path_from.display(),
                    path_backup.display()
                );
                Ok(imported)
            }
            Err(source) => Err(SettingsManagerError::FailedToMoveOldSettingsFile {
                path_from,
                path_to: path_backup,
                source,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use uuid::Uuid;

    use super::SettingsManager;

    #[test]
    fn it_should_write_the_default() {
        let temp = env::temp_dir().as_path().join(format!("test_config_{}.json", Uuid::new_v4()));

        assert!(!temp.exists());

        SettingsManager::write_default(&temp).unwrap();

        assert!(temp.is_file());
    }

    #[test]
    fn it_should_make_config_folder() {
        let temp = env::temp_dir().as_path().join(format!("test_config_{}", Uuid::new_v4()));

        assert!(!temp.exists());

        SettingsManager::make_folder(&temp).unwrap();

        assert!(temp.is_dir());
    }

    #[test]
    fn it_should_write_and_read_and_write_default_config() {
        SettingsManager::setup().unwrap();
    }
}
