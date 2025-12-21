use axum::{routing::get, Json, Router};
use shaku_axum::Inject;

use crate::model::param::{build_findref_query, GetParam, ValidatedQuery};
use crate::model::search::{SearchBriefResponse, SearchFullResponse, SearchResponse};
use common::error::AppResult;
use domain::model::event::{FindRefBuilder, FindResult};
use registry::{AppRegistry, AppState};
use service::services::UserService;

async fn search(
    user_service: Inject<AppRegistry, dyn UserService>,
    param: GetParam,
) -> AppResult<FindResult> {
    let query = FindRefBuilder::default().sota().pota();
    let mut query = build_findref_query(param, query)?;

    query.limit = query.limit.map_or(Some(500), |v| Some(v.min(500)));

    let results = user_service.find_references(query).await?;
    Ok(results)
}

async fn search_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<SearchResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

async fn search_reference_full(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<SearchFullResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

async fn search_reference_breif(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<SearchBriefResponse>> {
    let maxcount = param.max_count.unwrap_or(100);

    let query = FindRefBuilder::default().sota().pota();
    let query = build_findref_query(param.clone(), query)?;
    let count = user_service.count_references(&query).await? as u32;

    let mut res = SearchBriefResponse {
        count,
        candidates: Vec::new(),
    };

    if count < maxcount {
        res = search(user_service, param).await?.into();
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
