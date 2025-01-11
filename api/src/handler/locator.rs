use axum::{
    extract::{Multipart, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use shaku_axum::Inject;

use crate::model::{
    locator::{CenturyCodeResponse, MapcodeResponse},
    param::GetParam,
};
use common::csv_reader::maidenhead;
use common::error::{AppError, AppResult};
use registry::{AppRegistry, AppState};

use service::model::locator::UploadMuniCSV;
use service::services::{AdminService, UserService};

async fn import_muni_csv(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadMuniCSV { data };

        return admin_service
            .import_muni_century_list(reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

async fn find_century_code(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<CenturyCodeResponse>> {
    let muni_code: i32 = param.muni_code.unwrap_or_default();
    let (lon, lat) = (param.lon.unwrap_or_default(), param.lat.unwrap_or_default());
    let result = user_service.find_century_code(muni_code).await?;
    let mut result: CenturyCodeResponse = result.into();
    result.maidenhead = Some(maidenhead(lon, lat));
    Ok(Json(result))
}

async fn find_map_code(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<MapcodeResponse>> {
    let (lon, lat) = (param.lon.unwrap_or_default(), param.lat.unwrap_or_default());
    let mapcode = user_service.find_mapcode(lon, lat).await?;
    Ok(Json(mapcode.into()))
}

pub fn build_locator_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/jcc-jcg/import", post(import_muni_csv))
        .route("/jcc-jcg", get(find_century_code))
        .route("/mapcode", get(find_map_code));
    Router::new().nest("/locator", routers)
}
