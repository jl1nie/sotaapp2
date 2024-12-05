use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use csv::ReaderBuilder;
use geo_types::{coord, Rect};

use crate::model::sota::{
    CreateRefRequest, GetParam, SOTARefResponse, SOTARefShortResponse, UpdateRefRequest,
};
use application::model::sota::event::{
    CreateRef, CreateRefs, DeleteRef, SearchRefs, SearchResults,
};
use common::error::{AppError, AppResult};
use registry::AppRegistry;

pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub async fn create_sota_reference(
    State(registry): State<AppRegistry>,
    Json(req): Json<CreateRefRequest>,
) -> AppResult<StatusCode> {
    registry
        .sota_db()
        .create_a_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

pub async fn upload_sota_reference(
    State(registry): State<AppRegistry>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data.as_bytes());

        let mut sota_ref_list: Vec<CreateRef> = Vec::new();
        for result in rdr.deserialize() {
            let req: CreateRefRequest = result.unwrap();
            sota_ref_list.push(req.into());
        }

        let reqs = CreateRefs {
            requests: sota_ref_list,
        };

        return registry
            .sota_db()
            .create_references(reqs)
            .await
            .map(|_| StatusCode::CREATED);
    }
    Err(AppError::ForbiddenOperation)
}

pub async fn show_sota_reference(
    State(registry): State<AppRegistry>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SOTARefResponse>> {
    let req = SearchRefs {
        summit_code: Some(summit_code),
        ..Default::default()
    };
    let SearchResults { results, .. } = registry.sota_db().search(req).await?;
    if let Some(mut results) = results {
        if !results.is_empty() {
            let result = results.pop().unwrap();
            Ok(Json(result.into()))
        } else {
            Err(AppError::EntityNotFound("Summit not found.".to_string()))
        }
    } else {
        Err(AppError::EntityNotFound("Summit not found.".to_string()))
    }
}

pub async fn show_sota_reference_list(
    State(registry): State<AppRegistry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<SOTARefShortResponse>>> {
    let req: SearchRefs = if let Some(key) = param.key {
        let limit = param.limit.unwrap_or(100) as usize;
        SearchRefs {
            keyword: Some(key),
            max_results: Some(limit),
            ..Default::default()
        }
    } else if param.min_lon.is_some() {
        let min_lon = param.min_lon.unwrap();
        let min_lat = param.min_lat.unwrap_or(0.0);
        let max_lon = param.min_lon.unwrap_or(0.0);
        let max_lat = param.min_lat.unwrap_or(0.0);
        let elevation = param.elevation.unwrap_or(0);
        let limit = param.limit.unwrap_or(200) as usize;
        SearchRefs {
            region: Some(Rect::new(
                coord! {x: min_lon, y: min_lat},
                coord! {x: max_lon, y: max_lat},
            )),
            elevation: Some(elevation),
            max_results: Some(limit),
            ..Default::default()
        }
    } else {
        SearchRefs {
            ..Default::default()
        }
    };
    let SearchResults { brief_results, .. } = registry.sota_db().search(req).await?;
    if let Some(brief_results) = brief_results {
        Ok(Json(
            brief_results
                .into_iter()
                .map(SOTARefShortResponse::from)
                .collect(),
        ))
    } else {
        Err(AppError::EntityNotFound("Summit not found.".to_string()))
    }
}

pub async fn update_sota_reference(
    State(registry): State<AppRegistry>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    registry
        .sota_db()
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
        .sota_db()
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
