use async_trait::async_trait;
use common::error::AppResult;
use shaku::Interface;

#[async_trait]
pub trait HealthCheckRepositry: Send + Sync + Interface {
    async fn check_database(&self) -> AppResult<bool>;
}
