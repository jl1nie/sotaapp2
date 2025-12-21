use axum::{
    extract::{Multipart, Query},
    middleware,
    routing::{get, post},
    Json, Router,
};
use firebase_auth_sdk::FireAuth;
use shaku_axum::Inject;

use crate::model::import::ImportResult;
use crate::model::{
    locator::{CenturyCodeView, MapcodeView},
    param::GetParam,
};
use common::error::{AppError, AppResult};
use common::utils::maidenhead;
use registry::{AppRegistry, AppState};
use service::model::locator::UploadMuniCSV;
use service::services::{AdminService, UserService};

use super::auth::auth_middle;

async fn import_muni_csv(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::UnprocessableEntity(format!("マルチパートの読み込みに失敗しました: {}", e)))?
        .ok_or_else(|| AppError::UnprocessableEntity("ファイルが送信されていません".to_string()))?;

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::UnprocessableEntity(format!("ファイルの読み込みに失敗しました: {}", e)))?;

    let data = String::from_utf8(data.to_vec())
        .map_err(|_| AppError::UnprocessableEntity("ファイルがUTF-8形式ではありません".to_string()))?;

    let reqs = UploadMuniCSV { data };

    let count = admin_service.import_muni_century_list(reqs).await?;

    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn find_century_code(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
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
    Query(param): Query<GetParam>,
) -> AppResult<Json<MapcodeView>> {
    let (lon, lat) = (param.lon.unwrap_or_default(), param.lat.unwrap_or_default());
    let mapcode = user_service.find_mapcode(lon, lat).await?;
    Ok(Json(mapcode.into()))
}

pub fn build_locator_routers(auth: &FireAuth) -> Router<AppState> {
    let protected = Router::new()
        .route("/jcc-jcg/import", post(import_muni_csv))
        .route_layer(middleware::from_fn_with_state(auth.clone(), auth_middle));

    let public = Router::new()
        .route("/jcc-jcg", get(find_century_code))
        .route("/mapcode", get(find_map_code));

    let routers = Router::new().merge(protected).merge(public);

    Router::new().nest("/locator", routers)
}
