use aprs_message::AprsCallsign;
use axum::{extract::Query, routing::get, Json, Router};
use chrono::{Duration, Utc};
use common::error::AppResult;
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

async fn show_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    param: GetParam,
    mut query: FindActBuilder,
) -> AppResult<Json<Value>> {
    if let Some(callsign) = param.by_call {
        if callsign.starts_with("null") {
            query = query.group_by_callsign(None)
        } else {
            query = query.group_by_callsign(Some(callsign))
        }
    } else if let Some(reference) = param.by_ref {
        if reference.starts_with("null") {
            query = query.group_by_reference(None)
        } else {
            query = query.group_by_reference(Some(reference))
        }
    } else {
        query = query.group_by_callsign(None)
    }

    let hours = param.hours_ago.unwrap_or(3);
    query = query.issued_after(Utc::now() - Duration::hours(hours));

    if let Some(pat) = param.pat_ref {
        query = query.pattern(&pat);
    }

    let query = query.build();
    let key = query.to_key();
    if let Some(val) = kvs_repo.get(&key).await {
        tracing::info!("show_spots cache hit");
        return Ok(Json(val));
    };

    let result = user_service.find_spots(query).await?;
    let spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(SpotView::from).collect::<Vec<_>>()))
        })
        .collect();

    let value = serde_json::to_value(spots).unwrap();
    kvs_repo
        .set(key, value.clone(), Some(Duration::seconds(30)))
        .await;

    Ok(Json(value))
}

async fn show_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    param: GetParam,
    mut query: FindActBuilder,
) -> AppResult<Json<Value>> {
    if let Some(callsign) = param.by_call {
        if callsign.starts_with("null") {
            query = query.group_by_callsign(None)
        } else {
            query = query.group_by_callsign(Some(callsign))
        }
    } else if let Some(reference) = param.by_ref {
        if reference.starts_with("null") {
            query = query.group_by_reference(None)
        } else {
            query = query.group_by_reference(Some(reference))
        }
    } else {
        query = query.group_by_callsign(None)
    }

    if let Some(pat) = param.pat_ref {
        query = query.pattern(&pat);
    }

    let hours = param.hours_ago.unwrap_or(24);
    query = query.issued_after(Utc::now() - Duration::hours(hours));

    let query = query.build();
    let key = query.to_key();
    if let Some(val) = kvs_repo.get(&key).await {
        tracing::info!("show_alerts cache hit");
        return Ok(Json(val));
    };

    let result = user_service.find_alerts(query).await?;
    let alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(AlertView::from).collect::<Vec<_>>()))
        })
        .collect();

    let value = serde_json::to_value(alerts).unwrap();
    kvs_repo
        .set(key, value.clone(), Some(Duration::seconds(180)))
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
        request.callsign = Some(AprsCallsign::from(&callsign));
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
    let request = FindAprs {
        reference: param.pat_ref,
        callsign: None,
        after: Some(Utc::now() - Duration::hours(param.hours_ago.unwrap_or(24))),
    };

    let key = request.to_key();
    if let Some(val) = kvs_repo.get(&key).await {
        tracing::info!("show_aprs_track cache hit");
        return Ok(Json(val));
    };

    let tracks = user_service.get_aprs_track(request).await?;
    let tracks = tracks.into_iter().map(Track::from).collect();
    let value = Tracks { tracks };
    let value = serde_json::to_value(value).unwrap();

    kvs_repo
        .set(key, value.clone(), Some(Duration::seconds(60)))
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
