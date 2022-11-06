use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use log::{info, warn};

use super::{
    Settings, SettingsErrored, SettingsNamespace, TrackerSettings, TrackerSettingsBuilder, SETTINGS_NAMESPACE,
    SETTINGS_NAMESPACE_ERRORED,
};
use crate::config_const::{CONFIG_BACKUP_FOLDER, CONFIG_DEFAULT, CONFIG_ERROR_FOLDER, CONFIG_FOLDER, CONFIG_LOCAL, CONFIG_OLD};
use crate::errors::settings_manager::SettingsManagerError;
use crate::errors::wrappers::{IoError, SerdeJsonError};
use crate::helpers::get_file_at;
use crate::settings::{Clean, Fix};
use crate::Empty;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct SettingsManager {
    settings: Result<Settings, SettingsErrored>,
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self {
            settings: Ok(Default::default()),
        }
    }
}

impl From<Settings> for SettingsManager {
    fn from(okay: Settings) -> Self {
        Self { settings: Ok(okay) }
    }
}

impl TryFrom<SettingsManager> for Settings {
    type Error = SettingsErrored;

    fn try_from(manager: SettingsManager) -> Result<Self, Self::Error> {
        manager.settings
    }
}

impl From<SettingsErrored> for SettingsManager {
    fn from(error: SettingsErrored) -> Self {
        Self { settings: Err(error) }
    }
}

impl SettingsManager {
    pub fn empty() -> Self {
        Self {
            settings: Ok(Empty::empty()),
        }
    }

    pub fn error(errored: &SettingsErrored) -> Self {
        Self {
            settings: Err(errored.to_owned()),
        }
    }

