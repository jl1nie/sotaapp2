use crate::database::ConnectionPool;
use application::HealthCheck;
use async_trait::async_trait;
use derive_new::new;

#[derive(new)]
pub struct HealthCheckImpl {
    db: ConnectionPool,
}

#[async_trait]
impl HealthCheck for HealthCheckImpl {
    async fn check_db(&self) -> bool {
        sqlx::query("SELECT 1")
            .fetch_one(self.db.inner_ref())
            .await
            .is_ok()
    }
}
