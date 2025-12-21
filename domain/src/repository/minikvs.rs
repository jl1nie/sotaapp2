use async_trait::async_trait;
use chrono::Duration;
#[cfg(test)]
use mockall::automock;
use serde_json::Value;
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait KvsRepositry: Send + Sync + Interface {
    async fn set(&self, key: String, value: Value, expire: Option<Duration>);
    async fn get(&self, key: &str) -> Option<Value>;
}
