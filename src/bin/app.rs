use anyhow::{Context, Error, Result};
use axum::{http::HeaderValue, Router};
use chrono::Local;
use clap::{Parser, Subcommand};
use common::config::AppConfig;
use firebase_auth_sdk::FireAuth;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{net::TcpListener, sync::watch};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
};
use tracing_subscriber::EnvFilter;

use adapter::{
    aprs::connect_aprsis_with,
    database::connect::{backup_database, connect_database_with, reset_database, restore_database},
    geomag::connect_geomag_with,
    minikvs::MiniKvs,
};
use api::handler::v2;
use registry::{AppRegistry, AppState};

#[derive(Parser)]
#[command(name = "sotaapp2")]
#[command(about = "SOTA/POTA Application Server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the application server (default)
    Serve,

    /// Database maintenance commands
    Db {
        #[command(subcommand)]
        action: DbCommands,
    },
}

#[derive(Subcommand)]
enum DbCommands {
    /// Backup database to a file
    Backup {
        /// Output file path (default: /data/backup_YYYYMMDD_HHMMSS.db)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Restore database from a backup file
    Restore {
        /// Backup file path to restore from
        #[arg(short, long)]
        input: String,
    },

    /// Reset database (WARNING: destroys all data)
    Reset {
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Optimize database (VACUUM, ANALYZE)
    Optimize,

    /// Run pending migrations
    Migrate,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Db { action }) => handle_db_command(action).await,
        Some(Commands::Serve) | None => bootstrap().await,
    }
}

async fn handle_db_command(action: DbCommands) -> Result<()> {
    // 最低限のログ初期化
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    let config = AppConfig::new()?;
    let db_path = config.database.replace("sqlite:", "");

    match action {
        DbCommands::Backup { output } => {
            let backup_path = output.unwrap_or_else(|| {
                let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                format!("/data/backup_{}.db", timestamp)
            });
            backup_database(&db_path, &backup_path)?;
            println!("Backup created: {}", backup_path);
        }

        DbCommands::Restore { input } => {
            println!("Restoring database from: {}", input);
            println!("WARNING: This will overwrite the current database!");
            restore_database(&input, &db_path)?;
            println!("Database restored successfully.");
        }

        DbCommands::Reset { force } => {
            if !force {
                println!("WARNING: This will DELETE ALL DATA in the database!");
                println!("Use --force to confirm.");
                return Ok(());
            }
            println!("Resetting database...");
            reset_database(&config).await?;
            println!("Database reset completed.");
        }

        DbCommands::Optimize => {
            println!("Optimizing database...");
            let pool = connect_database_with(&config).await?;
            sqlx::query("PRAGMA optimize")
                .execute(pool.inner_ref())
                .await?;
            sqlx::query("VACUUM").execute(pool.inner_ref()).await?;
            sqlx::query("ANALYZE").execute(pool.inner_ref()).await?;
            println!("Database optimization completed.");
        }

        DbCommands::Migrate => {
            println!("Running migrations...");
            let m =
                sqlx::migrate::Migrator::new(std::path::Path::new(&config.migration_path)).await?;
            let pool = sqlx::sqlite::SqlitePool::connect(&config.database).await?;
            m.run(&pool).await?;
            println!("Migrations completed.");
        }
    }

    Ok(())
}

async fn bootstrap() -> Result<()> {
    let config = AppConfig::new()?;

    let filter = EnvFilter::new(&config.log_level);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .init();

    let pool = connect_database_with(&config).await?;
    let aprs = connect_aprsis_with(&config).await?;
    let geomag = connect_geomag_with(&config).await?;
    let minikvs = Arc::new(MiniKvs::new(config.auth_token_ttl));
    let module = AppRegistry::new(&config, pool, aprs, geomag, minikvs);
    let app_state = AppState::new(module);
    let job_state = app_state.clone();

    let firebase = FireAuth::new(config.firebase_api_key.clone());

    let cors = match config.cors_origin.clone() {
        Some(origin) => CorsLayer::new()
            .allow_origin(origin.parse::<HeaderValue>().unwrap())
            .allow_headers(Any)
            .allow_methods(Any),
        None => CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any),
    };

    let app = Router::new()
        .merge(v2::routes(firebase))
        .with_state(app_state)
        .layer(cors)
        .fallback_service(ServeDir::new("static").fallback(ServeFile::new("static/index.html")));

    let ip_addr: IpAddr = config.host.parse().context("無効なIPアドレス")?;
    let addr = SocketAddr::new(ip_addr, config.port);
    let listener = TcpListener::bind(&addr).await?;
    let http = async {
        axum::serve(listener, app)
            .with_graceful_shutdown(shudown_signal(config.shutdown_rx.clone()))
            .await
            .map_err(Error::from)
    };
    let job_monitor = async { api::aggregator::builder::build(&config, &job_state).await };

    tracing::info!("DATABASE_URL = {}", config.database);
    tracing::info!("Listening on {}", addr);

    let _res = tokio::join!(job_monitor, http);

    Ok(())
}

async fn shudown_signal(mut shutdown_rx: watch::Receiver<bool>) {
    let _ = shutdown_rx.changed().await;
    tracing::info!("Shutdowm axum server.");
}
