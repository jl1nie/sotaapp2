//! 山頂の地図と POTA の判定の見直し API(設計 9.1)。admin と reviewer が使える
use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use common::error::{AppError, AppResult};
use registry::{AppRegistry, AppState};
use serde::Deserialize;
use service::model::summit_review::{ConfirmSummit, StatusFilter};
use service::services::SummitReviewService;
use shaku_axum::Inject;

use super::auth::{with_review_auth, AdminAuth, AdminIdentity};
use super::sota::summit_prefix;
use crate::model::summit_review::{ConfirmRequest, SummitReviewDetailView, SummitReviewListView};

#[derive(Debug, Deserialize)]
struct ListParam {
    prefix: Option<String>,
    status: Option<String>,
    q: Option<String>,
}

async fn list_summits(
    service: Inject<AppRegistry, dyn SummitReviewService>,
    Query(param): Query<ListParam>,
) -> AppResult<Json<SummitReviewListView>> {
    let prefix = param
        .prefix
        .as_deref()
        .and_then(summit_prefix)
        .ok_or_else(|| {
            AppError::BadRequest("prefix は JA か JA/KN のように指定してください".to_string())
        })?;
    let filter = match param.status.as_deref().filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(
            StatusFilter::parse(s)
                .ok_or_else(|| AppError::BadRequest(format!("status を読めません: {s}")))?,
        ),
    };
    let list = service
        .list_summits(&prefix, filter, param.q.as_deref())
        .await?;
    Ok(Json(list.into()))
}

async fn show_summit(
    service: Inject<AppRegistry, dyn SummitReviewService>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SummitReviewDetailView>> {
    Ok(Json(service.show_summit(&summit_code).await?.into()))
}

async fn confirm_summit(
    service: Inject<AppRegistry, dyn SummitReviewService>,
    Extension(who): Extension<AdminIdentity>,
    Path(summit_code): Path<String>,
    Json(body): Json<ConfirmRequest>,
) -> AppResult<Json<SummitReviewDetailView>> {
    let answers = body.verdicts();
    let detail = service
        .confirm_summit(ConfirmSummit {
            summit_code,
            input_rev: body.input_rev,
            answers,
            note: body.note.filter(|n| !n.trim().is_empty()),
            reviewed_by: who.email,
        })
        .await?;
    Ok(Json(detail.into()))
}

async fn cancel_review(
    service: Inject<AppRegistry, dyn SummitReviewService>,
    Extension(who): Extension<AdminIdentity>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SummitReviewDetailView>> {
    Ok(Json(
        service
            .cancel_review(&summit_code, &who.email)
            .await?
            .into(),
    ))
}

/// `/sota` の下に入れる。山頂コードは `/` を含むので、クライアントは `JA%2FKN-017` と送る
pub fn build_review_routers(auth: &AdminAuth) -> Router<AppState> {
    with_review_auth(
        Router::new()
            .route("/review/summits", get(list_summits))
            .route(
                "/review/summits/{summit_code}",
                get(show_summit).put(confirm_summit).delete(cancel_review),
            ),
        auth,
    )
}
