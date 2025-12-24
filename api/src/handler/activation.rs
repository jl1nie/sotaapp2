use aprs_message::AprsCallsign;
use axum::{routing::get, Json, Router};
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
    param::{GetParam, ValidatedQuery},
    spots::SpotView,
};

/// キャッシュTTL定数
const CACHE_TTL_SPOTS: i64 = 30;
const CACHE_TTL_ALERTS: i64 = 180;
const CACHE_TTL_TRACK: i64 = 60;

/// パラメータからFindActBuilderにグルーピングとフィルタを適用
fn apply_common_filters(
    param: &GetParam,
    mut query: FindActBuilder,
    default_hours: i64,
) -> FindActBuilder {
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

    let value =
        serde_json::to_value(spots).map_err(|e| AppError::ConversionEntityError(e.to_string()))?;
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

    let value =
        serde_json::to_value(alerts).map_err(|e| AppError::ConversionEntityError(e.to_string()))?;
    kvs_repo
        .set(
            key,
            value.clone(),
            Some(Duration::seconds(CACHE_TTL_ALERTS)),
        )
        .await;

    Ok(Json(value))
}

async fn show_sota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().sota();
    show_spots(user_service, kvs_repo, param, query).await
}

async fn show_pota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().pota();
    show_spots(user_service, kvs_repo, param, query).await
}

async fn show_all_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default();
    show_spots(user_service, kvs_repo, param, query).await
}

async fn show_sota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().sota();
    show_alerts(user_service, kvs_repo, param, query).await
}

async fn show_pota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default().pota();
    show_alerts(user_service, kvs_repo, param, query).await
}

async fn show_all_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    kvs_repo: Inject<AppRegistry, dyn KvsRepositry>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Value>> {
    let query = FindActBuilder::default();
    show_alerts(user_service, kvs_repo, param, query).await
}

async fn show_aprs_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
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
    ValidatedQuery(param): ValidatedQuery<GetParam>,
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
    let value =
        serde_json::to_value(value).map_err(|e| AppError::ConversionEntityError(e.to_string()))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::param::GetParam;
    use chrono::{Duration, Utc};
    use domain::model::event::{FindActBuilder, GroupBy};

    // =====================================================
    // apply_common_filters テスト
    // =====================================================

    #[test]
    fn test_apply_common_filters_default_grouping() {
        let param = GetParam::default();
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // デフォルトはCallsign(None)でグルーピング
        assert!(matches!(query.group_by, Some(GroupBy::Callsign(None))));
    }

    #[test]
    fn test_apply_common_filters_group_by_callsign() {
        let param = GetParam {
            by_call: Some("JA1ABC".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        assert!(matches!(
            query.group_by,
            Some(GroupBy::Callsign(Some(ref s))) if s == "JA1ABC"
        ));
    }

    #[test]
    fn test_apply_common_filters_group_by_callsign_null() {
        let param = GetParam {
            by_call: Some("null".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // "null"で始まる場合はCallsign(None)
        assert!(matches!(query.group_by, Some(GroupBy::Callsign(None))));
    }

    #[test]
    fn test_apply_common_filters_group_by_reference() {
        let param = GetParam {
            by_ref: Some("JA/TK-001".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        assert!(matches!(
            query.group_by,
            Some(GroupBy::Reference(Some(ref s))) if s == "JA/TK-001"
        ));
    }

    #[test]
    fn test_apply_common_filters_group_by_reference_null() {
        let param = GetParam {
            by_ref: Some("null".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        assert!(matches!(query.group_by, Some(GroupBy::Reference(None))));
    }

    #[test]
    fn test_apply_common_filters_by_call_takes_priority() {
        // by_call と by_ref の両方が指定された場合、by_callが優先
        let param = GetParam {
            by_call: Some("JA1ABC".to_string()),
            by_ref: Some("JA/TK-001".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // by_callが優先されるのでCallsign
        assert!(matches!(query.group_by, Some(GroupBy::Callsign(_))));
    }

    #[test]
    fn test_apply_common_filters_default_hours() {
        let param = GetParam::default();
        let builder = FindActBuilder::default();
        let now = Utc::now();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // デフォルトの3時間が適用される
        assert!(query.issued_after.is_some());
        let after = query.issued_after.unwrap();
        // 3時間前より少し前（テスト実行時間を考慮）
        assert!(after > now - Duration::hours(4));
        assert!(after < now - Duration::hours(2));
    }

    #[test]
    fn test_apply_common_filters_custom_hours() {
        let param = GetParam {
            hours_ago: Some(24),
            ..Default::default()
        };
        let builder = FindActBuilder::default();
        let now = Utc::now();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        assert!(query.issued_after.is_some());
        let after = query.issued_after.unwrap();
        // 24時間前
        assert!(after > now - Duration::hours(25));
        assert!(after < now - Duration::hours(23));
    }

    #[test]
    fn test_apply_common_filters_pattern() {
        let param = GetParam {
            pat_ref: Some("JA/".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        assert_eq!(query.pattern, Some("JA/".to_string()));
    }

    #[test]
    fn test_apply_common_filters_log_id_valid_uuid() {
        // 有効なUUID形式のlog_id
        let param = GetParam {
            log_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // 有効なUUIDはLogId型に変換される
        assert!(query.log_id.is_some());
    }

    #[test]
    fn test_apply_common_filters_log_id_invalid() {
        // 無効なlog_id形式
        let param = GetParam {
            log_id: Some("invalid-log-id".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // 無効なUUIDはlog_idに設定されない（エラーログは出力される）
        assert!(query.log_id.is_none());
    }

    #[test]
    fn test_apply_common_filters_preserves_sota() {
        let param = GetParam::default();
        let builder = FindActBuilder::default().sota();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // SOTA設定が維持される
        assert!(matches!(
            query.program,
            Some(domain::model::AwardProgram::SOTA)
        ));
    }

    #[test]
    fn test_apply_common_filters_preserves_pota() {
        let param = GetParam::default();
        let builder = FindActBuilder::default().pota();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        // POTA設定が維持される
        assert!(matches!(
            query.program,
            Some(domain::model::AwardProgram::POTA)
        ));
    }

    #[test]
    fn test_apply_common_filters_all_options() {
        let param = GetParam {
            by_call: Some("JA1ABC".to_string()),
            hours_ago: Some(12),
            pat_ref: Some("JA/TK".to_string()),
            log_id: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            ..Default::default()
        };
        let builder = FindActBuilder::default().sota();

        let result = apply_common_filters(&param, builder, 3);
        let query = result.build();

        assert!(query.group_by.is_some());
        assert!(query.issued_after.is_some());
        assert_eq!(query.pattern, Some("JA/TK".to_string()));
        assert!(query.log_id.is_some());
        assert!(matches!(
            query.program,
            Some(domain::model::AwardProgram::SOTA)
        ));
    }
}
