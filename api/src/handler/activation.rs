use axum::{extract::Query, routing::get, Json, Router};

use chrono::{Duration, Utc};
use shaku_axum::Inject;

use common::error::AppResult;

use domain::model::event::{FindActBuilder, FindAprs};

use registry::{AppRegistry, AppState};
use service::services::UserService;

use crate::model::{
    activation::ActivationResponse, alerts::AlertResponse, aprslog::AprsLogResponse,
    param::GetParam, spots::SpotResponse,
};

async fn show_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    param: GetParam,
    mut query: FindActBuilder,
) -> AppResult<Json<Vec<ActivationResponse<SpotResponse>>>> {
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

    let result = user_service.find_spots(query.build()).await?;
    let spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationResponse::from((k, v.into_iter().map(SpotResponse::from).collect::<Vec<_>>()))
        })
        .collect();
    Ok(Json(spots))
}

async fn show_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    param: GetParam,
    mut query: FindActBuilder,
) -> AppResult<Json<Vec<ActivationResponse<AlertResponse>>>> {
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

    let hours = param.hours_ago.unwrap_or(24);
    query = query.issued_after(Utc::now() - Duration::hours(hours));

    if let Some(pat) = param.pat_ref {
        query = query.pattern(&pat);
    }

    let result = user_service.find_alerts(query.build()).await?;
    let alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationResponse::from((
                k,
                v.into_iter().map(AlertResponse::from).collect::<Vec<_>>(),
            ))
        })
        .collect();
    Ok(Json(alerts))
}

async fn show_sota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationResponse<SpotResponse>>>> {
    let query = FindActBuilder::default().sota();
    show_spots(user_service, param, query).await
}

async fn show_pota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationResponse<SpotResponse>>>> {
    let query = FindActBuilder::default().pota();
    show_spots(user_service, param, query).await
}

async fn show_all_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationResponse<SpotResponse>>>> {
    let query = FindActBuilder::default();
    show_spots(user_service, param, query).await
}

async fn show_sota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationResponse<AlertResponse>>>> {
    let query = FindActBuilder::default().sota();
    show_alerts(user_service, param, query).await
}

async fn show_pota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationResponse<AlertResponse>>>> {
    let query = FindActBuilder::default().pota();
    show_alerts(user_service, param, query).await
}

async fn show_all_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationResponse<AlertResponse>>>> {
    let query = FindActBuilder::default();
    show_alerts(user_service, param, query).await
}

async fn show_aprs_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<AprsLogResponse>>> {
    let mut request = FindAprs {
        callsign: None,
        after: None,
    };

    if param.by_call.is_some() {
        request.callsign = param.by_call;
    } else if param.hours_ago.is_some() {
        request.after = Some(Utc::now() - Duration::hours(param.hours_ago.unwrap()));
    }

    let result = user_service
        .find_aprslog(request)
        .await?
        .into_iter()
        .map(AprsLogResponse::from)
        .collect::<Vec<_>>();

    Ok(Json(result))
}

pub fn build_activation_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/alerts", get(show_all_alerts))
        .route("/alerts/sota", get(show_sota_alerts))
        .route("/alerts/pota", get(show_pota_alerts))
        .route("/spots", get(show_all_spots))
        .route("/spots/sota", get(show_sota_spots))
        .route("/spots/pota", get(show_pota_spots))
        .route("/aprslog", get(show_aprs_log));
    Router::new().nest("/activation", routers)
}
