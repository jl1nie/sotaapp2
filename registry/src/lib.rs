use std::sync::Arc;

use data_access::{database::ConnectionPool, health::HealthCheckImpl, HealthCheck};
#[derive(Clone)]
pub struct AppRegistry {
    health_check: Arc<dyn HealthCheck>,
}

impl AppRegistry {
    pub fn new(pool: ConnectionPool) -> Self {
        let health_check = Arc::new(HealthCheckImpl::new(pool.clone()));
        Self { health_check }
    }

    pub fn health_check(&self) -> Arc<dyn HealthCheck> {
        self.health_check.clone()
    }
}
