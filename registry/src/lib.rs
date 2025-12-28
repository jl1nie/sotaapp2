use axum::extract::FromRef;
use common::config::AppConfig;
use shaku::module;
use std::sync::Arc;

use aprs_message::AprsIS;

use adapter::{
    aprs::{AprsRepositryImpl, AprsRepositryImplParameters},
    database::connect::ConnectionPool,
    geomag::{GeoMag, GeoMagRepositryImpl, GeoMagRepositryImplParameters},
    minikvs::{MiniKvs, MiniKvsRepositryImpl, MiniKvsRepositryImplParameters},
};

use service::implement::{
    admin_periodic::{AdminPeriodicServiceImpl, AdminPeriodicServiceImplParameters},
    admin_service::{AdminServiceImpl, AdminServiceImplParameters},
    pota_log_service::{PotaLogServiceImpl, PotaLogServiceImplParameters},
    sota_log_service::SotaLogServiceImpl,
    user_service::UserServiceImpl,
};

#[cfg(not(feature = "sqlite"))]
use adapter::database::implement::postgis::{
    activation::{ActivationRepositryImpl, ActivationRepositryImplParameters},
    aprslog::{AprsLogRepositoryImpl, AprsLogRepositoryImplParameters},
    healthcheck::{HealthCheckRepositryImpl, HealthCheckRepositryImplParameters},
    locator::{LocatorRepositryImpl, LocatorRepositryImplParameters},
    pota_reference::{POTARepositoryImpl, POTARepositoryImplParameters},
    sota_reference::{SOTARepositoryImpl, SOTARepositoryImplParameters},
};

#[cfg(feature = "sqlite")]
use adapter::database::implement::sqlite::{
    activation::{ActivationRepositryImpl, ActivationRepositryImplParameters},
    aprslog::{AprsLogRepositoryImpl, AprsLogRepositoryImplParameters},
    healthcheck::{HealthCheckRepositryImpl, HealthCheckRepositryImplParameters},
    locator::{LocatorRepositryImpl, LocatorRepositryImplParameters},
    pota_reference::{PotaRepositoryImpl, PotaRepositoryImplParameters},
    sota_reference::{SotaRepositoryImpl, SotaRepositoryImplParameters},
};

module! {
    pub AppRegistry {
        components = [UserServiceImpl, SotaLogServiceImpl, PotaLogServiceImpl, AdminServiceImpl, AdminPeriodicServiceImpl,ActivationRepositryImpl,
        SotaRepositoryImpl,PotaRepositoryImpl,
        LocatorRepositryImpl,GeoMagRepositryImpl,AprsRepositryImpl,AprsLogRepositoryImpl,
        MiniKvsRepositryImpl,
        HealthCheckRepositryImpl],
        providers = [],
    }
}

impl AppRegistry {
    pub fn new(
        config: &AppConfig,
        pool: ConnectionPool,
        aprs: AprsIS,
        geomag: GeoMag,
        kvs: Arc<MiniKvs>,
    ) -> Self {
        AppRegistry::builder()
            .with_component_parameters::<SotaRepositoryImpl>(SotaRepositoryImplParameters {
                pool: pool.clone(),
            })
            .with_component_parameters::<PotaRepositoryImpl>(PotaRepositoryImplParameters {
                config: config.clone(),
                pool: pool.clone(),
            })
            .with_component_parameters::<ActivationRepositryImpl>(
                ActivationRepositryImplParameters { pool: pool.clone() },
            )
            .with_component_parameters::<AprsLogRepositoryImpl>(AprsLogRepositoryImplParameters {
                pool: pool.clone(),
            })
            .with_component_parameters::<LocatorRepositryImpl>(LocatorRepositryImplParameters {
                config: config.clone(),
                pool: pool.clone(),
            })
            .with_component_parameters::<PotaLogServiceImpl>(PotaLogServiceImplParameters {
                config: config.clone(),
            })
            .with_component_parameters::<AdminServiceImpl>(AdminServiceImplParameters {})
            .with_component_parameters::<AdminPeriodicServiceImpl>(
                AdminPeriodicServiceImplParameters {
                    config: config.clone(),
                },
            )
            .with_component_parameters::<GeoMagRepositryImpl>(GeoMagRepositryImplParameters {
                geomag: geomag.clone(),
            })
            .with_component_parameters::<AprsRepositryImpl>(AprsRepositryImplParameters {
                aprs: aprs.clone(),
            })
            .with_component_parameters::<MiniKvsRepositryImpl>(MiniKvsRepositryImplParameters {
                kvs: kvs.clone(),
            })
            .with_component_parameters::<HealthCheckRepositryImpl>(
                HealthCheckRepositryImplParameters { pool: pool.clone() },
            )
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

impl From<&AppState> for Arc<AppRegistry> {
    fn from(app_state: &AppState) -> Arc<AppRegistry> {
        app_state.module.clone()
    }
}
