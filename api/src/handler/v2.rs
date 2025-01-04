use axum::extract::DefaultBodyLimit;
use axum::Router;
use registry::AppState;

use crate::handler::{
    geomag::build_geomag_routers, health::build_health_chek_routers,
    locator::build_locator_routers, pota::build_pota_routers, sota::build_sota_routers,
};

pub fn routes() -> Router<AppState> {
    let router = Router::new()
        .merge(build_health_chek_routers())
        .merge(build_sota_routers())
        .merge(build_pota_routers())
        .merge(build_locator_routers())
        .merge(build_geomag_routers())
        .layer(DefaultBodyLimit::max(1024 * 1024 * 32));

    Router::new().nest("/api/v2", router)
}
