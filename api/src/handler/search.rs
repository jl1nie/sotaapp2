use axum::{extract::Query, routing::get, Json, Router};

use shaku_axum::Inject;
use std::time::Instant;

use common::error::AppResult;
use domain::model::event::{FindRefBuilder, FindResult};

use registry::{AppRegistry, AppState};
use service::services::UserService;

use crate::model::param::{build_findref_query, GetParam};

use crate::model::search::{SearchBriefResponse, SearchFullResponse, SearchResponse};

async fn search(
    user_service: Inject<AppRegistry, dyn UserService>,
    param: GetParam,
) -> AppResult<FindResult> {
    let start = Instant::now();
    let query = FindRefBuilder::default().sota().pota();
    let query = build_findref_query(param, query)?;

    let results = user_service.find_references(query).await?;
    let end = start.elapsed();
    tracing::info!("find reference {}ms", end.as_millis());
    Ok(results)
}

async fn search_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<SearchResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

async fn search_reference_full(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<SearchFullResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

async fn search_reference_breif(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<SearchBriefResponse>> {
    let max_count = param.max_count;
    let results = search(user_service, param).await?;
    let mut res: SearchBriefResponse = results.into();

    if res.count > max_count.unwrap_or(100) {
        res.candidates = vec![];
    }

    Ok(Json(res))
}

pub fn build_search_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", get(search_reference))
        .route("/full", get(search_reference_full))
        .route("/brief", get(search_reference_breif));

    Router::new().nest("/search", routers)
}
