use axum::extract::DefaultBodyLimit;
use axum::Router;
use firebase_auth_sdk::FireAuth;
use registry::AppState;

use super::{
    activation::build_activation_routers, auth::build_auth_routers, fle::fle_router,
    health::build_health_chek_routers, locator::build_locator_routers, logconv::logconv_router,
    pota::build_pota_routers, propagation::build_propagation_routers, search::build_search_routers,
    sota::build_sota_routers, wspr::wspr_router,
};

pub fn routes(auth: FireAuth) -> Router<AppState> {
    let router = Router::new()
        .merge(build_health_chek_routers())
        .merge(build_sota_routers(&auth))
        .merge(build_pota_routers(&auth))
        .merge(build_locator_routers(&auth))
        .merge(build_propagation_routers())
        .merge(build_search_routers())
        .merge(build_activation_routers())
        .merge(build_auth_routers(&auth))
        .nest("/wspr", wspr_router())
        .nest("/logconv", logconv_router())
        .nest("/fle", fle_router())
        .layer(DefaultBodyLimit::max(1024 * 1024 * 64));

    Router::new().nest("/api/v2", router)
}
