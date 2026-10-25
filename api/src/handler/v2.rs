use axum::Router;
use registry::AppState;

use super::{
    activation::build_activation_routers, admin::build_admin_routers, auth::build_auth_routers,
    auth::AdminAuth, award_admin::build_award_admin_routers, fle::fle_router,
    health::build_health_chek_routers, locator::build_locator_routers, logconv::logconv_router,
    map::build_map_routers, multipart::user_upload_limit, pota::build_pota_routers,
    propagation::build_propagation_routers, search::build_search_routers, sota::build_sota_routers,
    wspr::wspr_router,
};

pub fn routes(auth: AdminAuth) -> Router<AppState> {
    let router = Router::new()
        .merge(build_health_chek_routers())
        .merge(build_sota_routers(&auth))
        .merge(build_pota_routers(&auth))
        .merge(build_locator_routers(&auth))
        .merge(build_map_routers())
        .merge(build_propagation_routers())
        .merge(build_search_routers())
        .merge(build_activation_routers())
        .merge(build_auth_routers(&auth))
        .merge(build_admin_routers(&auth))
        .merge(build_award_admin_routers(&auth))
        .nest("/wspr", wspr_router())
        .nest("/logconv", logconv_router())
        .nest("/fle", fle_router())
        // 認証ありのルートは `with_auth` が管理者の上限に広げる
        .layer(user_upload_limit());

    Router::new().nest("/api/v2", router)
}
