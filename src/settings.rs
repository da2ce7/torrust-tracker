use std::collections::btree_map::Entry::Vacant;
use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use config::{Config, ConfigError, File};
use log::info;
use serde::{Deserialize, Serialize};

use crate::config_const::{CONFIG_DEFAULT, CONFIG_FOLDER, CONFIG_LOCAL, CONFIG_OLD_LOCAL, CONFIG_OVERRIDE};
use crate::databases::database::DatabaseDrivers;
use crate::errors::{
    CommonSettingsError, DatabaseSettingsError, GlobalSettingsError, ServiceSettingsError, SettingsError, TrackerSettingsError,
};
use crate::mode::TrackerMode;

#[macro_export]
macro_rules! old_to_new {
    ( $( $base_old:expr, $base_new:expr;  $($old:ident: $new:ident),+ )? ) => {
        {
            $( $(
                if let Some(val) = $base_old.$old{
                    $base_new.$new = Some(val)
                }
            )+
        )?
        }
    };
}

#[macro_export]
macro_rules! check_field_is_not_none {
    ( $(  $ctx:expr => $error:ident; $($value:ident),+, )? ) => {
        {
            $( $(
                if $ctx.$value.is_none() {
                    return Err($error::MissingRequiredField {
                        field: format!("{}", stringify!($value)),
                        data: $ctx.to_owned(),
                    })
                };
            )+
            )?
        }
    };
}

#[macro_export]
macro_rules! check_field_is_not_empty {
    ( $( $ctx:expr => $error:ident;$($value:ident : $value_type:ty),+, )? ) => {
        {
            $( $(
                match $ctx.$value {
                    Some(value) => {
                        if value == <$value_type>::default(){
                        return Err($error::EmptyRequiredField {
                            field: format!("{}", stringify!($value)),
                            data: $ctx.to_owned()});
                        }
                    },
                    None => {
                        return Err($error::MissingRequiredField {
                            field: format!("{}", stringify!($value)),
                            data: $ctx.to_owned(),
                        });
                    },
                }
            )+
            )?
        }
    };
}

pub mod old_settings {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    use serde_with::serde_as;

    use crate::databases::database::DatabaseDrivers;
    use crate::mode::TrackerMode;

    #[serde_as]
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct UdpTrackerConfig {
        pub display_name: Option<String>,
        pub enabled: Option<bool>,
        pub bind_address: Option<String>,
    }

    #[serde_as]
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
    pub struct HttpTrackerConfig {
        pub display_name: Option<String>,
        pub enabled: Option<bool>,
        pub bind_address: Option<String>,
        pub ssl_enabled: Option<bool>,
        pub ssl_cert_path: Option<String>,
        pub ssl_key_path: Option<String>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub struct HttpApiConfig {
        pub enabled: Option<bool>,
        pub bind_address: Option<String>,
        pub access_tokens: Option<BTreeMap<String, String>>,
    }

