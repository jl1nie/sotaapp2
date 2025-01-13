use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};

use chrono::{Duration, Utc};
use shaku_axum::Inject;

use common::error::{AppError, AppResult};

use domain::model::common::event::{DeleteRef, FindActBuilder, FindRefBuilder};
use domain::model::sota::SummitCode;

use registry::{AppRegistry, AppState};
use service::model::sota::{UploadSOTACSV, UploadSOTAOptCSV};
use service::services::{AdminService, UserService};

use crate::model::{
    alerts::AlertResponse,
    group::GroupByResponse,
    param::{build_findref_query, GetParam},
    spots::SpotResponse,
};

use crate::model::sota::{PagenatedResponse, SOTARefResponse, UpdateRefRequest};

async fn update_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_sota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn import_summit_list(
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

async fn update_summit_list(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let reqs = UploadSOTACSV { data };

        return admin_service
            .update_summit_list(reqs)
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
    let query = FindRefBuilder::default()
        .sota()
        .sota_code(summit_code)
        .build();
    let result = admin_service.show_sota_reference(query).await?;
    Ok(Json(result.into()))
}

async fn show_all_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<PagenatedResponse<SOTARefResponse>>> {
    let mut query = FindRefBuilder::default().sota();
    if param.limit.is_some() {
        query = query.limit(param.limit.unwrap());
    }

    if param.offset.is_some() {
        query = query.offset(param.offset.unwrap());
    }
    let result = admin_service
        .show_all_sota_references(query.build())
        .await?;
    Ok(Json(result.into()))
}
async fn search_sota_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<SOTARefResponse>>> {
    let query = FindRefBuilder::default().sota();
    let query = build_findref_query(param, query)?;

    let results = user_service.find_references(query).await?;

    let res: Vec<_> = results
        .sota
        .unwrap_or(vec![])
        .into_iter()
        .map(SOTARefResponse::from)
        .collect();
    Ok(Json(res))
}

async fn show_sota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<(GroupByResponse, Vec<SpotResponse>)>>> {
    let hours = param.after.unwrap_or(3);
    let query = FindActBuilder::default()
        .sota()
        .after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_spots(query).await?;
    let spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            (
                k.into(),
                v.into_iter().map(SpotResponse::from).collect::<Vec<_>>(),
            )
        })
        .collect();
    Ok(Json(spots))
}

async fn show_sota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<(GroupByResponse, Vec<AlertResponse>)>>> {
    let hours = param.after.unwrap_or(3);
    let query = FindActBuilder::default()
        .sota()
        .after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_alerts(query).await?;
    let alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            (
                k.into(),
                v.into_iter().map(AlertResponse::from).collect::<Vec<_>>(),
            )
        })
        .collect();
    Ok(Json(alerts))
}

pub fn build_sota_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/import", post(import_summit_list))
        .route("/import/ja", post(import_sota_opt_reference))
        .route("/update", post(update_summit_list))
        .route("/spots", get(show_sota_spots))
        .route("/alerts", get(show_sota_alerts))
        .route("/summits", get(show_all_sota_reference))
        .route("/summits/search", get(search_sota_reference))
        .route("/summits/:summit_code", get(show_sota_reference))
        .route("/summits/:summit_code", put(update_sota_reference))
        .route("/summits/:summit_code", delete(delete_sota_reference));

    Router::new().nest("/sota", routers)
}
