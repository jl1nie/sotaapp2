use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    middleware,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use common::error::{AppError, AppResult};
use fastrand;
use firebase_auth_sdk::FireAuth;
use serde_json::{json, Value};
use shaku_axum::Inject;
use std::str::FromStr;

use crate::model::pota::{
    PagenatedResponse, PotaLogHistView, PotaLogStatView, PotaRefLogView, PotaRefView,
    UpdateRefRequest,
};
use crate::model::{
    activation::ActivationView,
    alerts::AlertView,
    param::{build_findref_query, GetParam},
    spots::SpotView,
};
use domain::model::{
    event::{DeleteRef, FindActBuilder, FindRefBuilder},
    id::LogId,
};
use domain::{
    model::pota::ParkCode, repository::minikvs::KvsRepositry, repository::pota::PotaRepository,
};
use registry::{AppRegistry, AppState};
use service::model::pota::{UploadPOTALog, UploadPOTAReference};
use service::services::{AdminService, UserService};

use super::auth::auth_middle;

async fn update_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_pota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn import_pota_reference_ja(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadPOTAReference { data };

        return admin_service
            .import_pota_park_list_ja(reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

async fn upload_pota_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path((activator_logid, hunter_logid)): Path<(String, String)>,
    mut multipart: Multipart,
) -> AppResult<Json<PotaLogHistView>> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadPOTALog {
            activator_logid,
            hunter_logid,
            data,
        };

        let res = user_service.upload_pota_log(reqs).await;

        if let Ok(loguser) = res {
            return Ok(Json(loguser.into()));
        }
    }
    Err(AppError::ForbiddenOperation)
}

async fn get_pota_logid(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(log_id): Path<String>,
) -> AppResult<Json<PotaLogHistView>> {
    let log_id = LogId::from_str(&log_id)?;
    let loguser = user_service.find_logid(log_id).await?;
    Ok(Json(loguser.into()))
}

async fn delete_pota_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(log_id): Path<String>,
) -> AppResult<StatusCode> {
    let log_id = LogId::from_str(&log_id)?;

    user_service
        .delete_pota_log(log_id)
        .await
        .map(|_| StatusCode::OK)
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
    } else {
        query = query.limit(500);
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
) -> AppResult<Json<Vec<PotaRefLogView>>> {
    let query = FindRefBuilder::default().pota();
    let mut query = build_findref_query(param, query)?;

    query.limit = query.limit.map_or(Some(500), |v| Some(v.min(500)));

    let results = user_service.find_references(query).await?;

    let res = results
        .pota
        .unwrap_or(vec![])
        .into_iter()
        .map(PotaRefLogView::from)
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

async fn reqeust_shareid(
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Path((act_id, hntr_id)): Path<(String, String)>,
) -> AppResult<Json<Value>> {
    let mut share_id: u16 = fastrand::u16(1000..=9999);

    while kvs_repo.get(&share_id.to_string()).await.is_some() {
        share_id = fastrand::u16(1000..=9999);
    }

    let value = json!({ "share_id": share_id, "activator_logid": act_id, "hunter_logid": hntr_id });

    kvs_repo
        .set(
            share_id.to_string(),
            value.clone(),
            Some(Duration::minutes(30)),
        )
        .await;

    Ok(Json(value))
}

async fn obtain_shareid(
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Path(share_id): Path<String>,
) -> AppResult<Json<Value>> {
    if let Some(value) = kvs_repo.get(&share_id).await {
        Ok(Json(value))
    } else {
        Err(AppError::EntityNotFound("invalid share_id".to_string()))
    }
}

async fn log_stat(
    pota_repo: Inject<AppRegistry, dyn PotaRepository>,
) -> AppResult<Json<PotaLogStatView>> {
    let stat = pota_repo.log_statistics().await?;
    Ok(Json(stat.into()))
}

async fn log_migrate(
    pota_repo: Inject<AppRegistry, dyn PotaRepository>,
    Query(param): Query<GetParam>,
) -> AppResult<StatusCode> {
    if let Some(dbname) = param.name {
        pota_repo
            .migrate_legacy_log(dbname)
            .await
            .map(|_| StatusCode::OK)?;
    }
    Ok(StatusCode::NOT_FOUND)
}

pub fn build_pota_routers(auth: &FireAuth) -> Router<AppState> {
    let protected = Router::new()
        .route("/import", post(import_pota_reference_ja))
        .route("/parks/{park_code}", put(update_pota_reference))
        .route("/parks/{park_code}", delete(delete_pota_reference))
        .route("/log-migrate", get(log_migrate))
        .route_layer(middleware::from_fn_with_state(auth.clone(), auth_middle));

    let public = Router::new()
        .route("/log/{act_id}/{hntr_id}", post(upload_pota_log))
        .route("/log/{log_id}", get(get_pota_logid))
        .route("/log/{log_id}", delete(delete_pota_log))
        .route("/log-share/{act_id}/{hntr_id}", get(reqeust_shareid))
        .route("/log-share/{share_id}", get(obtain_shareid))
        .route("/log-stat", get(log_stat))
        .route("/spots", get(show_pota_spots))
        .route("/alerts", get(show_pota_alerts))
        .route("/parks", get(show_all_pota_reference))
        .route("/parks/search", get(find_pota_reference))
        .route("/parks/{park_code}", get(show_pota_reference));

    let routers = Router::new().merge(protected).merge(public);

    Router::new().nest("/pota", routers)
}
