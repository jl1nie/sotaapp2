use super::cache::{cached, GEOMAG_SECS};
use axum::{routing::get, Json, Router};
use shaku_axum::Inject;

use crate::model::geomag::GeomagView;
use common::error::{AppError, AppResult};
use registry::{AppRegistry, AppState};
use service::services::UserService;

async fn get_geomag(
    user_service: Inject<AppRegistry, dyn UserService>,
) -> AppResult<Json<GeomagView>> {
    let result = user_service.get_geomagnetic().await?;
    if let Some(result) = result {
        return Ok(Json(result.into()));
    }
    Err(AppError::EntityNotFound("GeoMag Error".to_string()))
}

pub fn build_propagation_routers() -> Router<AppState> {
    let routers = cached(Router::new().route("/geomag", get(get_geomag)), GEOMAG_SECS);
    Router::new().nest("/propagation", routers)
}
