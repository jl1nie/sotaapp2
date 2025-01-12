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
    pub async fn connect_database_with(cfg: &AppConfig) -> Result<ConnectionPool> {
        let pool = ConnectionPool(PgPool::connect_lazy(&cfg.database)?);
        Ok(pool)
    }
}
#[cfg(feature = "sqlite")]
pub mod connect {
    use anyhow::Result;
    use common::config::AppConfig;
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::SqlitePool;
    use std::{fs, path::Path};

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

    pub async fn connect_database_with(cfg: &AppConfig) -> Result<ConnectionPool> {
        let m = Migrator::new(std::path::Path::new(&cfg.migration_path)).await?;
        let dbname = cfg.database.replace("sqlite:", "");
        let database_path = Path::new(&dbname);
        let pool = ConnectionPool(SqlitePool::connect_lazy(&cfg.database)?);

        if fs::metadata(&database_path).is_err() {
            tracing::warn!(
                "Database file {} not found. Running migrations...",
                database_path.display()
            );
            let _file = fs::File::create(&database_path)?;
            m.run(pool.inner_ref()).await?
        } else {
            tracing::info!("Database file {} found.", database_path.display());
        }
        Ok(pool)
    }
}
