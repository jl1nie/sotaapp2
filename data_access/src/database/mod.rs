use anyhow::Result;
use common::config::DatabaseConfig;
use sqlx::sqlite::SqlitePool;

#[derive(Clone)]
pub struct ConnectionPool(SqlitePool);

impl ConnectionPool {
    pub fn inner_ref(&self) -> &SqlitePool {
        &self.0
    }
}

pub fn connect_database_with(cfg: &DatabaseConfig) -> Result<ConnectionPool> {
    let pool = ConnectionPool(SqlitePool::connect_lazy(&cfg.url)?);
    Ok(pool)
}