    pub fn setup() -> Result<Self, SettingsManagerError> {
        let config = Path::new(CONFIG_FOLDER);
        let backup = &Path::new(CONFIG_BACKUP_FOLDER);
        let default = &Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT).with_extension("json");
        let old = &Path::new(CONFIG_FOLDER).join(CONFIG_OLD).with_extension("toml");
        let local = &Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL).with_extension("json");

        Self::make_folder(&config.to_path_buf())?;

        let manager = Self::load(default, old, local)?;

        manager.save(local, &Some(backup.to_path_buf()))?;

        Ok(manager)
    }

    pub fn load(default: &PathBuf, old: &PathBuf, local: &PathBuf) -> Result<Self, SettingsManagerError> {
        Self::write_default(default)?;

        if let Some(res) = Self::import_old(old)? {
            return Ok(res);
        }

        // If no old settings, lets try the local settings.
        let local_settings = match Self::read(local) {
            Ok(settings) => Some(settings),
            Err(err) => match err {
                SettingsManagerError::FailedToReadFromFile { .. } => {
                    info!("No Configuration To Load: {err}");
                    None
                }
                err => {
                    return Err(err);
                }
            },
        };

        if let Some(res) = local_settings {
            return Ok(res);
        };

        // if nothing else, lets load the default.
        Ok(Default::default())
    }

    pub fn save(&self, to: &PathBuf, archive_folder: &Option<PathBuf>) -> Result<(), SettingsManagerError> {
        // lets backup the previous configuration, if we have any...
        let existing = get_file_at(to, OpenOptions::new().read(true)).ok();

        if let Some(existing) = existing {
            if let Some(archive_folder) = archive_folder {
                Self::archive(existing.0, &existing.1, archive_folder)?
            }
        }

        let dest = get_file_at(to, OpenOptions::new().write(true).create(true).truncate(true))
            .map_err(|err| SettingsManagerError::FailedToOpenFileForWriting { source: err })?;

        self.write(dest.0)
    }

    pub fn write_default(to: &PathBuf) -> Result<(), SettingsManagerError> {
        let dest = get_file_at(to, OpenOptions::new().write(true).create(true).truncate(true))
            .map_err(|err| SettingsManagerError::FailedToOpenFileForWriting { source: err })?;

        Self::default().write(dest.0)
    }

    pub fn read(from: &PathBuf) -> Result<Self, SettingsManagerError> {
        let source = get_file_at(from, OpenOptions::new().read(true))
            .map_err(|err| SettingsManagerError::FailedToOpenFileForReading { source: err })?;

        Self::read_json(source.0)
    }

    pub fn write(&self, writer: impl Write) -> Result<(), SettingsManagerError> {
        self.write_json(writer)
    }

    pub fn read_json<R>(mut rdr: R) -> Result<Self, SettingsManagerError>
    where
        R: Read,
    {
        let data: &mut Vec<u8> = &mut Default::default();

        rdr.read_to_end(data)
            .map_err(|err| SettingsManagerError::FailedToReadFromBuffer {
                source: IoError::from(err).into(),
            })?;

        let settings = serde_json::from_reader::<Cursor<&mut Vec<u8>>, SettingsNamespace>(Cursor::new(data)).map_err(|err| {
            SettingsManagerError::FailedToDeserializeFromJson {
                message: "(read as \"SettingsNamespace\")".to_string(),
                source: SerdeJsonError::from(err).into(),
            }
        })?;
        {
            match settings.namespace.as_str() {
                SETTINGS_NAMESPACE => serde_json::from_reader::<Cursor<&mut Vec<u8>>, Settings>(Cursor::new(data))
                    .map_err(|err| SettingsManagerError::FailedToDeserializeFromJson {
                        message: "(read as \"Settings\")".to_string(),
                        source: SerdeJsonError::from(err).into(),
                    })
                    .map(SettingsManager::from),

                SETTINGS_NAMESPACE_ERRORED => serde_json::from_reader::<Cursor<&mut Vec<u8>>, SettingsErrored>(Cursor::new(data))
                    .map_err(|err| SettingsManagerError::FailedToDeserializeFromJson {
                        message: "(read as \"SettingsErrored\")".to_string(),
                        source: SerdeJsonError::from(err).into(),
                    })
                    .map(SettingsManager::from),

                namespace => Err(SettingsManagerError::FailedToMatchNamespace {
                    namespace: namespace.to_string(),
                }),
            }
        }
    }

    pub fn write_json<W>(&self, writer: W) -> Result<(), SettingsManagerError>
    where
        W: Write,
    {
        match &self.settings {
            Ok(okay) => {
                serde_json::to_writer_pretty(writer, okay).map_err(|err| SettingsManagerError::FailedToDeserializeFromJson {
                    message: "(read as \"Settings\")".to_string(),
                    source: SerdeJsonError::from(err).into(),
                })
            }
            Err(err) => {
                serde_json::to_writer_pretty(writer, err).map_err(|err| SettingsManagerError::FailedToDeserializeFromJson {
                    message: "(read as \"SettingsErrored\")".to_string(),
                    source: SerdeJsonError::from(err).into(),
                })
            }
        }
    }

    fn backup(&self, to: &Path, folder: PathBuf) -> Result<(), SettingsManagerError> {
        let ext = match to.extension().map(|f| f.to_os_string()) {
            Some(mut ext) => {
                ext.push(".json");
                ext
            }
            None => OsString::from("json"),
        };

        let data: &mut Vec<u8> = &mut Default::default();

        self.write_json(data.by_ref())
            .map_err(|err| SettingsManagerError::FailedToWriteToFile {
                message: "(backup)".to_string(),
                to: to.to_path_buf(),

                source: err.into(),
            })?;

        Self::archive(Cursor::new(data), &to.with_extension(ext), &folder)?;
        Ok(())
    }

    fn archive(mut rdr: impl Read, from: &PathBuf, to_folder: &Path) -> Result<(), SettingsManagerError> {
        Self::make_folder(&to_folder.to_path_buf())?;

        let to_folder = to_folder
            .canonicalize()
            .map_err(|err| SettingsManagerError::FailedToResolvePath {
                at: to_folder.to_owned(),
                source: IoError::from(err).into(),
            })?;

        let mut hasher: DefaultHasher = Default::default();
        let data: &mut Vec<u8> = &mut Default::default();

        // todo: lock and stream the file instead of loading the full file into memory.
        let _size = rdr
            .read_to_end(data)
            .map_err(|err| SettingsManagerError::FailedToReadFromBuffer {
                source: IoError::from(err).into(),
            })
            .map_err(|err| SettingsManagerError::FailedToReadFromFile {
                message: "(archive, read into)".to_string(),
                from: from.to_owned(),
                source: err.into(),
            })?;

        data.hash(&mut hasher);

        let ext = match from.extension() {
            Some(ext) => {
                let mut ostr = OsString::from(format!("{}.", hasher.finish()));
                ostr.push(ext);
                ostr
            }
            None => OsString::from(hasher.finish().to_string()),
        };

        let to = to_folder.join(from.file_name().unwrap()).with_extension(ext);

        // if we do not have a backup already, lets make one.
        if to.canonicalize().is_err() {
            let mut dest = get_file_at(&to, OpenOptions::new().write(true).create_new(true))
                .map_err(|err| SettingsManagerError::FailedToCreateNewFile { source: err })?;

            dest.0
                .write_all(data)
                .map_err(|err| SettingsManagerError::FailedToWriteToFile {
                    to: dest.1,
                    message: "(archive, making backup)".to_string(),
                    source: SettingsManagerError::FailedToWriteIntoBuffer {
                        source: IoError::from(err).into(),
                    }
                    .into(),
                })?;
        };

        Ok(())
    }

    #[allow(irrefutable_let_patterns)]
    pub fn import_old(from: &PathBuf) -> Result<Option<Self>, SettingsManagerError> {
        Self::make_folder(&Path::new(CONFIG_ERROR_FOLDER).to_path_buf())?;

        let error_folder = Path::new(CONFIG_ERROR_FOLDER).join("import");
        Self::make_folder(&error_folder)?;

        let mut file = match get_file_at(from, OpenOptions::new().read(true)) {
            Ok(rdr) => rdr,
            Err(_) => {
                return Ok(None);
            }
        };

        let data: &mut Vec<u8> = &mut Default::default();

        let _size = file
            .0
            .read_to_end(data)
            .map_err(|err| SettingsManagerError::FailedToProcessOldSettings {
                source: SettingsManagerError::FailedToReadFromFile {
                    message: "(old_file)".to_string(),
                    from: file.1.to_owned(),
                    source: SettingsManagerError::FailedToReadFromBuffer {
                        source: IoError::from(err).into(),
                    }
                    .into(),
                }
                .into(),
            })?;

        let parsed = toml::de::from_slice(data.as_slice()).map_err(|err| SettingsManagerError::FailedToProcessOldSettings {
            source: SettingsManagerError::FailedToReadFromFile {
                message: "(old settings toml)".to_string(),
                from: file.1.to_owned(),
                source: SettingsManagerError::FailedToDeserializeFromToml { source: err.into() }.into(),
            }
            .into(),
        })?;

        let mut builder = TrackerSettingsBuilder::empty();

        // Attempt One
        if let test_builder = builder.to_owned().import_old(&parsed) {
            if let Err(err) = TryInto::<TrackerSettings>::try_into(test_builder.to_owned()) {
                Self::make_folder(&Path::new(CONFIG_ERROR_FOLDER).to_path_buf())?;
                Self::make_folder(&error_folder)?;
                let test = "First";

                warn!(
                    "{} import attempt failed: {}\nWith Error: {}",
                    test,
                    error_folder.to_string_lossy(),
                    err
                );

                let manager_broken = Self::error(&SettingsErrored::new(&test_builder.tracker_settings, &err));

                let ext = match file.1.extension().map(|f| f.to_os_string()) {
                    Some(mut ext) => {
                        ext.push(format!(".{}", test.to_lowercase()));
                        ext
                    }
                    None => OsString::from(test.to_lowercase()),
                };

                manager_broken.backup(&file.1.with_extension(ext), error_folder.to_owned())?;
            }

            // Replace broken with default, and remove everything else.

            builder = test_builder.tracker_settings.empty_fix().into();
        }

        // Attempt with Defaults
        if let test_builder = builder.to_owned().import_old(&parsed) {
            if let Err(err) = TryInto::<TrackerSettings>::try_into(test_builder.to_owned()) {
                Self::make_folder(&Path::new(CONFIG_ERROR_FOLDER).to_path_buf())?;
                Self::make_folder(&error_folder)?;
                let test = "Second";

                warn!(
                    "{} import attempt failed: {}\nWith Error: {}",
                    test,
                    error_folder.to_string_lossy(),
                    err
                );

                let manager_broken = Self::error(&SettingsErrored::new(&test_builder.tracker_settings, &err));

                let ext = match file.1.extension().map(|f| f.to_os_string()) {
                    Some(mut ext) => {
                        ext.push(format!(".{}", test.to_lowercase()));
                        ext
                    }
                    None => OsString::from(test.to_lowercase()),
                };

                manager_broken.backup(&file.1.with_extension(ext), error_folder.to_owned())?;
            }

            builder = test_builder.tracker_settings.clean().into();
        }

        // Final Attempt
        let settings = match TryInto::<TrackerSettings>::try_into(builder.to_owned()) {
            Ok(tracker) => Self {
                settings: Ok(tracker.into()),
            },

            Err(err) => {
                Self::make_folder(&Path::new(CONFIG_ERROR_FOLDER).to_path_buf())?;
                Self::make_folder(&error_folder)?;
                let test = "Final";

                warn!(
                    "{} import attempt failed: {}\nWith Error: {}",
                    test,
                    error_folder.to_string_lossy(),
                    err
                );

                let manager_broken = Self::error(&SettingsErrored::new(&builder.tracker_settings, &err));

                let ext = match file.1.extension().map(|f| f.to_os_string()) {
                    Some(mut ext) => {
                        ext.push(format!(".{}", test.to_lowercase()));
                        ext
                    }
                    None => OsString::from(test.to_lowercase()),
                };

                manager_broken.backup(&file.1.with_extension(ext), error_folder)?;

                return Err(SettingsManagerError::FailedToImportOldSettings {
                    from: file.1,
                    source: Box::new(err),
                });
            }
        };

        let ext = match file.1.extension() {
            Some(ext) => {
                let mut ostr = OsString::from("old.");
                ostr.push(ext);
                ostr
            }
            None => OsString::from("old"),
        };

        let backup = Path::new(CONFIG_BACKUP_FOLDER)
            .join(file.1.file_name().unwrap())
            .with_extension(ext);

        // if import was success, lets rename the extension to ".toml.old".
        match fs::rename(&file.1, &backup) {
            Ok(_) => {
                info!(
                    "\nOld Settings Was Successfully Imported!\n And moved from: \"{}\", to: \"{}\".\n",
                    file.1.display(),
                    backup.display()
                );
                Ok(Some(settings))
            }
            Err(err) => Err(SettingsManagerError::FailedToMoveFile {
                from: file.1,
                to: backup,
                source: IoError::from(err).into(),
            }),
        }
    }

    pub fn make_folder(folder: &PathBuf) -> Result<(), SettingsManagerError> {
        if let Ok(path) = folder.canonicalize() {
            if path.is_dir() {
                return Ok(());
            } else {
                return Err(SettingsManagerError::FailedToResolveDirectory { at: folder.to_owned() });
            }
        }
        match fs::create_dir(folder) {
            Ok(_) => Ok(()),
            Err(err) => Err(SettingsManagerError::FailedToPrepareDirectory {
                at: folder.to_owned(),
                source: IoError::from(err).into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::OpenOptions;
    use std::io::Seek;

    use thiserror::Error;
    use uuid::Uuid;

    use super::SettingsManager;
    use crate::helpers::get_file_at;
    use crate::settings::{Settings, SettingsErrored, TrackerSettings};

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

    #[test]
    fn it_should_write_and_read_errored_settings() {
        let path = env::temp_dir().as_path().join(format!("test_errored.{}", Uuid::new_v4()));
        let mut file_rw = get_file_at(&path, OpenOptions::new().write(true).read(true).create_new(true)).unwrap();

        #[derive(Error, Debug)]
        enum TestErrors {
            #[error("Test Error!")]
            Error,
        }

        let errored: SettingsManager = SettingsErrored::new(&TrackerSettings::default(), &TestErrors::Error).into();

        errored.write_json(std::io::Write::by_ref(&mut file_rw.0)).unwrap();
        file_rw.0.rewind().unwrap();

        let error_returned = SettingsManager::read_json(file_rw.0).unwrap();

        assert_eq!(errored, error_returned)
    }

    #[test]
    fn it_should_write_and_read_settings() {
        let path = env::temp_dir().as_path().join(format!("test_errored.{}", Uuid::new_v4()));
        let mut file_rw = get_file_at(&path, OpenOptions::new().write(true).read(true).create_new(true)).unwrap();

        let settings: SettingsManager = Settings::default().into();

        settings.write_json(std::io::Write::by_ref(&mut file_rw.0)).unwrap();
        file_rw.0.rewind().unwrap();

        let settings_returned = SettingsManager::read_json(file_rw.0).unwrap();

        assert_eq!(settings, settings_returned)
    }
}
