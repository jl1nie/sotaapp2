use axum::{extract::Query, routing::get, Json, Router};

use domain::model::common::event::FindResult;
use shaku_axum::Inject;
use std::str::FromStr;
use std::time::Instant;
use std::vec;

use common::error::AppResult;

use domain::model::common::{event::FindRefBuilder, id::UserId};

use registry::{AppRegistry, AppState};
use service::services::UserService;

use crate::model::param::GetParam;

use crate::model::search::{SearchBriefResponse, SerachFullResponse};

async fn search(
    user_service: Inject<AppRegistry, dyn UserService>,
    param: GetParam,
) -> AppResult<FindResult> {
    let start = Instant::now();
    let mut query = FindRefBuilder::default().sota().pota();

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

    if param.user_id.is_some() {
        query = query.user_id(UserId::from_str(&param.user_id.unwrap())?);
    }

    if param.min_area.is_some() {
        query = query.min_area(param.min_area.unwrap());
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

    let results = user_service.find_references(query.build()).await?;
    let end = start.elapsed();
    tracing::info!("find reference {}ms", end.as_millis());
    Ok(results)
}

async fn search_all_reference_full(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<SerachFullResponse>> {
    let results = search(user_service, param).await?;
    Ok(Json(results.into()))
}

async fn search_all_reference_breif(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<SearchBriefResponse>> {
    let max_count = param.max_count;
    let results = search(user_service, param).await?;
    let mut res: SearchBriefResponse = results.into();

    if res.count > max_count.unwrap_or_default() {
        res.candidates = vec![];
    }

    Ok(Json(res))
}

pub fn build_search_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", get(search_all_reference_full))
        .route("/brief", get(search_all_reference_breif));

    Router::new().nest("/search", routers)
}
