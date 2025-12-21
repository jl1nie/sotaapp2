use anyhow::{Context, Result};
use chrono::Duration;
use tokio::sync::watch;

#[derive(Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
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
    pub pota_parklist_endpoint: String,
    pub pota_parklist_update_schedule: String,
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
    pub aprs_exclude_user: Option<String>,
    pub aprs_arrival_mesg_regex: Option<String>,
    pub reboot_after_update: bool,
    pub shutdown_tx: watch::Sender<bool>,
    pub shutdown_rx: watch::Receiver<bool>,
}

/// 環境変数を取得（必須）
fn env_required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("環境変数 {} が設定されていません", key))
}

/// 環境変数を取得してパース（デフォルト値あり）
fn env_parse_or<T: std::str::FromStr>(key: &str, default: T) -> T
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// 環境変数を取得（デフォルト値あり）
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 必須環境変数のリスト
const REQUIRED_ENV_VARS: &[&str] = &[
    "DATABASE_URL",
    "FIREBASE_API_KEY",
    "APRSUSER",
    "APRSPASSWORD",
];

impl AppConfig {
    /// 必須環境変数を事前検証
    pub fn validate_required_env() -> Result<()> {
        let missing: Vec<_> = REQUIRED_ENV_VARS
            .iter()
            .filter(|key| std::env::var(key).is_err())
            .collect();

        if !missing.is_empty() {
            anyhow::bail!(
                "必須環境変数が設定されていません: {}",
                missing.into_iter().copied().collect::<Vec<_>>().join(", ")
            );
        }
        Ok(())
    }

    pub fn new() -> Result<Self> {
        Self::validate_required_env()?;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Ok(Self {
            // 必須の設定
            host: env_or("HOST", "0.0.0.0"),
            port: env_parse_or("PORT", 8080),
            log_level: env_or("LOG_LEVEL", "info"),
            database: env_required("DATABASE_URL")?,
            migration_path: env_or("MIGRATION_PATH", "./migrations"),
            firebase_api_key: env_required("FIREBASE_API_KEY")?,

            // オプションの設定
            run_migration: env_parse_or("RUN_MIGRATION", false),
            cors_origin: std::env::var("CORS_ORIGIN").ok(),

            // 認証
            auth_token_ttl: Duration::hours(env_parse_or("AUTH_TOKEN_TTL", 24)),

            // SOTA エンドポイント
            sota_alert_endpoint: env_or(
                "SOTA_ALERT_ENDPOINT",
                "https://api2.sota.org.uk/api/alerts",
            ),
            sota_spot_endpoint: env_or(
                "SOTA_SPOT_ENDPOINT",
                "https://api2.sota.org.uk/api/spots/20?",
            ),
            sota_summitlist_endpoint: env_or(
                "SOTA_SUMMITLIST_ENDPOINT",
                "https://www.sotadata.org.uk/summitslist.csv",
            ),
            sota_summitlist_update_schedule: env_or("SUMMITLIST_SCHEDULE", "0 0 9 * * *"),

            // POTA エンドポイント
            pota_alert_endpoint: env_or("POTA_ALERT_ENDPOINT", "https://api.pota.app/activation/"),
            pota_spot_endpoint: env_or(
                "POTA_SPOT_ENDPOINT",
                "https://api.pota.app/spot/activator/",
            ),
            pota_parklist_endpoint: env_or(
                "POTA_PARKLIST_ENDPOINT",
                "https://pota.app/all_parks_ext.csv",
            ),
            pota_parklist_update_schedule: env_or("PARKLIST_SCHEDULE", "0 0 10 * * *"),

            // Geomag
            geomag_endpoint: env_or(
                "GEOMAG_ENDPOINT",
                "https://services.swpc.noaa.gov/text/daily-geomagnetic-indices.txt",
            ),
            geomag_update_schedule: env_or("GEOMAG_SCHEDULE", "0 0 */3 * * *"),

            // Mapcode
            mapcode_endpoint: env_or("MAPCODE_ENDPOINT", "https://japanmapcode.com/mapcode"),

            // 更新間隔（秒）
            alert_update_interval: env_parse_or("ALERT_INTERVAL", 600),
            spot_update_interval: env_parse_or("SPOT_INTERVAL", 120),

            // 有効期限
            alert_expire: Duration::hours(env_parse_or("ALERT_EXPIRE", 24)),
            spot_expire: Duration::hours(env_parse_or("SPOT_EXPIRE", 48)),
            aprs_log_expire: Duration::days(env_parse_or("APRS_LOG_EXPIRE", 10)),
            pota_log_expire: Duration::days(env_parse_or("POTA_LOG_EXPIRE", 180)),

            // APRS
            aprs_host: env_or("APRSHOST", "rotate.aprs2.net:14580"),
            aprs_user: env_required("APRSUSER")?,
            aprs_password: env_required("APRSPASSWORD")?,
            aprs_exclude_user: std::env::var("APRS_EXCLUDE_USER").ok(),
            aprs_arrival_mesg_regex: std::env::var("APRS_ARRIVAL_MESG_REGEX").ok(),

            // その他
            reboot_after_update: env_parse_or("REBOOT_AFTER_UPDATE", false),

            shutdown_rx,
            shutdown_tx,
        })
    }
}
