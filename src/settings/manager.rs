use core::slice::SlicePattern;
use std::fs::OpenOptions;
use std::io::{self, BufReader, BufRead, Read};
use std::path::Path;

use super::{Settings, TrackerSettingsBuilder};
use crate::config_const::{CONFIG_DEFAULT, CONFIG_FOLDER, CONFIG_LOCAL, CONFIG_OLD_LOCAL};
use crate::errors::helpers::get_existing_file_path;
use crate::errors::SettingsManagerError;

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
    pub fn new() -> Self {
        let settings: Settings = Default::default();

        Self {
            settings: Settings {
                namespace: settings.namespace,
                version: settings.version,
                tracker: TrackerSettingsBuilder::empty().try_into().unwrap(),
            },
        }
    }

    pub fn import_old() -> Result<Self, SettingsManagerError> {
        let path = Path::new(CONFIG_FOLDER).join(CONFIG_OLD_LOCAL).with_extension(".toml");

        if let Err(source) = get_existing_file_path(&path) {
            return Err(SettingsManagerError::NoExistingConfigFile { source });
        };

        match OpenOptions::new().read(true).write(false).create(false).open(&path) {
            Ok(rdr) => { 
                let mut res  = Vec::<u8>::new();
                BufReader::new(rdr).read_to_end(&mut res);
                toml::de::from_slice(res.as_slice())

            }
            Err(source) => Err(SettingsManagerError::FailedToOpenFile { path, source }),

        toml::de::from_slice(bytes)

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

    pub fn read() -> Result<Self, SettingsManagerError> {
        let path = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL).with_extension("new.json");

        if let Err(source) = get_existing_file_path(&path) {
            return Err(SettingsManagerError::NoExistingConfigFile { source });
        }

        match OpenOptions::new().read(true).write(false).create(false).open(&path) {
            Ok(rdr) => Self::read_json(rdr).map_err(|source| SettingsManagerError::FailedToReadIn { path, source }),
            Err(source) => Err(SettingsManagerError::FailedToOpenFile { path, source }),
        }
    }

    pub fn write_json<W>(&self, writer: W) -> Result<(), serde_json::Error>
    where
        W: io::Write,
    {
        serde_json::to_writer_pretty(writer, &self.settings)
    }

    pub fn write_default() -> Result<(), SettingsManagerError> {
        let path = Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT).with_extension("new.json");

        match OpenOptions::new().read(false).write(true).create(true).open(&path) {
            Ok(writer) => Self::default()
                .write_json(writer)
                .map_err(|source| SettingsManagerError::FailedToWriteOut { path, source }),
            Err(source) => Err(SettingsManagerError::FailedToOpenFile { path, source }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SettingsManager;

    #[test]
    fn it_should_write_default_config() {
        SettingsManager::write_default().unwrap()
    }
}
