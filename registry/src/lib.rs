use application::HealthCheck;
use application::SOTA;

use data_access::sota::SOTAImpl;
use data_access::{database::ConnectionPool, health::HealthCheckImpl};

use std::sync::Arc;
#[derive(Clone)]
pub struct AppRegistry {
    health_check: Arc<dyn HealthCheck>,
    sota: Arc<dyn SOTA>,
}

impl AppRegistry {
    pub fn new(pool: ConnectionPool) -> Self {
        let health_check = Arc::new(HealthCheckImpl::new(pool.clone()));
        let sota = Arc::new(SOTAImpl::new(pool.clone()));
        Self { health_check, sota }
    }

    pub fn health_check(&self) -> Arc<dyn HealthCheck> {
        self.health_check.clone()
    }

    pub fn sota(&self) -> Arc<dyn SOTA> {
        self.sota.clone()
    }
}
