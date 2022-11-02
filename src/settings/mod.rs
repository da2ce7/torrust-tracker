use std::collections::btree_map::Entry::Vacant;
use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use derive_more::Display;
use serde::{Deserialize, Serialize};

use self::old_settings::DatabaseDriversOld;
use crate::databases::database::DatabaseDrivers;
use crate::errors::helpers::get_existing_file_path;
use crate::errors::{
    CommonSettingsError, DatabaseSettingsError, GlobalSettingsError, ServiceSettingsError, SettingsError, TlsSettingsError,
    TrackerSettingsError,
};
use crate::tracker::mode::TrackerMode;

pub mod manager;
pub mod old_settings;

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
    ( $(  $ctx:expr => $error:ident; $($value:ident),+ )? ) => {
        {
            $( $(
                if $ctx.$value.is_none() {
                    return Err($error::MissingRequiredField {
                        field: format!("{}", stringify!($value)),
                        data: $ctx.into(),
                    })
                };
            )+
            )?
        }
    };
}

#[macro_export]
macro_rules! check_field_is_not_empty {
    ( $( $ctx:expr => $error:ident;$($value:ident : $value_type:ty),+ )? ) => {
        {
            $( $(
                match &$ctx.$value {
                    Some(value) => {
                        if value == &<$value_type>::default(){
                        return Err($error::EmptyRequiredField {
                            field: format!("{}", stringify!($value)),
                            data: $ctx.into()});
                        }
                    },
                    None => {
                        return Err($error::MissingRequiredField {
                            field: format!("{}", stringify!($value)),
                            data: $ctx.into(),
                        });
                    },
                }
            )+
            )?
        }
    };
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
        if self.namespace != *SETTINGS_NAMESPACE {
            return Err(SettingsError::NamespaceError {
                message: format!("Actual: \"{}\", Expected: \"{}\"", self.namespace, SETTINGS_NAMESPACE),
                field: "tracker".to_string(),
            });
        }

        // Todo: Make this Check use Semantic Versioning 2.0.0
        if self.version != *SETTINGS_VERSION {
            return Err(SettingsError::VersionError {
                message: format!("Actual: \"{}\", Expected: \"{}\"", self.version, SETTINGS_NAMESPACE),
                field: "version".to_string(),
            });
        }

        if let Err(source) = self.tracker.check() {
            return Err(SettingsError::TrackerSettingsError {
                message: source.to_string(),
                field: source.get_field(),
                source,
            });
        }

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
    pub services: Option<Services>,
}

