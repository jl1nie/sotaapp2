//! 共有HTTPクライアント
//!
//! `reqwest::Client` は接続プールを持つため、アプリケーション全体で共有することで
//! 効率的な接続再利用が可能になる。

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::Client;

/// 共有HTTPクライアント（遅延初期化）
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

/// デフォルトのタイムアウト（秒）
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 共有HTTPクライアントを取得
pub fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client")
    })
}

/// カスタムUser-Agentでリクエストを送信するためのヘルパー
pub mod header {
    pub use reqwest::header::*;
}

/// リトライ設定
#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(10),
        }
    }
}

/// リトライ付きで非同期処理を実行
///
/// # Arguments
/// * `name` - ログ出力用の処理名
/// * `config` - リトライ設定
/// * `f` - 実行する非同期処理（Result<T, E>を返す）
///
/// # Returns
/// 成功時はSome(T)、全リトライ失敗時はNone
pub async fn with_retry<T, E, F, Fut>(name: &str, config: &RetryConfig, f: F) -> Option<T>
where
    E: std::fmt::Display,
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut delay = config.initial_delay;

    for attempt in 1..=config.max_retries {
        match f().await {
            Ok(result) => return Some(result),
            Err(e) => {
                if attempt < config.max_retries {
                    tracing::warn!(
                        "{} failed (attempt {}/{}): {}. Retrying in {:?}...",
                        name,
                        attempt,
                        config.max_retries,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, config.max_delay);
                } else {
                    tracing::error!(
                        "{} failed after {} attempts: {}",
                        name,
                        config.max_retries,
                        e
                    );
                }
            }
        }
    }
    None
}
