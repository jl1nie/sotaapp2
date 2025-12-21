use axum::{
    extract::{Multipart, Path, Query},
    http::StatusCode,
    middleware,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{Duration, TimeZone, Utc};
use firebase_auth_sdk::FireAuth;
use shaku_axum::Inject;

use common::error::{AppError, AppResult};
use domain::model::sota::SummitCode;
use domain::model::{
    event::{DeleteRef, FindActBuilder, FindLogBuilder, FindRefBuilder},
    id::UserId,
};
use registry::{AppRegistry, AppState};
use service::model::sota::{UploadSOTALog, UploadSOTASummit, UploadSOTASummitOpt};
use service::services::{AdminService, UserService};

use crate::model::award::{
    ActivatorAwardResult, AwardJudgmentResult, ChaserAwardResult, JudgmentMode, LogType,
    SummitActivation, SummitChase,
};
use crate::model::import::ImportResult;
use crate::model::sota::{PagenatedResponse, SotaRefView, UpdateRefRequest};
use crate::model::{
    activation::ActivationView,
    alerts::AlertView,
    param::{build_findref_query, GetParam},
    spots::SpotView,
};

use super::auth::auth_middle;
use super::multipart::extract_text_file;

async fn update_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_sota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn import_summit_list(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadSOTASummit { data };
    let count = admin_service.import_summit_list(reqs).await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn update_summit_list(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadSOTASummit { data };
    let count = admin_service.update_summit_list(reqs).await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn import_sota_opt_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadSOTASummitOpt { data };
    let count = admin_service.import_summit_opt_list(reqs).await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn upload_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Extension(user_id): Extension<UserId>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadSOTALog { data };
    user_service
        .upload_sota_log(user_id, reqs)
        .await
        .map(|_| StatusCode::CREATED)
}

async fn delete_log(
    user_service: Inject<AppRegistry, dyn UserService>,
    Extension(user_id): Extension<UserId>,
) -> AppResult<StatusCode> {
    user_service
        .delete_sota_log(user_id)
        .await
        .map(|_| StatusCode::OK)
}

async fn show_progress(
    user_service: Inject<AppRegistry, dyn UserService>,
    Extension(user_id): Extension<UserId>,
) -> AppResult<Json<String>> {
    let mut query = FindLogBuilder::default();
    let from = Utc
        .with_ymd_and_hms(2024, 6, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::UnprocessableEntity("無効な日付".to_string()))?;
    let to = Utc
        .with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::UnprocessableEntity("無効な日付".to_string()))?;

    query = query.after(from).before(to);
    let query = query.build();

    let result = user_service.award_progress(user_id, query).await?;
    Ok(Json(result))
}

async fn delete_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(summit_code): Path<String>,
) -> AppResult<StatusCode> {
    let req = DeleteRef::Delete(SummitCode::new(summit_code));
    admin_service
        .delete_sota_reference(req)
        .await
        .map(|_| StatusCode::OK)
}

async fn show_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SotaRefView>> {
    let query = FindRefBuilder::default()
        .sota()
        .sota_code(summit_code)
        .build();
    let result = admin_service.show_sota_reference(query).await?;
    Ok(Json(result.into()))
}

async fn show_all_sota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<PagenatedResponse<SotaRefView>>> {
    let mut query = FindRefBuilder::default()
        .sota()
        .limit(param.limit.unwrap_or(500));

    if let Some(offset) = param.offset {
        query = query.offset(offset);
    }
    let result = admin_service
        .show_all_sota_references(query.build())
        .await?;
    Ok(Json(result.into()))
}

async fn search_sota_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<SotaRefView>>> {
    let query = FindRefBuilder::default().sota();
    let mut query = build_findref_query(param, query)?;

    query.limit = query.limit.map_or(Some(500), |v| Some(v.min(500)));

    let results = user_service.find_references(query).await?;

    let res: Vec<_> = results
        .sota
        .unwrap_or(vec![])
        .into_iter()
        .map(SotaRefView::from)
        .collect();
    Ok(Json(res))
}

async fn show_sota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationView<SpotView>>>> {
    let hours = param.hours_ago.unwrap_or(3);
    let query = FindActBuilder::default()
        .sota()
        .issued_after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_spots(query).await?;
    let spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(SpotView::from).collect::<Vec<_>>()))
        })
        .collect();
    Ok(Json(spots))
}

