use application::HealthCheck;
use application::SOTADatabase;

use data_access::sota::SOTADatabaseImpl;
use data_access::{database::ConnectionPool, health::HealthCheckImpl};

use std::sync::Arc;
#[derive(Clone)]
pub struct AppRegistry {
    health_check: Arc<dyn HealthCheck>,
    sota_db: Arc<dyn SOTADatabase>,
}

impl AppRegistry {
    pub fn new(pool: ConnectionPool) -> Self {
        let health_check = Arc::new(HealthCheckImpl::new(pool.clone()));
        let sota_db = Arc::new(SOTADatabaseImpl::new(pool.clone()));
        Self {
            health_check,
            sota_db,
        }
    }

    pub fn health_check(&self) -> Arc<dyn HealthCheck> {
        self.health_check.clone()
    }

    pub fn sota_db(&self) -> Arc<dyn SOTADatabase> {
        self.sota_db.clone()
    }
}
