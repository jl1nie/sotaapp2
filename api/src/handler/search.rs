use axum::{routing::get, Json, Router};
use shaku_axum::Inject;
use utoipa::OpenApi;

use crate::model::param::{build_findref_query, GetParam, ValidatedQuery};
use crate::model::search::{SearchBriefResponse, SearchFullResponse, SearchResponse};
use common::error::AppResult;
use domain::model::event::{FindRefBuilder, FindResult};
use registry::{AppRegistry, AppState};
use service::services::UserService;

/// Search API
#[derive(OpenApi)]
#[openapi(
    paths(search_reference, search_reference_full, search_reference_breif),
    components(schemas(
        GetParam,
        SearchResponse,
        SearchFullResponse,
        SearchBriefResponse,
        crate::model::search::SearchBriefData,
        crate::model::sota::SotaSearchView,
        crate::model::sota::SotaRefView,
        crate::model::pota::PotaSearchView,
        crate::model::pota::PotaRefLogView,
    )),
    tags((name = "search", description = "SOTA/POTA リファレンス検索API"))
)]
pub struct SearchApi;

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

/// SOTA/POTAリファレンス検索
#[utoipa::path(
    get,
    path = "/api/v2/search",
    params(GetParam),
    responses(
        (status = 200, description = "検索成功", body = SearchResponse),
        (status = 400, description = "無効なパラメータ"),
    ),
    tag = "search"
)]
async fn search_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<SearchResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

/// SOTA/POTAリファレンス検索（詳細）
#[utoipa::path(
    get,
    path = "/api/v2/search/full",
    params(GetParam),
    responses(
        (status = 200, description = "検索成功", body = SearchFullResponse),
        (status = 400, description = "無効なパラメータ"),
    ),
    tag = "search"
)]
async fn search_reference_full(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<SearchFullResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

/// SOTA/POTAリファレンス検索（簡易）
#[utoipa::path(
    get,
    path = "/api/v2/search/brief",
    params(GetParam),
    responses(
        (status = 200, description = "検索成功", body = SearchBriefResponse),
        (status = 400, description = "無効なパラメータ"),
    ),
    tag = "search"
)]
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
