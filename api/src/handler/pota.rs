use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use common::error::{AppError, AppResult};
use shaku_axum::Inject;
use std::str::FromStr;

use domain::model::pota::ParkCode;
use domain::model::{
    event::{DeleteRef, FindActBuilder, FindRefBuilder},
    id::LogId,
};

use registry::{AppRegistry, AppState};
use service::model::pota::{UploadActivatorCSV, UploadHunterCSV, UploadPOTACSV};
use service::services::{AdminService, UserService};

use crate::model::{
    activation::ActivationView,
    alerts::AlertView,
    param::{build_findref_query, GetParam},
    pota::POTARefLogView,
    spots::SpotView,
};

use crate::model::pota::{PagenatedResponse, PotaRefView, UpdateRefRequest};

async fn update_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_pota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn import_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadPOTACSV { data };

        return admin_service
            .import_pota_park_list(reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

async fn upload_pota_activator_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(log_id): Path<String>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();
        let log_id = LogId::from_str(&log_id)?;
        let reqs = UploadActivatorCSV { data };

        return user_service
            .upload_activator_csv(log_id, reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

async fn upload_pota_hunter_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(log_id): Path<String>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();
        let log_id = LogId::from_str(&log_id)?;
        let reqs = UploadHunterCSV { data };

        return user_service
            .upload_hunter_csv(log_id, reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

async fn delete_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(park_code): Path<String>,
) -> AppResult<StatusCode> {
    let req = DeleteRef::Delete(ParkCode::new(park_code));
    admin_service
        .delete_pota_reference(req)
        .await
        .map(|_| StatusCode::OK)
}

async fn show_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(park_code): Path<String>,
) -> AppResult<Json<PotaRefView>> {
    let query = FindRefBuilder::default()
        .pota()
        .pota_code(park_code)
        .build();
    let result = admin_service.show_pota_reference(query).await?;
    Ok(Json(result.into()))
}

async fn show_all_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<PagenatedResponse<PotaRefView>>> {
    let mut query = FindRefBuilder::default().pota();
    if param.limit.is_some() {
        query = query.limit(param.limit.unwrap());
    }

    if param.offset.is_some() {
        query = query.offset(param.offset.unwrap());
    }
    let result = admin_service
        .show_all_pota_references(query.build())
        .await?;
    Ok(Json(result.into()))
}

async fn find_pota_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<POTARefLogView>>> {
    let query = FindRefBuilder::default().pota();
    let query = build_findref_query(param, query)?;

    let results = user_service.find_references(query).await?;

    let res = results
        .pota
        .unwrap_or(vec![])
        .into_iter()
        .map(POTARefLogView::from)
        .collect();
    Ok(Json(res))
}

async fn show_pota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationView<SpotView>>>> {
    let hours = param.hours_ago.unwrap_or(3);
    let query = FindActBuilder::default()
        .pota()
        .issued_after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_spots(query).await?;
    let spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(SpotView::from).collect::<Vec<_>>()))
        })
        .collect();
    Ok(Json(spots))
}

async fn show_pota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationView<AlertView>>>> {
    let hours = param.hours_ago.unwrap_or(3);
    let query = FindActBuilder::default()
        .pota()
        .issued_after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_alerts(query).await?;
    let alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(AlertView::from).collect::<Vec<_>>()))
        })
        .collect();
    Ok(Json(alerts))
}

pub fn build_pota_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/import", post(import_pota_reference))
        .route(
            "/upload/activator/{user_id}",
            post(upload_pota_activator_log),
        )
        .route("/upload/hunter/{user_id}", post(upload_pota_hunter_log))
        .route("/spots", get(show_pota_spots))
        .route("/alerts", get(show_pota_alerts))
        .route("/parks", get(show_all_pota_reference))
        .route("/parks/search", get(find_pota_reference))
        .route("/parks/{park_code}", get(show_pota_reference))
        .route("/parks/{park_code}", put(update_pota_reference))
        .route("/parks/{park_code}", delete(delete_pota_reference));

    Router::new().nest("/pota", routers)
}
