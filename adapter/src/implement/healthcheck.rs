use async_trait::async_trait;
use common::error::AppResult;
use shaku::Component;

use crate::database::ConnectionPool;
use domain::repository::healthcheck::HealthCheck;

#[derive(Component)]
#[shaku(interface = HealthCheck)]
pub struct HealthCheckImpl {
    pool: ConnectionPool,
}

#[async_trait]
impl HealthCheck for HealthCheckImpl {
    async fn check_database(&self) -> AppResult<bool> {
        Ok(sqlx::query("SELECT 1")
            .fetch_one(self.pool.inner_ref())
            .await
            .is_ok())
    }
}
