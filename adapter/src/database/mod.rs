pub mod implement;
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
        let mut force_migrate = false;

        if cfg.init_database {
            tracing::info!("Drop database file {}", database_path.display());
            if fs::remove_file(database_path).is_err() {
                tracing::warn!("Failed to remove database file {}", database_path.display());
            }
        }

        if fs::metadata(database_path).is_err() {
            tracing::warn!(
                "Database file {} not found. Create it.",
                database_path.display()
            );
            let _file = fs::File::create(database_path)?;
            force_migrate = true;
        };

        if cfg.run_migration || force_migrate {
            tracing::info!("Running migrations...");
            m.run(pool.inner_ref()).await?;
            tracing::info!("done.");
        }

        // 毎晩リブートするので起動時にデータベースを最適化
        tracing::info!("Optimizing database...");
        sqlx::query("PRAGMA optimize")
            .execute(pool.inner_ref())
            .await?;
        sqlx::query("VACUUM").execute(pool.inner_ref()).await?;
        sqlx::query("ANALYZE").execute(pool.inner_ref()).await?;
        tracing::info!("Database optimization completed.");

        Ok(pool)
    }
}
