use std::fs::OpenOptions;
use std::path::Path;

use super::{Settings, TrackerSettingsBuilder};
use crate::config_const::{CONFIG_DEFAULT, CONFIG_FOLDER};

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

    pub fn load() {}

    pub fn write(&self) {}

    pub fn write_default() -> Result<(), std::io::Error> {
        let writer = OpenOptions::new()
            .read(false)
            .write(true)
            .create(true)
            .open(Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT).with_extension("new.json"))?;

        let output = Self::default().settings;

        serde_json::to_writer_pretty(writer, &output).unwrap();

        Ok(())
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
