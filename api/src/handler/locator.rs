use axum::{
    extract::Multipart,
    routing::{get, post},
    Json, Router,
};
use firebase_auth_sdk::FireAuth;
use shaku_axum::Inject;

use crate::model::import::ImportResult;
use crate::model::{
    locator::{CenturyCodeView, MapcodeView},
    param::{GetParam, ValidatedQuery},
};
use common::error::AppResult;
use common::utils::maidenhead;
use registry::{AppRegistry, AppState};
use service::model::locator::UploadMuniCSV;
use service::services::{AdminService, UserService};

use super::auth::with_auth;
use super::multipart::extract_text_file;

async fn import_muni_csv(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadMuniCSV { data };
    let count = admin_service.import_muni_century_list(reqs).await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn find_century_code(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<CenturyCodeView>> {
    let muni_code: i32 = param.muni_code.unwrap_or_default();
    let (lon, lat) = (param.lon.unwrap_or_default(), param.lat.unwrap_or_default());
    let result = user_service.find_century_code(muni_code).await;

    let mut res = CenturyCodeView::default();
    if let Ok(result) = result {
        res = result.into();
    }
    res.maidenhead = maidenhead(lon, lat);
    Ok(Json(res))
}

async fn find_map_code(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<MapcodeView>> {
    let (lon, lat) = (param.lon.unwrap_or_default(), param.lat.unwrap_or_default());
    let mapcode = user_service.find_mapcode(lon, lat).await?;
    Ok(Json(mapcode.into()))
}

pub fn build_locator_routers(auth: &FireAuth) -> Router<AppState> {
    let protected = with_auth(
        Router::new().route("/jcc-jcg/import", post(import_muni_csv)),
        auth,
    );

    let public = Router::new()
        .route("/jcc-jcg", get(find_century_code))
        .route("/mapcode", get(find_map_code));

    let routers = Router::new().merge(protected).merge(public);

    Router::new().nest("/locator", routers)
}
