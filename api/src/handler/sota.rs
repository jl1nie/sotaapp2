use std::vec;

use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};

use chrono::{Duration, Utc};
use shaku_axum::Inject;

use crate::model::sota::{
    SOTARefResponse, SOTARefSearchResponse, SOTASearchResult, UpdateRefRequest,
};
use crate::model::{alerts::AlertResponse, param::GetParam, spots::SpotResponse};
use common::error::{AppError, AppResult};
use domain::model::common::event::{DeleteRef, FindActBuilder, FindRefBuilder};
use domain::model::sota::SummitCode;
use registry::{AppRegistry, AppState};

use service::model::sota::{UploadSOTACSV, UploadSOTAOptCSV};
use service::services::{AdminService, UserService};

async fn update_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_sota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn import_sota_reference(
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

async fn import_sota_opt_reference(
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

async fn delete_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(summit_code): Path<String>,
) -> AppResult<StatusCode> {
    let req = DeleteRef::Delete(SummitCode::new(summit_code));
    admin_service
        .delete_sota_reference(req)
        .await
        .map(|_| StatusCode::OK)
}

async fn show_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SOTARefResponse>> {
    let query = FindRefBuilder::default().sota().ref_id(summit_code).build();
    let mut result = admin_service.find_sota_reference(query).await?;
    if let Some(sotaref) = result.pop() {
        Ok(Json(sotaref.into()))
    } else {
        Err(AppError::EntityNotFound("Summit not found.".to_string()))
    }
}

async fn show_sota_reference_list(
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
    res.results = result.into_iter().map(SOTASearchResult::from).collect();
    res.count = res.results.len() as i32;
    if param.max_results.is_some() && res.count > param.max_results.unwrap() {
        res.results = vec![];
    }
    return Ok(Json(res));
}

async fn show_sota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<SpotResponse>>> {
    let hours = param.after.unwrap_or(3);
    let query = FindActBuilder::default()
        .sota()
        .after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_spots(query).await?;
    let spots: Vec<_> = result.into_iter().map(SpotResponse::from).collect();
    Ok(Json(spots))
}

async fn show_sota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<AlertResponse>>> {
    let hours = param.after.unwrap_or(3);
    let query = FindActBuilder::default()
        .sota()
        .after(Utc::now() - Duration::hours(hours))
        .build();
    tracing::info!("query: {:?}", query);
    let result = user_service.find_alerts(query).await?;
    let alerts: Vec<_> = result.into_iter().map(AlertResponse::from).collect();
    Ok(Json(alerts))
}

pub fn build_sota_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", get(show_sota_reference_list))
        .route("/import", post(import_sota_reference))
        .route("/import/ja", post(import_sota_opt_reference))
        .route("/spots", get(show_sota_spots))
        .route("/alerts", get(show_sota_alerts))
        .route("/:summit_code", get(show_sota_reference))
        .route("/:summit_code", put(update_sota_reference))
        .route("/:summit_code", delete(delete_sota_reference));

    Router::new().nest("/sota", routers)
}