impl TrackerSettings {
    fn check(&self) -> Result<(), TrackerSettingsError> {
        check_field_is_not_none!(self.to_owned() => TrackerSettingsError;
            global, common, database, services
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
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
                field: source.get_field(),
                source,
            });
        }

        let settings = TrackerSettings {
            global: Some(GlobalSettingsBuilder::from(&self.tracker_settings.global.unwrap()).try_into()?),
            common: Some(CommonSettingsBuilder::from(&self.tracker_settings.common.unwrap()).try_into()?),
            database: Some(DatabaseSettingsBuilder::from(&self.tracker_settings.database.unwrap()).try_into()?),
            services: match self.tracker_settings.services {
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
                services: Some(ServicesBuilder::default().services),
            },
        }
    }

    pub fn with_global(self, global: &GlobalSettings) -> Self {
        Self {
            tracker_settings: TrackerSettings {
                global: Some(global.to_owned()),
                common: self.tracker_settings.common,
                database: self.tracker_settings.database,
                services: self.tracker_settings.services,
            },
        }
    }

    pub fn with_common(self, common: &CommonSettings) -> Self {
        Self {
            tracker_settings: TrackerSettings {
                global: self.tracker_settings.global,
                common: Some(common.to_owned()),
                database: self.tracker_settings.database,
                services: self.tracker_settings.services,
            },
        }
    }

    pub fn with_database(self, database: &DatabaseSettings) -> Self {
        Self {
            tracker_settings: TrackerSettings {
                global: self.tracker_settings.global,
                common: self.tracker_settings.common,
                database: Some(database.to_owned()),
                services: self.tracker_settings.services,
            },
        }
    }

    pub fn with_services(self, services: &Services) -> Self {
        Self {
            tracker_settings: TrackerSettings {
                global: self.tracker_settings.global,
                common: self.tracker_settings.common,
                database: self.tracker_settings.database,
                services: Some(services.to_owned()),
            },
        }
    }

    pub fn import_old(mut self, old_settings: &old_settings::Settings) -> Self {
        // Global
        let mut builder = match self.tracker_settings.global.as_ref() {
            Some(settings) => GlobalSettingsBuilder::from(settings),
            None => GlobalSettingsBuilder::empty(),
        };
        builder = builder.import_old(old_settings);

        self.tracker_settings.global = Some(builder.global_settings);

        // Common
        let mut builder = match self.tracker_settings.common.as_ref() {
            Some(settings) => CommonSettingsBuilder::from(settings),
            None => CommonSettingsBuilder::empty(),
        };
        builder = builder.import_old(old_settings);

        self.tracker_settings.common = Some(builder.common_settings);

        // Database
        if let Some(driver) = old_settings.db_driver {
            self.tracker_settings.database = Some(DatabaseSettingsBuilder::empty().database_settings);

            self.tracker_settings.database.as_mut().unwrap().driver = Some(match driver {
                DatabaseDriversOld::Sqlite3 => DatabaseDrivers::Sqlite3,
                DatabaseDriversOld::MySQL => DatabaseDrivers::MySQL,
            });

            if let Some(val) = old_settings.db_path.as_ref() {
                match driver {
                    DatabaseDriversOld::Sqlite3 => {
                        if let Ok(path) = PathBuf::from_str(val) {
                            self.tracker_settings.database.as_mut().unwrap().sql_lite_3_db_file_path = Some(path);
                        }
                    }
                    DatabaseDriversOld::MySQL => {
                        self.tracker_settings.database.as_mut().unwrap().my_sql_connection_url = Some(val.to_owned())
                    }
                }
            }
        }

        // Services
        let mut builder = match self.tracker_settings.services.as_ref() {
            Some(settings) => ServicesBuilder::from(settings),
            None => ServicesBuilder::empty(),
        };
        builder = builder.import_old(old_settings);

        self.tracker_settings.services = Some(builder.services);

        self
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct GlobalSettings {
    tracker_mode: Option<TrackerMode>,
    log_filter_level: Option<LogFilterLevel>,
    external_ip: Option<IpAddr>,
    on_reverse_proxy: Option<bool>,
}

impl GlobalSettings {
    fn check(&self) -> Result<(), GlobalSettingsError> {
        self.is_on_reverse_proxy()?;

        Ok(())
    }

    pub fn get_tracker_mode(&self) -> TrackerMode {
        self.tracker_mode.unwrap_or_default()
    }

    pub fn get_log_filter_level(&self) -> log::LevelFilter {
        match self.log_filter_level.unwrap_or(LogFilterLevel::Info) {
            LogFilterLevel::Off => log::LevelFilter::Off,
            LogFilterLevel::Error => log::LevelFilter::Error,
            LogFilterLevel::Warn => log::LevelFilter::Warn,
            LogFilterLevel::Info => log::LevelFilter::Info,
            LogFilterLevel::Debug => log::LevelFilter::Debug,
            LogFilterLevel::Trace => log::LevelFilter::Trace,
        }
    }

    pub fn get_external_ip_opt(&self) -> Option<IpAddr> {
        self.external_ip
    }

    pub fn is_on_reverse_proxy(&self) -> Result<bool, GlobalSettingsError> {
        check_field_is_not_none!(self.to_owned() => GlobalSettingsError; on_reverse_proxy);

        Ok(self.on_reverse_proxy.unwrap())
    }
}

#[derive(Debug)]
pub struct GlobalSettingsBuilder {
    global_settings: GlobalSettings,
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
                field: source.get_field(),
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
                tracker_mode: Some(TrackerMode::Listed),
                log_filter_level: Some(LogFilterLevel::Info),
                external_ip: None,
                on_reverse_proxy: Some(false),
            },
        }
    }

    pub fn with_external_ip(mut self, external_ip: &IpAddr) -> Self {
        self.global_settings.external_ip = Some(external_ip.to_owned());
        self
    }

    pub fn with_log_filter(mut self, log_filter: &LogFilterLevel) -> Self {
        self.global_settings.log_filter_level = Some(*log_filter);
        self
    }

    pub fn with_mode(mut self, mode: TrackerMode) -> Self {
        self.global_settings.tracker_mode = Some(mode);
        self
    }

    pub fn with_reverse_proxy(mut self, reverse_proxy: bool) -> Self {
        self.global_settings.on_reverse_proxy = Some(reverse_proxy);
        self
    }

    pub fn import_old(mut self, old_settings: &old_settings::Settings) -> Self {
        if let Some(val) = old_settings.mode.as_ref() {
            self.global_settings.tracker_mode = Some(match val {
                old_settings::TrackerModeOld::Public => TrackerMode::Public,
                old_settings::TrackerModeOld::Listed => TrackerMode::Listed,
                old_settings::TrackerModeOld::Private => TrackerMode::Private,
                old_settings::TrackerModeOld::PrivateListed => TrackerMode::PrivateListed,
            })
        }

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
            if let Ok(ip) = IpAddr::from_str(val) {
                self.global_settings.external_ip = Some(ip);
            };
        }

        if let Some(val) = old_settings.on_reverse_proxy {
            self.global_settings.on_reverse_proxy = Some(val);
        }
        self
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct CommonSettings {
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
        check_field_is_not_none!(self.to_owned() => CommonSettingsError;
            enable_tracker_usage_statistics,
            enable_persistent_statistics,
            enable_peerless_torrent_pruning
        );

        check_field_is_not_empty!(self.to_owned() => CommonSettingsError;
            announce_interval_seconds: u32,
            announce_interval_seconds_minimum: u32,
            peer_timeout_seconds_maximum: u32,
            cleanup_inactive_peers_interval_seconds: u64
        );

        Ok(())
    }
}

