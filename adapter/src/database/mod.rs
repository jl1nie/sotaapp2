pub mod model;

#[cfg(not(feature = "sqlite"))]
pub mod connect {
    use anyhow::Result;
    use common::config::AppConfig;
    use sqlx::postgres::PgPool;

    #[derive(Clone)]
    pub struct ConnectionPool(PgPool);
    #[cfg(not(feature = "sqlite"))]
    impl ConnectionPool {
        pub fn new(pool: PgPool) -> Self {
            Self(pool)
        }

        pub fn inner_ref(&self) -> &PgPool {
            &self.0
        }
    }

    #[cfg(not(feature = "sqlite"))]
    pub fn connect_database_with(cfg: &AppConfig) -> Result<ConnectionPool> {
        let pool = ConnectionPool(PgPool::connect_lazy(&cfg.database)?);
        Ok(pool)
    }
}
#[cfg(feature = "sqlite")]
pub mod connect {
    use anyhow::Result;
    use common::config::AppConfig;
    use sqlx::sqlite::SqlitePool;

    #[derive(Clone)]
    pub struct ConnectionPool(SqlitePool);

    impl ConnectionPool {
        pub fn new(pool: SqlitePool) -> Self {
            Self(pool)
        }

        pub fn inner_ref(&self) -> &SqlitePool {
            &self.0
        }
    }
    pub fn connect_database_with(cfg: &AppConfig) -> Result<ConnectionPool> {
        let pool = ConnectionPool(SqlitePool::connect_lazy(&cfg.database)?);
        Ok(pool)
    }
}