async fn show_sota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<GetParam>,
) -> AppResult<Json<Vec<ActivationView<AlertView>>>> {
    let hours = param.hours_ago.unwrap_or(3);
    let query = FindActBuilder::default()
        .sota()
        .issued_after(Utc::now() - Duration::hours(hours))
        .build();
    let result = user_service.find_alerts(query).await?;
    let alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(AlertView::from).collect::<Vec<_>>()))
        })
        .collect();
    Ok(Json(alerts))
}

/// アワード判定クエリパラメータ
#[derive(Debug, serde::Deserialize, Default)]
pub struct AwardJudgeQuery {
    /// 判定モード: strict（デフォルト）または lenient
    #[serde(default)]
    pub mode: Option<String>,
}

/// SOTA日本支部設立10周年記念アワード判定エンドポイント
/// CSVをアップロードしてin-memoryで判定、結果を返す（DBに保存しない）
async fn judge_10th_anniversary_award(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(query): Query<AwardJudgeQuery>,
    mut multipart: Multipart,
) -> AppResult<Json<AwardJudgmentResult>> {
    // モードを解析（デフォルトは厳格モード）
    let api_mode = match query.mode.as_deref() {
        Some("lenient") => JudgmentMode::Lenient,
        _ => JudgmentMode::Strict,
    };

    // API層のモードをサービス層のモードに変換
    let service_mode = match api_mode {
        JudgmentMode::Strict => service::model::award::JudgmentMode::Strict,
        JudgmentMode::Lenient => service::model::award::JudgmentMode::Lenient,
    };

    let data = extract_text_file(&mut multipart).await?;

    // in-memoryで判定（モード指定）
    let result = user_service.judge_10th_anniversary_award(&data, service_mode)?;

    // サービス層のログタイプをAPI層の型に変換
    let log_type = match result.log_type {
        service::model::award::LogType::Activator => LogType::Activator,
        service::model::award::LogType::Chaser => LogType::Chaser,
        service::model::award::LogType::Unknown => LogType::Unknown,
    };

    // サービス層の結果をAPI層の型に変換
    let response = AwardJudgmentResult {
        success: true,
        callsign: result.callsign,
        total_qsos: result.total_qsos,
        log_type,
        activator: result.activator.map(|a| ActivatorAwardResult {
            achieved: a.achieved,
            qualified_summits: a.qualified_summits,
            summits: a
                .summits
                .into_iter()
                .map(|s| SummitActivation {
                    summit_code: s.summit_code,
                    unique_stations: s.unique_stations,
                    qualified: s.qualified,
                })
                .collect(),
        }),
        chaser: result.chaser.map(|c| ChaserAwardResult {
            achieved: c.achieved,
            qualified_summits: c
                .qualified_summits
                .into_iter()
                .map(|s| SummitChase {
                    summit_code: s.summit_code,
                    unique_activators: s.unique_activators,
                    activators: s.activators,
                })
                .collect(),
        }),
        mode: api_mode,
    };

    Ok(Json(response))
}

pub fn build_sota_routers(auth: &FireAuth) -> Router<AppState> {
    let protected = Router::new()
        .route("/import", post(import_summit_list))
        .route("/import/ja", post(import_sota_opt_reference))
        .route("/log", post(upload_log))
        .route("/log", delete(delete_log))
        .route("/log", get(show_progress))
        .route("/update", post(update_summit_list))
        .route("/summits/{summit_code}", put(update_sota_reference))
        .route("/summits/{summit_code}", delete(delete_sota_reference))
        .route_layer(middleware::from_fn_with_state(auth.clone(), auth_middle));

    let public = Router::new()
        .route("/spots", get(show_sota_spots))
        .route("/alerts", get(show_sota_alerts))
        .route("/summits", get(show_all_sota_reference))
        .route("/summits/{summit_code}", get(show_sota_reference))
        .route("/summits/search", get(search_sota_reference))
        .route("/award/10th-anniversary/judge", post(judge_10th_anniversary_award));

    let routers = Router::new().merge(protected).merge(public);

    Router::new().nest("/sota", routers)
}
