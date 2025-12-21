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
