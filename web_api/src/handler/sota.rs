use application::model::sota::event::DeleteRef;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::error::{AppError, AppResult};
use geo_types::{coord, Rect};
use registry::AppRegistry;

use crate::model::sota::{CreateRefRequest, GetParam, SOTARefResponse, UpdateRefRequest};

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub async fn create_sota_reference(
    State(registry): State<AppRegistry>,
    Json(req): Json<CreateRefRequest>,
) -> AppResult<StatusCode> {
    registry
        .sota()
        .create_a_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

pub async fn upload_sota_reference(
    State(registry): State<AppRegistry>,
    Json(req): Json<CreateRefRequest>,
) -> AppResult<StatusCode> {
    registry
        .sota()
        .create_a_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

pub async fn show_sota_reference(
    State(registry): State<AppRegistry>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SOTARefResponse>> {
    registry
        .sota()
        .find_by_summit_code(&summit_code)
        .await
        .and_then(|r| match r {
            Some(r) => Ok(Json(r.into())),
            None => Err(AppError::EntityNotFound("Summit not found.".to_string())),
        })
}

pub async fn show_sota_reference_list(
    State(registry): State<AppRegistry>,
    Query(pos): Query<GetParam>,
) -> AppResult<StatusCode> {
    let r = Rect::new(
        coord! {x: pos.min_lon, y: pos.min_lat},
        coord! {x: pos.max_lon, y: pos.max_lat},
    );
    registry
        .sota()
        .find_by_location(&r)
        .await
        .map(|_| StatusCode::CREATED)
}

pub async fn update_sota_reference(
    State(registry): State<AppRegistry>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    registry
        .sota()
        .update_a_reference(req.into())
        .await
        .map(|_| StatusCode::OK)
}

pub async fn delete_sota_reference(
    State(registry): State<AppRegistry>,
    Path(summit_code): Path<String>,
) -> AppResult<StatusCode> {
    let req = DeleteRef { summit_code };
    registry
        .sota()
        .delete_a_reference(req)
        .await
        .map(|_| StatusCode::OK)
}

pub fn build_sota_routers() -> Router<AppRegistry> {
    let routers = Router::new()
        .route("/", post(create_sota_reference))
        .route("/", get(show_sota_reference_list))
        .route("/upload", post(upload_sota_reference))
        .route("/:summit_code", get(show_sota_reference))
        .route("/:summit_code", put(update_sota_reference))
        .route("/:summit_code", delete(delete_sota_reference));

    Router::new().nest("/sota", routers)
}
