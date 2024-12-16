use async_trait::async_trait;
use common::error::AppResult;
use shaku::Component;

use crate::database::ConnectionPool;
use domain::repository::healthcheck::HealthCheckRepositry;

#[derive(Component)]
#[shaku(interface = HealthCheckRepositry)]
pub struct HealthCheckRepositryImpl {
    pool: ConnectionPool,
}

#[async_trait]
impl HealthCheckRepositry for HealthCheckRepositryImpl {
    async fn check_database(&self) -> AppResult<bool> {
        Ok(sqlx::query("SELECT 1")
            .fetch_one(self.pool.inner_ref())
            .await
            .is_ok())
    }
}