    #[serde_as]
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
    pub struct Settings {
        pub log_level: Option<String>,
        pub mode: Option<TrackerMode>,
        pub db_driver: Option<DatabaseDrivers>,
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
}

const SETTINGS_NAMESPACE: &str = "org.torrust.tracker";
const SETTINGS_VERSION: &str = "1.0.0";

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Settings {
    pub namespace: String,
    pub version: String,
    pub tracker: TrackerSettings,
}

impl Settings {
    pub fn check(&self) -> Result<(), SettingsError> {
        if self.namespace != SETTINGS_NAMESPACE.to_string() {
            return Err(SettingsError::NamespaceError {
                message: format!("Actual: \"{}\", Expected: \"{}\"", self.namespace, SETTINGS_NAMESPACE),
            });
        }

        // Todo: Make this Check use Semantic Versioning 2.0.0
        if self.version != SETTINGS_VERSION.to_string() {
            return Err(SettingsError::VersionError {
                message: format!("Actual: \"{}\", Expected: \"{}\"", self.namespace, SETTINGS_NAMESPACE),
            });
        }

        if match Err(source) = self.tracker.check()

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            namespace: SETTINGS_NAMESPACE.to_string(),
            version: SETTINGS_VERSION.to_string(),
            tracker: Default::default(),
        }
    }
}

impl From<&TrackerSettings> for Settings {
    fn from(tracker: &TrackerSettings) -> Self {
        Self {
            namespace: SETTINGS_NAMESPACE.to_string(),
            version: SETTINGS_VERSION.to_string(),
            tracker: tracker.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct TrackerSettings {
    pub global: Option<GlobalSettings>,
    pub common: Option<CommonSettings>,
    pub database: Option<DatabaseSettings>,
    pub service: Option<BTreeMap<String, ServiceSetting>>,
}

impl TrackerSettings {
    fn check(&self) -> Result<(), TrackerSettingsError> {
        check_field_is_not_none!(self => TrackerSettingsError;
            global, common, database, service,
        );
        Ok(())
    }
}

#[derive(Debug)]
pub struct TrackerSettingsBuilder {
    tracker_settings: TrackerSettings,
}

impl From<&TrackerSettings> for TrackerSettingsBuilder {
    fn from(tracker_settings: &TrackerSettings) -> Self {
        Self {
            tracker_settings: tracker_settings.clone(),
        }
    }
}

impl TryInto<TrackerSettings> for TrackerSettingsBuilder {
    type Error = SettingsError;

    fn try_into(self) -> Result<TrackerSettings, Self::Error> {
        if let Err(source) = self.tracker_settings.check() {
            return Err(SettingsError::TrackerSettingsError {
                message: "".to_string(),
                source,
            });
        }

        let settings = TrackerSettings {
            global: Some(GlobalSettingsBuilder::from(&self.tracker_settings.global.unwrap()).try_into()?),
            common: Some(CommonSettingsBuilder::from(&self.tracker_settings.common.unwrap()).try_into()?),
            database: Some(DatabaseSettingsBuilder::from(&self.tracker_settings.database.unwrap()).try_into()?),
            service: match self.tracker_settings.service {
                Some(services) => Some(ServicesBuilder::from(&services).try_into()?),
                None => None,
            },
        };

        Ok(settings)
    }
}

impl TrackerSettingsBuilder {
    pub fn empty() -> TrackerSettingsBuilder {
        Self {
            tracker_settings: TrackerSettings::default(),
        }
    }

    pub fn default() -> TrackerSettingsBuilder {
        Self {
            tracker_settings: TrackerSettings {
                global: Some(GlobalSettingsBuilder::default().global_settings),
                common: Some(CommonSettingsBuilder::default().common_settings),
                database: Some(DatabaseSettingsBuilder::default().database_settings),
                service: Some(ServicesBuilder::default().services),
            },
        }
    }

    pub fn import_old(&mut self, old_settings: &old_settings::Settings) {
        // Global
        let mut builder = match self.tracker_settings.global.as_ref() {
            Some(settings) => GlobalSettingsBuilder::from(settings),
            None => GlobalSettingsBuilder::empty(),
        };
        builder.import_old(old_settings);

        self.tracker_settings.global = Some(builder.global_settings);

        // Common
        let mut builder = match self.tracker_settings.common.as_ref() {
            Some(settings) => CommonSettingsBuilder::from(settings),
            None => CommonSettingsBuilder::empty(),
        };
        builder.import_old(old_settings);

        self.tracker_settings.common = Some(builder.common_settings);

        // Database
        if old_settings.db_driver.is_some() | old_settings.db_path.is_some() {
            if self.tracker_settings.database.is_none() {
                self.tracker_settings.database = Some(DatabaseSettingsBuilder::empty().database_settings);
            }

            if let Some(val) = old_settings.db_driver.as_ref() {
                self.tracker_settings.database.as_mut().unwrap().driver = Some(*val)
            }

            if let Some(val) = old_settings.db_path.as_ref() {
                self.tracker_settings.database.as_mut().unwrap().path = Some(val.clone())
            }
        }

        // Services
        let mut builder = match self.tracker_settings.service.as_ref() {
            Some(settings) => ServicesBuilder::from(settings),
            None => ServicesBuilder::empty(),
        };
        builder.import_old(old_settings);

        self.tracker_settings.service = Some(builder.services);
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct GlobalSettings {
    pub log_filter_level: Option<LogFilterLevel>,
    pub external_ip: Option<String>,
    pub is_on_reverse_proxy: Option<bool>,
}

impl GlobalSettings {
    fn check(&self) -> Result<(), GlobalSettingsError> {
        check_field_is_not_none!(self => GlobalSettingsError;
            log_filter_level, is_on_reverse_proxy,
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct GlobalSettingsBuilder {
    pub global_settings: GlobalSettings,
}

impl From<&GlobalSettings> for GlobalSettingsBuilder {
    fn from(global_settings: &GlobalSettings) -> Self {
        Self {
            global_settings: global_settings.clone(),
        }
    }
}

impl TryInto<GlobalSettings> for GlobalSettingsBuilder {
    type Error = SettingsError;

    fn try_into(self) -> Result<GlobalSettings, Self::Error> {
        match self.global_settings.check() {
            Ok(_) => Ok(self.global_settings),
            Err(source) => Err(SettingsError::GlobalSettingsError {
                message: "".to_string(),
                source,
            }),
        }
    }
}

impl GlobalSettingsBuilder {
    pub fn empty() -> GlobalSettingsBuilder {
        Self {
            global_settings: GlobalSettings::default(),
        }
    }

    pub fn default() -> GlobalSettingsBuilder {
        Self {
            global_settings: GlobalSettings {
                log_filter_level: Some(LogFilterLevel::Info),
                external_ip: Some("".to_string()),
                is_on_reverse_proxy: Some(false),
            },
        }
    }

    pub fn import_old(&mut self, old_settings: &old_settings::Settings) {
        if let Some(val) = old_settings.log_level.as_ref() {
            self.global_settings.log_filter_level = match val.to_lowercase().as_str() {
                "off" => Some(LogFilterLevel::Off),
                "trace" => Some(LogFilterLevel::Trace),
                "debug" => Some(LogFilterLevel::Debug),
                "info" => Some(LogFilterLevel::Info),
                "warn" => Some(LogFilterLevel::Warn),
                "error" => Some(LogFilterLevel::Error),
                _ => None,
            }
        }

        if let Some(val) = old_settings.external_ip.as_ref() {
            self.global_settings.external_ip = Some(val.clone());
        }

        if let Some(val) = old_settings.on_reverse_proxy {
            self.global_settings.is_on_reverse_proxy = Some(val);
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct CommonSettings {
    pub tracker_mode: Option<TrackerMode>,
    pub announce_interval_seconds: Option<u32>,
    pub announce_interval_seconds_minimum: Option<u32>,
    pub peer_timeout_seconds_maximum: Option<u32>,
    pub enable_tracker_usage_statistics: Option<bool>,
    pub enable_persistent_statistics: Option<bool>,
    pub cleanup_inactive_peers_interval_seconds: Option<u64>,
    pub enable_peerless_torrent_pruning: Option<bool>,
}

impl CommonSettings {
    fn check(&self) -> Result<(), CommonSettingsError> {
        check_field_is_not_none!(self => CommonSettingsError;
            tracker_mode,
            enable_tracker_usage_statistics,
            enable_persistent_statistics,
            enable_peerless_torrent_pruning,
        );

        check_field_is_not_empty!(self => CommonSettingsError;
            announce_interval_seconds: u32,
            announce_interval_seconds_minimum: u32,
            peer_timeout_seconds_maximum: u32,
            cleanup_inactive_peers_interval_seconds: u64,
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct CommonSettingsBuilder {
    pub common_settings: CommonSettings,
}

impl From<&CommonSettings> for CommonSettingsBuilder {
    fn from(common_settings: &CommonSettings) -> Self {
        Self {
            common_settings: common_settings.clone(),
        }
    }
}

impl TryInto<CommonSettings> for CommonSettingsBuilder {
    type Error = SettingsError;

    fn try_into(self) -> Result<CommonSettings, Self::Error> {
        match self.common_settings.check() {
            Ok(_) => Ok(self.common_settings),
            Err(source) => Err(SettingsError::CommonSettingsError {
                message: source.to_string(),
                source,
            }),
        }
    }
}

impl CommonSettingsBuilder {
    pub fn empty() -> CommonSettingsBuilder {
        Self {
            common_settings: CommonSettings::default(),
        }
    }

    pub fn default() -> CommonSettingsBuilder {
        Self {
            common_settings: CommonSettings {
                tracker_mode: Some(TrackerMode::Listed),
                announce_interval_seconds: Some(120),
                announce_interval_seconds_minimum: Some(120),
                peer_timeout_seconds_maximum: Some(900),
                enable_tracker_usage_statistics: Some(true),
                enable_persistent_statistics: Some(false),
                cleanup_inactive_peers_interval_seconds: Some(600),
                enable_peerless_torrent_pruning: Some(false),
            },
        }
    }

    pub fn import_old(&mut self, old_settings: &old_settings::Settings) {
        old_to_new!(old_settings, self.common_settings;
         mode: tracker_mode,
         announce_interval: announce_interval_seconds,
         max_peer_timeout: peer_timeout_seconds_maximum,
         tracker_usage_statistics: enable_tracker_usage_statistics,
         persistent_torrent_completed_stat: enable_persistent_statistics,
         inactive_peer_cleanup_interval: cleanup_inactive_peers_interval_seconds,
         remove_peerless_torrents: enable_peerless_torrent_pruning
        );
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct DatabaseSettings {
    pub driver: Option<DatabaseDrivers>,
    pub path: Option<String>,
}

impl DatabaseSettings {
    fn check(&self) -> Result<(), DatabaseSettingsError> {
        check_field_is_not_none!(self => DatabaseSettingsError;
            driver,);

        check_field_is_not_empty!(self => DatabaseSettingsError;
            path: String,);

        Ok(())
    }
}

#[derive(Debug)]
pub struct DatabaseSettingsBuilder {
    pub database_settings: DatabaseSettings,
}

impl From<&DatabaseSettings> for DatabaseSettingsBuilder {
    fn from(database_settings: &DatabaseSettings) -> Self {
        Self {
            database_settings: database_settings.clone(),
        }
    }
}

impl TryInto<DatabaseSettings> for DatabaseSettingsBuilder {
    type Error = SettingsError;

    fn try_into(self) -> Result<DatabaseSettings, Self::Error> {
        match self.database_settings.check() {
            Ok(_) => Ok(self.database_settings),
            Err(source) => Err(SettingsError::DatabaseSettingsError {
                message: source.to_string(),
                source,
            }),
        }
    }
}

impl DatabaseSettingsBuilder {
    pub fn empty() -> DatabaseSettingsBuilder {
        Self {
            database_settings: DatabaseSettings::default(),
        }
    }
    pub fn default() -> DatabaseSettingsBuilder {
        Self {
            database_settings: DatabaseSettings {
                driver: Some(DatabaseDrivers::Sqlite3),
                path: Some("data.db".to_string()),
            },
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct ServiceSetting {
    pub enabled: Option<bool>,
    pub display_name: Option<String>,
    pub service: Option<ServiceProtocol>,
    pub socket: Option<String>,
    pub tls: Option<TlsSettings>,
    pub access_tokens: Option<BTreeMap<String, String>>,
}

pub type Services = BTreeMap<String, ServiceSetting>;

impl ServiceSetting {
    fn check(&self) -> Result<(), ServiceSettingsError> {
        check_field_is_not_none!(self => ServiceSettingsError;
        enabled,service,);

        check_field_is_not_empty!(self => ServiceSettingsError;
            display_name: String,socket: String,);

        Ok(())
    }
}

#[derive(Debug)]
pub struct ServicesBuilder {
    pub services: Services,
}

impl TryInto<Services> for ServicesBuilder {
    type Error = SettingsError;

    fn try_into(self) -> Result<Services, Self::Error> {
        for service in &self.services {
            if let Err(source) = service.1.check() {
                return Err(SettingsError::ServiceSettingsError {
                    id: service.0.into(),
                    message: source.to_string(),
                    source,
                });
            }
        }

        Ok(self.services)
    }
}

impl From<&Services> for ServicesBuilder {
    fn from(services: &Services) -> Self {
        Self {
            services: services.clone(),
        }
    }
}

impl ServicesBuilder {
    pub fn empty() -> ServicesBuilder {
        Self {
            services: BTreeMap::new(),
        }
    }
    pub fn default() -> ServicesBuilder {
        let mut access_tokens = BTreeMap::new();
        access_tokens.insert("admin".to_string(), "password".to_string());

        let api = ServiceSetting {
            enabled: Some(false),
            display_name: Some("HTTP API (default)".to_string()),
            service: Some(ServiceProtocol::API),
            socket: Some("127.0.0.1:1212".to_string()),
            tls: None,
            access_tokens: Some(access_tokens),
        };

        let udp = ServiceSetting {
            enabled: Some(false),
            display_name: Some("UDP (default)".to_string()),
            service: Some(ServiceProtocol::UDP),
            socket: Some("0.0.0.0:6969".to_string()),
            tls: None,
            access_tokens: None,
        };

        let http = ServiceSetting {
            enabled: Some(false),
            display_name: Some("HTTP (default)".to_string()),
            service: Some(ServiceProtocol::HTTP),
            socket: Some("0.0.0.0:6969".to_string()),
            tls: None,
            access_tokens: None,
        };

        let tls = ServiceSetting {
            enabled: Some(false),
            display_name: Some("TLS (default)".to_string()),
            service: Some(ServiceProtocol::HTTP),
            socket: Some("0.0.0.0:6969".to_string()),
            tls: Some(TlsSettings {
                certificate_file_path: Some("".to_string()),
                key_file_path: Some("".to_string()),
            }),
            access_tokens: None,
        };

        let mut services = BTreeMap::new();

        services.insert("default_api".to_string(), api);
        services.insert("default_udp".to_string(), udp);
        services.insert("default_http".to_string(), http);
        services.insert("default_tls".to_string(), tls);

        Self { services }
    }

    pub fn import_old(&mut self, old_settings: &old_settings::Settings) {
        let existing_service_map = self.services.clone();
        let existing_services: HashSet<&ServiceSetting, RandomState> = HashSet::from_iter(existing_service_map.values());

        let mut new_values: HashSet<(ServiceSetting, String)> = HashSet::new();

        if let Some(api) = old_settings.http_api.as_ref() {
            new_values.insert((
                ServiceSetting {
                    enabled: api.enabled,
                    display_name: Some("HTTP API (imported)".to_string()),
                    service: Some(ServiceProtocol::API),
                    socket: api.bind_address.clone(),
                    tls: None,
                    access_tokens: api.access_tokens.clone(),
                },
                "api_imported".to_string(),
            ));
        };

        if let Some(udp) = old_settings.udp_trackers.as_ref() {
            for service in udp {
                new_values.insert((
                    ServiceSetting {
                        enabled: service.enabled,
                        display_name: Some("UDP Service (imported)".to_string()),
                        service: Some(ServiceProtocol::UDP),
                        socket: service.bind_address.clone(),
                        tls: None,
                        access_tokens: None,
                    },
                    "udp_imported".to_string(),
                ));
            }
        };

        if let Some(http_or_tls) = old_settings.http_trackers.as_ref() {
            for service in http_or_tls {
                new_values.insert(if service.ssl_enabled.unwrap_or_default() {
                    (
                        ServiceSetting {
                            enabled: service.enabled,
                            display_name: Some("HTTP Service(imported)".to_string()),
                            service: Some(ServiceProtocol::HTTP),
                            socket: service.bind_address.clone(),
                            tls: None,
                            access_tokens: None,
                        },
                        "http_imported".to_string(),
                    )
                } else {
                    (
                        ServiceSetting {
                            enabled: service.enabled,
                            display_name: Some("TLS Service (imported)".to_string()),
                            service: Some(ServiceProtocol::TLS),
                            socket: service.bind_address.clone(),
                            tls: Some(TlsSettings {
                                certificate_file_path: service.ssl_cert_path.clone(),
                                key_file_path: service.ssl_key_path.clone(),
                            }),
                            access_tokens: None,
                        },
                        "tls_imported".to_string(),
                    )
                });
            }
        };

        for (value, name) in new_values {
            // Lets not import something we already have...
            if !existing_services.contains(&value) {
                for count in 0.. {
                    let key = format!("{name}_{count}");
                    if let Vacant(e) = self.services.entry(key) {
                        e.insert(value.clone());
                        break;
                    } else {
                        continue;
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct TlsSettings {
    pub certificate_file_path: Option<String>,
    pub key_file_path: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum LogFilterLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone, Hash)]
pub enum ServiceProtocol {
    UDP,
    HTTP,
    TLS,
    API,
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

impl old_settings::Settings {
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
        let settings = &mut self.clone();

        let toml_string = match toml::to_string(settings) {
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
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::path::Path;
    use std::{env, fs};

    use config::Config;
    use uuid::Uuid;

    use super::{TrackerSettings, TrackerSettingsBuilder};
    use crate::config_const::{CONFIG_DEFAULT, CONFIG_FOLDER, CONFIG_LOCAL};
    use crate::settings::old_settings::Settings;

    #[test]
    fn write_test_configuration() {
        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_DEFAULT);
        let json_string = serde_json::to_string_pretty(&TrackerSettingsBuilder::default().tracker_settings).unwrap();

        fs::write(local_source.with_extension("new.json"), json_string).unwrap()
    }

    #[test]
    fn default_settings_should_be_complete() {
        let default_settings_builder = TrackerSettingsBuilder::default();

        let _: TrackerSettings = default_settings_builder.try_into().unwrap();
    }

    #[test]
    fn default_config_should_roundtrip() {
        let term_dir = env::temp_dir();
        let default_settings = &TrackerSettingsBuilder::default().tracker_settings;
        let settings_json = serde_json::to_string_pretty(default_settings).unwrap();

        let mut hasher = DefaultHasher::new();
        settings_json.hash(&mut hasher);
        let temp_file_path = &term_dir.join(format!("test-{}", hasher.finish())).with_extension("json");

        fs::write(temp_file_path, settings_json).unwrap();

        let settings2: TrackerSettings = Config::builder()
            .add_source(config::File::from(temp_file_path.as_path()))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        assert_eq!(default_settings, &settings2);
    }

    #[test]
    fn load_old_settings() {
        let old_settings = Settings::default().unwrap();

        let mut new_settings_builder = TrackerSettingsBuilder::from(&TrackerSettings::default());

        new_settings_builder.import_old(&old_settings);

        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL);
        let json_string = serde_json::to_string_pretty(&new_settings_builder.tracker_settings).unwrap();

        fs::write(local_source.with_extension("new.json"), json_string).unwrap()
    }

    #[test]
    fn load_old_settings_into_default() {
        let old_settings = Settings::default().unwrap();

        let mut new_settings_builder = TrackerSettingsBuilder::default();

        new_settings_builder.import_old(&old_settings);

        let local_source = Path::new(CONFIG_FOLDER).join(CONFIG_LOCAL);
        let json_string = serde_json::to_string_pretty(&new_settings_builder.tracker_settings).unwrap();

        fs::write(local_source.with_extension("default.json"), json_string).unwrap()
    }

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
