use async_trait::async_trait;
use common::error::AppResult;
#[cfg(test)]
use mockall::automock;
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait HealthCheckRepositry: Send + Sync + Interface {
    async fn check_database(&self) -> AppResult<bool>;
}
