use aprs_message::AprsCallsign;
use axum::{extract::Query, routing::get, Json, Router};
use chrono::{Duration, Utc};
use common::error::{AppError, AppResult};
use serde_json::Value;
use shaku_axum::Inject;

use domain::model::event::{FindActBuilder, FindAprs};

use domain::repository::minikvs::KvsRepositry;
use registry::{AppRegistry, AppState};
use service::services::UserService;

use crate::model::{
    activation::ActivationView,
    alerts::AlertView,
    aprslog::{AprsLogView, Track, Tracks},
    param::GetParam,
    spots::SpotView,
};

/// キャッシュTTL定数
const CACHE_TTL_SPOTS: i64 = 30;
const CACHE_TTL_ALERTS: i64 = 180;
const CACHE_TTL_TRACK: i64 = 60;

/// パラメータからFindActBuilderにグルーピングとフィルタを適用
fn apply_common_filters(param: &GetParam, mut query: FindActBuilder, default_hours: i64) -> FindActBuilder {
    // グルーピング設定
    if let Some(callsign) = &param.by_call {
        if callsign.starts_with("null") {
            query = query.group_by_callsign(None)
        } else {
            query = query.group_by_callsign(Some(callsign.clone()))
        }
    } else if let Some(reference) = &param.by_ref {
        if reference.starts_with("null") {
            query = query.group_by_reference(None)
        } else {
            query = query.group_by_reference(Some(reference.clone()))
        }
    } else {
        query = query.group_by_callsign(None)
    }

    // 時間フィルタ
    let hours = param.hours_ago.unwrap_or(default_hours);
    query = query.issued_after(Utc::now() - Duration::hours(hours));

    // パターンフィルタ
    if let Some(pat) = &param.pat_ref {
        query = query.pattern(pat);
    }

    // ログIDフィルタ
    if let Some(log_id) = &param.log_id {
        query = query.log_id(log_id);
    }

    query
}

async fn show_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    param: GetParam,
    query: FindActBuilder,
) -> AppResult<Json<Value>> {
    let key = param.to_key();
    if let Some(val) = kvs_repo.get(&key).await {
        return Ok(Json(val));
    };

    let query = apply_common_filters(&param, query, 3).build();

    let result = user_service.find_spots(query).await?;
    let mut spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(SpotView::from).collect::<Vec<_>>()))
        })
        .collect();

    spots.sort_by(|a, b| a.key.cmp(&b.key));

    let value = serde_json::to_value(spots)
        .map_err(|e| AppError::ConversionEntityError(e.to_string()))?;
    kvs_repo
        .set(key, value.clone(), Some(Duration::seconds(CACHE_TTL_SPOTS)))
        .await;

    Ok(Json(value))
}

async fn show_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    param: GetParam,
    query: FindActBuilder,
) -> AppResult<Json<Value>> {
    let key = param.to_key();
    if let Some(val) = kvs_repo.get(&key).await {
        return Ok(Json(val));
    };

    let query = apply_common_filters(&param, query, 24).build();

    let result = user_service.find_alerts(query).await?;
    let mut alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(AlertView::from).collect::<Vec<_>>()))
        })
        .collect();

    alerts.sort_by(|a, b| a.key.cmp(&b.key));

    let value = serde_json::to_value(alerts)
        .map_err(|e| AppError::ConversionEntityError(e.to_string()))?;
    kvs_repo
        .set(key, value.clone(), Some(Duration::seconds(CACHE_TTL_ALERTS)))
        .await;

    Ok(Json(value))
}

async fn show_sota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().sota();
    show_spots(user_service, kvs_repo, param, query).await
}

async fn show_pota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().pota();
    show_spots(user_service, kvs_repo, param, query).await
}

async fn show_all_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default();
    show_spots(user_service, kvs_repo, param, query).await
}

async fn show_sota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().sota();
    show_alerts(user_service, kvs_repo, param, query).await
}

async fn show_pota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().pota();
    show_alerts(user_service, kvs_repo, param, query).await
}

async fn show_all_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default();
    show_alerts(user_service, kvs_repo, param, query).await
}

async fn show_aprs_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<AprsLogView>>> {
    let mut request = FindAprs {
        reference: None,
        callsign: None,
        after: None,
    };

    if let Some(callsign) = param.by_call {
        request.callsign = Some(AprsCallsign::from(callsign));
    } else {
        if let Some(ago) = param.hours_ago {
            request.after = Some(Utc::now() - Duration::hours(ago));
        }
        if let Some(pat) = param.pat_ref {
            request.reference = Some(pat);
        }
    }

    let result = user_service
        .find_aprs_log(request)
        .await?
        .into_iter()
        .map(AprsLogView::from)
        .collect::<Vec<_>>();

    Ok(Json(result))
}

async fn show_aprs_track(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Value>> {
    let key = param.to_key();
    if let Some(val) = kvs_repo.get(&key).await {
        return Ok(Json(val));
    };

    let request = FindAprs {
        reference: param.pat_ref,
        callsign: None,
        after: Some(Utc::now() - Duration::hours(param.hours_ago.unwrap_or(24))),
    };

    let tracks = user_service.get_aprs_track(request).await?;
    let tracks = tracks.into_iter().map(Track::from).collect();
    let value = Tracks { tracks };
    let value = serde_json::to_value(value)
        .map_err(|e| AppError::ConversionEntityError(e.to_string()))?;

    kvs_repo
        .set(key, value.clone(), Some(Duration::seconds(CACHE_TTL_TRACK)))
        .await;

    Ok(Json(value))
}

pub fn build_activation_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/alerts", get(show_all_alerts))
        .route("/alerts/sota", get(show_sota_alerts))
        .route("/alerts/pota", get(show_pota_alerts))
        .route("/spots", get(show_all_spots))
        .route("/spots/sota", get(show_sota_spots))
        .route("/spots/pota", get(show_pota_spots))
        .route("/aprs/log", get(show_aprs_log))
        .route("/aprs/track", get(show_aprs_track));
    Router::new().nest("/activation", routers)
}
