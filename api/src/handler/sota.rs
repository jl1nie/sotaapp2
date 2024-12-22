use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};

use shaku_axum::Inject;

use crate::model::sota::{
    GetParam, SOTARefResponse, SOTARefSearchResponse, SOTASearchResult, UpdateRefRequest,
};
use common::error::{AppError, AppResult};
use domain::model::common::event::{DeleteRef, FindRefBuilder};
use domain::model::sota::SummitCode;
use registry::{AppRegistry, AppState};

use service::model::sota::{UploadSOTACSV, UploadSOTAOptCSV};
use service::services::AdminService;

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub async fn update_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_sota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

pub async fn import_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadSOTACSV { data };

        return admin_service
            .import_summit_list(reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

pub async fn import_sota_opt_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadSOTAOptCSV { data };

        return admin_service
            .import_summit_opt_list(reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

pub async fn delete_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(summit_code): Path<String>,
) -> AppResult<StatusCode> {
    let req = DeleteRef::Delete(SummitCode::new(summit_code));
    admin_service
        .delete_sota_reference(req)
        .await
        .map(|_| StatusCode::OK)
}

pub async fn show_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SOTARefResponse>> {
    let query = FindRefBuilder::default().sota().ref_id(summit_code).build();
    let result = admin_service.find_sota_reference(query).await?;
    if let Some(sotaref) = result.get_first() {
        Ok(Json(sotaref.into()))
    } else {
        Err(AppError::EntityNotFound("Summit not found.".to_string()))
    }
}

pub async fn show_sota_reference_list(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<SOTARefSearchResponse>> {
    let mut query = FindRefBuilder::default().sota();

    if param.limit.is_some() {
        query = query.limit(param.limit.unwrap());
    }

    if param.offset.is_some() {
        query = query.offset(param.offset.unwrap());
    }

    if param.name.is_some() {
        query = query.name(param.name.unwrap());
    }

    if param.ref_id.is_some() {
        query = query.ref_id(param.ref_id.unwrap());
    }

    if param.min_elev.is_some() {
        query = query.min_elev(param.min_elev.unwrap());
    }

    if param.max_lat.is_some()
        && param.min_lat.is_some()
        && param.max_lon.is_some()
        && param.min_lon.is_some()
    {
        query = query.bbox(
            param.min_lon.unwrap(),
            param.min_lat.unwrap(),
            param.max_lon.unwrap(),
            param.max_lat.unwrap(),
        );
    }

    let result = admin_service.find_sota_reference(query.build()).await?;

    let mut res = SOTARefSearchResponse::default();

    if let Some(sotarefs) = result.get_values() {
        res.result = sotarefs.into_iter().map(SOTASearchResult::from).collect();
    }

    if res.counts > 0 {
        return Ok(Json(res));
    }
    Err(AppError::EntityNotFound("Summit not found.".to_string()))
}

pub fn build_sota_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", post(update_sota_reference))
        .route("/", get(show_sota_reference_list))
        .route("/import", post(import_sota_reference))
        .route("/import/ja", post(import_sota_opt_reference))
        .route("/:summit_code", get(show_sota_reference))
        .route("/:summit_code", put(update_sota_reference))
        .route("/:summit_code", delete(delete_sota_reference));

    Router::new().nest("/sota", routers)
}
