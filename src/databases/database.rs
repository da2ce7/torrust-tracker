use async_trait::async_trait;
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::databases::mysql::MysqlDatabase;
use crate::databases::sqlite::SqliteDatabase;
use crate::protocol::common::InfoHash;
use crate::settings::DatabaseSettings;
use crate::tracker::key::AuthKey;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Copy, Clone, Hash, Display)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseDrivers {
    Sqlite3,
    MySQL,
}

impl Default for DatabaseDrivers {
    fn default() -> Self {
        DatabaseDrivers::Sqlite3
    }
}

pub fn connect_database(database_settings: &DatabaseSettings) -> Result<Box<dyn Database>, r2d2::Error> {
    let database: Box<dyn Database> = match database_settings.get_driver().unwrap() {
        // todo: handel errors
        DatabaseDrivers::Sqlite3 => Box::new(SqliteDatabase::new(&database_settings.try_into().unwrap())?),
        DatabaseDrivers::MySQL => Box::new(MysqlDatabase::new(&database_settings.try_into().unwrap())?),
    };

    database.create_database_tables().expect("Could not create database tables.");

    Ok(database)
}

#[async_trait]
pub trait Database: Sync + Send {
    fn create_database_tables(&self) -> Result<(), Error>;

    async fn load_persistent_torrents(&self) -> Result<Vec<(InfoHash, u32)>, Error>;

    async fn load_keys(&self) -> Result<Vec<AuthKey>, Error>;

    async fn load_whitelist(&self) -> Result<Vec<InfoHash>, Error>;

    async fn save_persistent_torrent(&self, info_hash: &InfoHash, completed: u32) -> Result<(), Error>;

    async fn get_info_hash_from_whitelist(&self, info_hash: &str) -> Result<InfoHash, Error>;

    async fn add_info_hash_to_whitelist(&self, info_hash: InfoHash) -> Result<usize, Error>;

    async fn remove_info_hash_from_whitelist(&self, info_hash: InfoHash) -> Result<usize, Error>;

    async fn get_key_from_keys(&self, key: &str) -> Result<AuthKey, Error>;

    async fn add_key_to_keys(&self, auth_key: &AuthKey) -> Result<usize, Error>;

    async fn remove_key_from_keys(&self, key: &str) -> Result<usize, Error>;
}

#[derive(Debug, Display, PartialEq, Error)]
#[allow(dead_code)]
pub enum Error {
    #[display(fmt = "Query returned no rows.")]
    QueryReturnedNoRows,
    #[display(fmt = "Invalid query.")]
    InvalidQuery,
    #[display(fmt = "Database error.")]
    DatabaseError,
}

impl From<r2d2_sqlite::rusqlite::Error> for Error {
    fn from(e: r2d2_sqlite::rusqlite::Error) -> Self {
        match e {
            r2d2_sqlite::rusqlite::Error::QueryReturnedNoRows => Error::QueryReturnedNoRows,
            _ => Error::InvalidQuery,
        }
    }
}
