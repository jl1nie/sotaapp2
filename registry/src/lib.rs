use axum::extract::FromRef;
use shaku::module;
use std::sync::Arc;

use common::config::AppConfig;
use data_access::{
    database::connect_database_with,
    implement::{
        health::HealthCheckImplParameters, pota_activation::POTActivationDatabaseImplParameters,
        pota_database::POTADatabaseImplParameters,
        sota_activation::SOTAActivationDatabaseImplParameters,
        sota_database::SOTADatabaseImplParameters,
    },
};

use service::implement::{
    admin_periodic::{AdminPeriodicServiceImpl, AdminPeriodicServiceImplParameters},
    admin_service::{AdminServiceImpl, AdminServiceImplParameters},
    user_service::{UserServiceImpl, UserServiceImplParameters},
};

use data_access::implement::{
    health::HealthCheckImpl, pota_activation::POTActivationDatabaseImpl,
    pota_database::POTADatabaseImpl, sota_activation::SOTAActivationDatabaseImpl,
    sota_database::SOTADatabaseImpl,
};

module! {
    pub AppRegistry {
        components = [UserServiceImpl, AdminServiceImpl, AdminPeriodicServiceImpl,
        SOTADatabaseImpl,SOTAActivationDatabaseImpl,POTADatabaseImpl,POTActivationDatabaseImpl,
        HealthCheckImpl],
        providers = [],
    }
}

impl AppRegistry {
    pub fn new(config: AppConfig) -> Self {
        let pool = connect_database_with(&config).unwrap();
        AppRegistry::builder()
            .with_component_parameters::<SOTADatabaseImpl>(SOTADatabaseImplParameters {
                config: config.clone(),
                pool: pool.clone(),
            })
            .with_component_parameters::<SOTAActivationDatabaseImpl>(
                SOTAActivationDatabaseImplParameters {
                    config: config.clone(),
                    pool: pool.clone(),
                },
            )
            .with_component_parameters::<POTADatabaseImpl>(POTADatabaseImplParameters {
                config: config.clone(),
                pool: pool.clone(),
            })
            .with_component_parameters::<POTActivationDatabaseImpl>(
                POTActivationDatabaseImplParameters {
                    config: config.clone(),
                    pool: pool.clone(),
                },
            )
            .with_component_parameters::<UserServiceImpl>(UserServiceImplParameters {
                config: config.clone(),
            })
            .with_component_parameters::<AdminServiceImpl>(AdminServiceImplParameters {
                config: config.clone(),
            })
            .with_component_parameters::<AdminPeriodicServiceImpl>(
                AdminPeriodicServiceImplParameters {
                    config: config.clone(),
                },
            )
            .with_component_parameters::<HealthCheckImpl>(HealthCheckImplParameters {
                pool: pool.clone(),
            })
            .build()
    }
}

#[derive(Clone)]
pub struct AppState {
    module: Arc<AppRegistry>,
}

impl AppState {
    pub fn new(module: AppRegistry) -> Self {
        Self {
            module: Arc::new(module),
        }
    }
}

impl FromRef<AppState> for Arc<AppRegistry> {
    fn from_ref(app_state: &AppState) -> Arc<AppRegistry> {
        app_state.module.clone()
    }
}
