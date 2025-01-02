use axum::{
    extract::{Path, Query},
    routing::get,
    Json, Router,
};

use shaku_axum::Inject;

use crate::model::locator::{CenturyCodeResponse, MapcodeResponse};
use crate::model::param::GetParam;
use common::error::AppResult;
use registry::{AppRegistry, AppState};
use service::services::UserService;

async fn find_century_code(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(muni_code): Path<String>,
) -> AppResult<Json<CenturyCodeResponse>> {
    let muni_code: i32 = muni_code.parse().unwrap_or_default();
    let result = user_service.find_century_code(muni_code).await?;
    let result = result.get_first().unwrap().into();
    Ok(Json(result))
}

async fn find_mapcode(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<MapcodeResponse>> {
    let (lon, lat) = (param.lon.unwrap_or_default(), param.lat.unwrap_or_default());
    let result = user_service.find_mapcode(lon, lat).await?;
    Ok(Json(result.into()))
}

pub fn build_locator_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/jcc-jcg/:muni_code", get(find_century_code))
        .route("/mapcode", get(find_mapcode));
    Router::new().nest("/locator", routers)
}