#[derive(Debug)]
pub struct CommonSettingsBuilder {
    common_settings: CommonSettings,
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
                field: source.get_field(),
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

    pub fn import_old(mut self, old_settings: &old_settings::Settings) -> Self {
        old_to_new!(old_settings, self.common_settings;
         announce_interval: announce_interval_seconds,
         max_peer_timeout: peer_timeout_seconds_maximum,
         tracker_usage_statistics: enable_tracker_usage_statistics,
         persistent_torrent_completed_stat: enable_persistent_statistics,
         inactive_peer_cleanup_interval: cleanup_inactive_peers_interval_seconds,
         remove_peerless_torrents: enable_peerless_torrent_pruning
        );
        self
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct DatabaseSettings {
    driver: Option<DatabaseDrivers>,
    sql_lite_3_db_file_path: Option<PathBuf>,
    my_sql_connection_url: Option<String>,
}

impl DatabaseSettings {
    fn check(&self) -> Result<(), DatabaseSettingsError> {
        match self.get_driver()? {
            DatabaseDrivers::Sqlite3 => {
                let _ = self.get_slq_lite_3_file_path()?;
            }
            DatabaseDrivers::MySQL => {
                let _ = self.get_my_sql_connection_url()?;
            }
        }

        Ok(())
    }

    pub fn get_driver(&self) -> Result<DatabaseDrivers, DatabaseSettingsError> {
        check_field_is_not_none!(self.to_owned() => DatabaseSettingsError; driver);

        Ok(self.driver.unwrap())
    }

    pub fn get_slq_lite_3_file_path(&self) -> Result<PathBuf, DatabaseSettingsError> {
        check_field_is_not_empty!(self.to_owned() => DatabaseSettingsError; sql_lite_3_db_file_path: PathBuf);

        // todo: more checks here.
        Ok(Path::new(self.sql_lite_3_db_file_path.as_ref().unwrap()).to_path_buf())
    }

    pub fn get_my_sql_connection_url(&self) -> Result<String, DatabaseSettingsError> {
        check_field_is_not_empty!(self.to_owned() => DatabaseSettingsError; my_sql_connection_url: String);

        // todo: more checks here.
        Ok(self.my_sql_connection_url.to_owned().unwrap())
    }
}

#[derive(Debug)]
pub struct DatabaseSettingsBuilder {
    database_settings: DatabaseSettings,
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
                field: source.get_field(),
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
                sql_lite_3_db_file_path: Some(PathBuf::from_str("data.db").unwrap()),
                my_sql_connection_url: None,
            },
        }
    }
}

/// Special Service Settings with the Private Access Secrets Removed
#[derive(PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct ServiceSettingClean {
    pub enabled: Option<bool>,
    pub display_name: Option<String>,
    pub service: Option<ServiceProtocol>,
    pub socket: Option<SocketAddr>,
    pub tls: Option<TlsSettings>,
    pub access_tokens: Option<BTreeMap<String, String>>,
}

