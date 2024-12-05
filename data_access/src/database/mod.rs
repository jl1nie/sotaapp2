use anyhow::Result;
use common::config::DatabaseConfig;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct ConnectionPool(PgPool);

impl ConnectionPool {
    pub fn new(pool: PgPool) -> Self {
        Self(pool)
    }

    pub fn inner_ref(&self) -> &PgPool {
        &self.0
    }
}

pub fn connect_database_with(cfg: &DatabaseConfig) -> Result<ConnectionPool> {
    let pool = ConnectionPool(PgPool::connect_lazy(&cfg.url)?);
    Ok(pool)
}
