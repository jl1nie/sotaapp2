use async_trait::async_trait;
use bytes::Bytes;
use chrono::Duration;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

/// 有効期限つきの小さなキャッシュ。値は JSON を直列化したバイト列で持ち、そのままレスポンスに使う
#[cfg_attr(test, automock)]
#[async_trait]
pub trait KvsRepositry: Send + Sync + Interface {
    async fn set(&self, key: String, value: Bytes, expire: Option<Duration>);
    async fn get(&self, key: &str) -> Option<Bytes>;
}
