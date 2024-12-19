use axum::extract::FromRef;
use common::config::AppConfig;
use shaku::module;
use std::sync::Arc;

use adapter::{
    database::connect_database_with,
    implement::{
        healthcheck::HealthCheckRepositryImplParameters,
        pota_activation::POTActivationRepositryImplParameters,
        pota_reference::POTAReferenceRepositryImplParameters,
        sota_activation::SOTAActivationRepositryImplParameters,
        sota_reference::SOTAReferenceReposityImplParameters,
    },
};

use service::implement::{
    admin_periodic::{AdminPeriodicServiceImpl, AdminPeriodicServiceImplParameters},
    admin_service::{AdminServiceImpl, AdminServiceImplParameters},
    user_service::{UserServiceImpl, UserServiceImplParameters},
};

use adapter::implement::{
    healthcheck::HealthCheckRepositryImpl, pota_activation::POTActivationRepositryImpl,
    pota_reference::POTAReferenceRepositryImpl, sota_activation::SOTAActivationRepositryImpl,
    sota_reference::SOTAReferenceReposityImpl,
};

module! {
    pub AppRegistry {
        components = [UserServiceImpl, AdminServiceImpl, AdminPeriodicServiceImpl,
        SOTAReferenceReposityImpl,SOTAActivationRepositryImpl,POTAReferenceRepositryImpl,POTActivationRepositryImpl,
        HealthCheckRepositryImpl],
        providers = [],
    }
}

impl AppRegistry {
    pub fn new(config: &AppConfig) -> Self {
        let pool = connect_database_with(config).unwrap();
        AppRegistry::builder()
            .with_component_parameters::<SOTAReferenceReposityImpl>(
                SOTAReferenceReposityImplParameters {
                    config: config.clone(),
                    pool: pool.clone(),
                },
            )
            .with_component_parameters::<SOTAActivationRepositryImpl>(
                SOTAActivationRepositryImplParameters {
                    config: config.clone(),
                    pool: pool.clone(),
                },
            )
            .with_component_parameters::<POTAReferenceRepositryImpl>(
                POTAReferenceRepositryImplParameters {
                    config: config.clone(),
                    pool: pool.clone(),
                },
            )
            .with_component_parameters::<POTActivationRepositryImpl>(
                POTActivationRepositryImplParameters {
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
            .with_component_parameters::<HealthCheckRepositryImpl>(
                HealthCheckRepositryImplParameters { pool: pool.clone() },
            )
            .build()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub module: Arc<AppRegistry>,
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
