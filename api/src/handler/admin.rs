//! 管理コンソールハンドラー
//!
//! システム状態の表示とグレースフルリブート機能

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use firebase_auth_sdk::FireAuth;
use registry::AppState;
use serde::Serialize;
use shaku_axum::Inject;
use std::time::Instant;

use super::auth::with_auth;
use registry::AppRegistry;
use service::services::AdminService;

/// アプリ起動時刻（グローバル）
static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// 起動時刻を初期化
pub fn init_start_time() {
    let _ = START_TIME.get_or_init(Instant::now);
}

/// システムメトリクス
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    /// アップタイム（秒）
    pub uptime_secs: u64,
    /// メモリ使用量（バイト）
    pub memory_used_bytes: Option<u64>,
    /// メモリ使用量（MB）
    pub memory_used_mb: Option<f64>,
    /// データベース状態
    pub db_status: String,
}

/// メトリクス取得
async fn get_metrics(admin_service: Inject<AppRegistry, dyn AdminService>) -> impl IntoResponse {
    let uptime = START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0);

    // メモリ使用量を取得（Linux /proc/self/statm）
    let (memory_bytes, memory_mb) = get_memory_usage();

    // DB状態確認
    let db_status = match admin_service.health_check().await.unwrap_or(false) {
        true => "healthy".to_string(),
        false => "unhealthy".to_string(),
    };

    let metrics = SystemMetrics {
        uptime_secs: uptime,
        memory_used_bytes: memory_bytes,
        memory_used_mb: memory_mb,
        db_status,
    };

    Json(metrics)
}

/// メモリ使用量を取得
fn get_memory_usage() -> (Option<u64>, Option<f64>) {
    // Linux: /proc/self/statm から RSS を読み取る
    if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(rss_pages) = parts[1].parse::<u64>() {
                let page_size = 4096u64; // 通常のページサイズ
                let bytes = rss_pages * page_size;
                let mb = bytes as f64 / (1024.0 * 1024.0);
                return (Some(bytes), Some(mb));
            }
        }
    }
    (None, None)
}

/// グレースフルリブート
async fn restart_server(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("Admin requested graceful restart");

    // shutdown signal を送信
    let _ = state.config.shutdown_tx.send(true);

    // レスポンス返却後に非同期でexit(1)
    // Fly.ioのrestart.policy='on-failure'でプロセスが再起動される
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tracing::info!("Exiting with code 1 for restart");
        std::process::exit(1);
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "restarting",
            "message": "Server will restart gracefully"
        })),
    )
}

/// 管理ルーター作成
pub fn build_admin_routers(auth: &FireAuth) -> Router<AppState> {
    let router = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/restart", post(restart_server));

    // 認証ミドルウェアを適用
    let protected = with_auth(router, auth);

    Router::new().nest("/admin", protected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_usage() {
        let (bytes, mb) = get_memory_usage();
        // Linux以外では None になる可能性
        if cfg!(target_os = "linux") {
            assert!(bytes.is_some());
            assert!(mb.is_some());
        }
    }

    #[test]
    fn test_init_start_time() {
        // 初期化テスト（複数回呼んでも安全）
        init_start_time();
        init_start_time();

        // 初期化後はSome
        assert!(START_TIME.get().is_some());
    }

    #[test]
    fn test_system_metrics_serialize() {
        let metrics = SystemMetrics {
            uptime_secs: 3661,
            memory_used_bytes: Some(104857600),
            memory_used_mb: Some(100.0),
            db_status: "healthy".to_string(),
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("uptime_secs"));
        assert!(json.contains("3661"));
        assert!(json.contains("healthy"));
    }

    #[test]
    fn test_system_metrics_serialize_null_memory() {
        let metrics = SystemMetrics {
            uptime_secs: 0,
            memory_used_bytes: None,
            memory_used_mb: None,
            db_status: "unhealthy".to_string(),
        };

        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("null"));
        assert!(json.contains("unhealthy"));
    }
}
