use async_trait::async_trait;
pub mod database;
pub mod health;

#[async_trait]
pub trait HealthCheck: Send + Sync {
    async fn check_db(&self) -> bool;
}
