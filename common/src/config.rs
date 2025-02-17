use anyhow::Result;
use chrono::Duration;
use tokio::sync::watch;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub init_database: bool,
    pub run_migration: bool,
    pub migration_path: String,
    pub cors_origin: Option<String>,
    pub firebase_api_key: String,
    pub auth_token_ttl: Duration,
    pub log_level: String,
    pub sota_alert_endpoint: String,
    pub sota_spot_endpoint: String,
    pub pota_alert_endpoint: String,
    pub pota_spot_endpoint: String,
    pub sota_summitlist_endpoint: String,
    pub sota_summitlist_update_schedule: String,
    pub geomag_endpoint: String,
    pub geomag_update_schedule: String,
    pub mapcode_endpoint: String,
    pub alert_update_interval: u64,
    pub alert_expire: Duration,
    pub spot_update_interval: u64,
    pub spot_expire: Duration,
    pub aprs_log_expire: Duration,
    pub pota_log_expire: Duration,
    pub aprs_host: String,
    pub aprs_user: String,
    pub aprs_password: String,
    pub shutdown_tx: watch::Sender<bool>,
    pub shutdown_rx: watch::Receiver<bool>,
}

impl AppConfig {
    pub fn new() -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Ok(Self {
            host: std::env::var("HOST").expect("HOST"),
            port: std::env::var("PORT").expect("PORT").parse::<u16>()?,
            log_level: std::env::var("LOG_LEVEL").expect("LOG_LEVEL"),
            database: std::env::var("DATABASE_URL").expect("DATABASE_URL"),
            init_database: std::env::var("INIT_DATABASE")
                .expect("INIT_DATABASE")
                .parse::<bool>()?,
            run_migration: std::env::var("RUN_MIGRATION")
                .expect("RUN_MIGRATION")
                .parse::<bool>()?,
            migration_path: std::env::var("MIGRATION_PATH").expect("MIGRATION_PATH"),
            cors_origin: std::env::var("CORS_ORIGIN").ok(),
            firebase_api_key: std::env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY"),
            auth_token_ttl: Duration::hours(
                std::env::var("AUTH_TOKEN_TTL")
                    .expect("AUTH_TOKEN_TTL")
                    .parse::<i64>()?,
            ),
            sota_alert_endpoint: std::env::var("SOTA_ALERT_ENDPOINT").expect("SOTA_ALERT_ENDPOINT"),
            sota_spot_endpoint: std::env::var("SOTA_SPOT_ENDPOINT").expect("SOTA_SPOT_ENDPOINT"),
            sota_summitlist_endpoint: std::env::var("SOTA_SUMMITLIST_ENDPOINT")
                .expect("SOTA_SUMMITLIST_ENDPOINT"),
            sota_summitlist_update_schedule: std::env::var("SUMMITLIST_SCHEDULE")
                .expect("SUMMITLIST_SCHEDULE"),
            pota_alert_endpoint: std::env::var("POTA_ALERT_ENDPOINT").expect("POTA_ALERT_ENDPOINT"),
            pota_spot_endpoint: std::env::var("POTA_SPOT_ENDPOINT").expect("POTA_SPOT_ENDPOINT"),
            geomag_endpoint: std::env::var("GEOMAG_ENDPOINT").expect("GEOMAG_ENDPOINT"),
            geomag_update_schedule: std::env::var("GEOMAG_SCHEDULE").expect("GEOMAG_SCHEDULE"),
            mapcode_endpoint: std::env::var("MAPCODE_ENDPOINT").expect("MAPCODE_ENDPOINT"),
            alert_update_interval: std::env::var("ALERT_INTERVAL")
                .expect("ALERT INTERVAL")
                .parse::<u64>()?,
            spot_update_interval: std::env::var("SPOT_INTERVAL")
                .expect("SPOT_INTERVAL")
                .parse::<u64>()?,
            alert_expire: Duration::hours(
                std::env::var("ALERT_EXPIRE")
                    .expect("ALERT_EXPIRE")
                    .parse::<i64>()?,
            ),
            spot_expire: Duration::hours(
                std::env::var("SPOT_EXPIRE")
                    .expect("SPOT_EXPIRE")
                    .parse::<i64>()?,
            ),
            aprs_log_expire: Duration::days(
                std::env::var("APRS_LOG_EXPIRE")
                    .expect("APRS_LOG_EXPIRE")
                    .parse::<i64>()?,
            ),
            pota_log_expire: Duration::days(
                std::env::var("POTA_LOG_EXPIRE")
                    .expect("POTA_LOG_EXPIRE")
                    .parse::<i64>()?,
            ),
            aprs_host: std::env::var("APRSHOST").expect("APRSHOST"),
            aprs_user: std::env::var("APRSUSER").expect("APRSUSER"),
            aprs_password: std::env::var("APRSPASSWORD").expect("APRSPASSWORD"),
            shutdown_rx,
            shutdown_tx,
        })
    }
}
