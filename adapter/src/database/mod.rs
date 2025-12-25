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

        // 最適化はサーバー起動後に非同期で実行（起動時間短縮のため）
        // optimize_database() を別途呼び出す
        sqlx::query("PRAGMA optimize")
            .execute(pool.inner_ref())
            .await?;

        Ok(pool)
    }

    /// サーバー起動後に非同期でデータベースを最適化
    pub async fn optimize_database(pool: &ConnectionPool) -> Result<()> {
        tracing::info!("Starting background database optimization...");
        sqlx::query("VACUUM").execute(pool.inner_ref()).await?;
        sqlx::query("ANALYZE").execute(pool.inner_ref()).await?;
        tracing::info!("Database optimization completed.");
        Ok(())
    }

    /// データベースのバックアップを作成
    pub fn backup_database(db_path: &str, backup_path: &str) -> Result<()> {
        let source = Path::new(db_path);
        let dest = Path::new(backup_path);
        fs::copy(source, dest)?;
        tracing::info!("Database backed up to {}", backup_path);
        Ok(())
    }

    /// データベースをリストア
    pub fn restore_database(backup_path: &str, db_path: &str) -> Result<()> {
        let source = Path::new(backup_path);
        let dest = Path::new(db_path);
        fs::copy(source, dest)?;
        tracing::info!("Database restored from {}", backup_path);
        Ok(())
    }

    /// データベースを初期化（削除して再作成）
    pub async fn reset_database(cfg: &AppConfig) -> Result<()> {
        let m = Migrator::new(std::path::Path::new(&cfg.migration_path)).await?;
        let dbname = cfg.database.replace("sqlite:", "");
        let database_path = Path::new(&dbname);

        // 既存のDBを削除
        if fs::metadata(database_path).is_ok() {
            tracing::warn!("Removing existing database: {}", database_path.display());
            fs::remove_file(database_path)?;
        }

        // 新規作成
        let _file = fs::File::create(database_path)?;
        tracing::info!("Created new database file: {}", database_path.display());

        // マイグレーション実行
        let pool = SqlitePool::connect(&cfg.database).await?;
        tracing::info!("Running migrations...");
        m.run(&pool).await?;
        tracing::info!("Database reset completed.");

        Ok(())
    }
}