impl From<&ServiceSettings> for ServiceSettingClean {
    fn from(services: &ServiceSettings) -> Self {
        Self {
            enabled: services.enabled,
            display_name: services.display_name.to_owned(),
            service: services.service,
            socket: services.socket,
            tls: services.tls.to_owned(),
            access_tokens: {
                services.access_tokens.as_ref().map(|access_tokens| {
                    access_tokens
                        .iter()
                        .map(|pair| (pair.0.to_owned(), "SECRET_REMOVED".to_string()))
                        .collect()
                })
            },
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct ServiceSettings {
    pub enabled: Option<bool>,
    pub display_name: Option<String>,
    pub service: Option<ServiceProtocol>,
    pub socket: Option<SocketAddr>,
    pub tls: Option<TlsSettings>,
    pub access_tokens: Option<BTreeMap<String, String>>,
}

pub type Services = BTreeMap<String, ServiceSettings>;

impl ServiceSettings {
    fn check(&self) -> Result<(), ServiceSettingsError> {
        check_field_is_not_none!(self => ServiceSettingsError;
        enabled, service, socket);

        check_field_is_not_empty!(self => ServiceSettingsError;
            display_name: String);

        match self.service.unwrap() {
            ServiceProtocol::Api => {
                if self.access_tokens.as_ref().filter(|f| !f.is_empty()).is_none() {
                    return Err(ServiceSettingsError::ApiRequiresAccessToken {
                        field: "access_tokens".to_string(),
                        data: self.into(),
                    });
                };
            }
            ServiceProtocol::Tls => match &self.tls {
                Some(tls) => {
                    if let Err(source) = tls.check() {
                        return Err(ServiceSettingsError::TlsSettingsError {
                            field: format!("tls.{}", source.get_field()),
                            source,
                            data: self.into(),
                        });
                    }
                }
                None => {
                    return Err(ServiceSettingsError::TlsRequiresTlsConfig {
                        field: "tls".to_string(),
                        data: self.into(),
                    });
                }
            },
            _ => {}
        }

        Ok(())
    }

    pub fn get_socket(&self) -> Result<SocketAddr, ServiceSettingsError> {
        check_field_is_not_none!(self => ServiceSettingsError; socket);

        Ok(self.socket.unwrap())
    }
}

#[derive(Debug)]
pub struct ServicesBuilder {
    services: Services,
}

impl TryInto<Services> for ServicesBuilder {
    type Error = SettingsError;

    fn try_into(self) -> Result<Services, Self::Error> {
        for service in &self.services {
            if let Err(source) = service.1.check() {
                return Err(SettingsError::ServiceSettingsError {
                    id: service.0.into(),
                    field: source.get_field(),
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

        let api = ServiceSettings {
            enabled: Some(false),
            display_name: Some("HTTP API (default)".to_string()),
            service: Some(ServiceProtocol::Api),
            socket: Some(SocketAddr::from_str("127.0.0.1:1212").unwrap()),
            tls: None,
            access_tokens: Some(access_tokens),
        };

        let udp = ServiceSettings {
            enabled: Some(false),
            display_name: Some("UDP (default)".to_string()),
            service: Some(ServiceProtocol::Udp),
            socket: Some(SocketAddr::from_str("0.0.0.0:6969").unwrap()),
            tls: None,
            access_tokens: None,
        };

        let http = ServiceSettings {
            enabled: Some(false),
            display_name: Some("HTTP (default)".to_string()),
            service: Some(ServiceProtocol::Http),
            socket: Some(SocketAddr::from_str("0.0.0.0:6969").unwrap()),
            tls: None,
            access_tokens: None,
        };

        let tls = ServiceSettings {
            enabled: Some(false),
            display_name: Some("TLS (default)".to_string()),
            service: Some(ServiceProtocol::Http),
            socket: Some(SocketAddr::from_str("0.0.0.0:6969").unwrap()),
            tls: Some(TlsSettings {
                certificate_file_path: Some(PathBuf::default()),
                key_file_path: Some(PathBuf::default()),
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

    /// will remove the services that failed the configuration check, returns removed services.
    fn remove_check_fail(&mut self) -> Services {
        let removed = self
            .services
            .iter()
            .filter(|service| service.1.check().is_err())
            .map(|pair| (pair.0.to_owned(), pair.1.to_owned()))
            .collect();

        self.services = self
            .services
            .iter()
            .filter(|service| service.1.check().is_ok())
            .map(|pair| (pair.0.to_owned(), pair.1.to_owned()))
            .collect();

        removed
    }

    pub fn import_old(mut self, old_settings: &old_settings::Settings) -> Self {
        let existing_service_map = self.services.clone();
        let existing_services: HashSet<&ServiceSettings, RandomState> = HashSet::from_iter(existing_service_map.values());

        let mut new_values: HashSet<(ServiceSettings, String)> = HashSet::new();

        if let Some(api) = old_settings.http_api.as_ref() {
            new_values.insert((
                ServiceSettings {
                    enabled: api.enabled,
                    display_name: Some("HTTP API (imported)".to_string()),
                    service: Some(ServiceProtocol::Api),
                    socket: api
                        .bind_address
                        .as_ref()
                        .map(|socket| SocketAddr::from_str(socket.as_str()).ok())
                        .unwrap_or(None),
                    tls: None,
                    access_tokens: api.access_tokens.clone(),
                },
                "api_imported".to_string(),
            ));
        };

        if let Some(udp) = old_settings.udp_trackers.as_ref() {
            for service in udp {
                new_values.insert((
                    ServiceSettings {
                        enabled: service.enabled,
                        display_name: Some("UDP Service (imported)".to_string()),
                        service: Some(ServiceProtocol::Udp),
                        socket: service
                            .bind_address
                            .as_ref()
                            .map(|socket| SocketAddr::from_str(socket.as_str()).ok())
                            .unwrap_or(None),
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
                        ServiceSettings {
                            enabled: service.enabled,
                            display_name: Some("HTTP Service(imported)".to_string()),
                            service: Some(ServiceProtocol::Http),
                            socket: service
                                .bind_address
                                .as_ref()
                                .map(|socket| SocketAddr::from_str(socket.as_str()).ok())
                                .unwrap_or(None),
                            tls: None,
                            access_tokens: None,
                        },
                        "http_imported".to_string(),
                    )
                } else {
                    (
                        ServiceSettings {
                            enabled: service.enabled,
                            display_name: Some("TLS Service (imported)".to_string()),
                            service: Some(ServiceProtocol::Tls),
                            socket: service
                                .bind_address
                                .as_ref()
                                .map(|socket| SocketAddr::from_str(socket.as_str()).ok())
                                .unwrap_or(None),
                            tls: Some(TlsSettings {
                                certificate_file_path: {
                                    service
                                        .ssl_cert_path
                                        .as_ref()
                                        .map(|path| PathBuf::from_str(path.as_str()).ok())
                                        .unwrap_or(None)
                                },
                                key_file_path: {
                                    service
                                        .ssl_key_path
                                        .as_ref()
                                        .map(|path| PathBuf::from_str(path.as_str()).ok())
                                        .unwrap_or(None)
                                },
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
        self
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct TlsSettings {
    pub certificate_file_path: Option<PathBuf>,
    pub key_file_path: Option<PathBuf>,
}

impl TlsSettings {
    fn check(&self) -> Result<(), TlsSettingsError> {
        check_field_is_not_empty!(self.to_owned() => TlsSettingsError;
            certificate_file_path: PathBuf,
            key_file_path: PathBuf);

        Ok(())
    }

    pub fn get_certificate_file_path(&self) -> Result<PathBuf, TlsSettingsError> {
        check_field_is_not_empty!(self.to_owned() => TlsSettingsError;
            certificate_file_path: PathBuf);

        match get_existing_file_path(self.certificate_file_path.as_ref().unwrap()) {
            Ok(path) => Ok(path),
            Err(error) => Err(TlsSettingsError::BadCertificateFilePath {
                field: "certificate_file_path".to_string(),
                source: error,
            }),
        }
    }

    pub fn get_key_file_path(&self) -> Result<PathBuf, TlsSettingsError> {
        check_field_is_not_empty!(self.to_owned() => TlsSettingsError;
            key_file_path: PathBuf);

        match get_existing_file_path(self.key_file_path.as_ref().unwrap()) {
            Ok(path) => Ok(path),
            Err(error) => Err(TlsSettingsError::BadKeyFilePath {
                field: "key_file_path".to_string(),
                source: error,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LogFilterLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for LogFilterLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", *self).to_lowercase())
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone, Hash, Display)]
#[serde(rename_all = "snake_case")]
pub enum ServiceProtocol {
    Udp,
    Http,
    Tls,
    Api,
}
