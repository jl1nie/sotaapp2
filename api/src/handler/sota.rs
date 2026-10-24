use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use fireauth::FireAuth;
use shaku_axum::Inject;

use common::award_config::AwardTemplateConfig;
use common::error::AppResult;
use domain::model::sota::SummitCode;
use domain::model::{
    event::{DeleteRef, FindActBuilder, FindRefBuilder},
    id::UserId,
};
use registry::{AppRegistry, AppState};
use service::implement::award_pdf::{AwardPdfGenerator, AwardType, CertificateInfo};
use service::model::sota::{
    UploadSOTALog, UploadSOTASummit, UploadSOTASummitOpt, UploadSummitMaps, UploadSummitPota,
};
use service::services::{AdminService, SotaLogService, UserService};
use std::path::PathBuf;
use tower_http::compression::CompressionLayer;

use crate::model::award::{
    ActivatorAwardResult, AwardJudgmentResult, ChaserAwardResult, JudgmentMode, LogType,
    SummitActivation, SummitChase,
};
use crate::model::import::ImportResult;
use crate::model::sota::{PagenatedResponse, SotaRefView, SummitPotaView, UpdateRefRequest};
use crate::model::{
    activation::ActivationView,
    alerts::AlertView,
    param::{build_findref_query, GetParam, ValidatedQuery},
    spots::SpotView,
};

use super::auth::with_auth;
use super::multipart::{extract_file, extract_text_file};

async fn export_sota_reference_ja(
    admin_service: Inject<AppRegistry, dyn AdminService>,
) -> AppResult<Response> {
    let csv = admin_service.export_sota_summits_csv().await?;
    let filename = format!("ja_summits_{}.csv", Utc::now().format("%Y%m%d"));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        csv,
    )
        .into_response())
}

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

async fn import_summit_pota(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_text_file(&mut multipart).await?;
    let count = admin_service
        .import_summit_pota(UploadSummitPota { data })
        .await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn import_summit_maps(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_file(&mut multipart).await?;
    let count = admin_service
        .import_summit_maps(UploadSummitMaps { data })
        .await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn show_summit_pota(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(summit_code): Path<String>,
) -> AppResult<Json<SummitPotaView>> {
    let rows = user_service
        .find_summit_pota(SummitCode::new(summit_code.clone()))
        .await?;
    Ok(Json(SummitPotaView::new(summit_code, rows)))
}

fn geojson_response(body: String) -> Response {
    (
        [(header::CONTENT_TYPE, "application/geo+json; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn show_summit_map(
    user_service: Inject<AppRegistry, dyn UserService>,
    Path(summit_code): Path<String>,
) -> AppResult<Response> {
    match user_service
        .find_summit_map(SummitCode::new(summit_code))
        .await?
    {
        Some(m) => Ok(geojson_response(m.geojson)),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[derive(Debug, serde::Deserialize)]
struct MapsParam {
    prefix: String,
}

/// 地域ごとの簡易地図(端末にまとめて保存する用)。prefix は "JA/KN" のように地域まで指定する
async fn show_summit_maps(
    user_service: Inject<AppRegistry, dyn UserService>,
    Query(param): Query<MapsParam>,
) -> AppResult<Response> {
    let Some(prefix) = region_prefix(&param.prefix) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            "prefix は JA/KN のように地域まで指定してください",
        )
            .into_response());
    };
    let maps = user_service.find_summit_maps(&prefix).await?;
    // 保存してある GeoJSON をそのまま並べる(読み直して組み立て直さない)
    let mut body =
        String::with_capacity(maps.iter().map(|m| m.geojson.len() + 1).sum::<usize>() + 2);
    body.push('[');
    for (i, m) in maps.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&m.geojson);
    }
    body.push(']');
    Ok((
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
        .into_response())
}

/// "JA/KN" や "JA/KN-0" を受け付け、山頂コードの前方一致に使う形("JA/KN-")にする
fn region_prefix(s: &str) -> Option<String> {
    let (assoc, rest) = s.split_once('/')?;
    let (region, num) = rest.split_once('-').unwrap_or((rest, ""));
    let ok = !assoc.is_empty()
        && assoc
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && region.len() == 2
        && region.chars().all(|c| c.is_ascii_uppercase())
        && num.chars().all(|c| c.is_ascii_digit());
    ok.then(|| format!("{}/{}-{}", assoc, region, num))
}

async fn upload_log(
    sota_log_service: Inject<AppRegistry, dyn SotaLogService>,
    Extension(user_id): Extension<UserId>,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadSOTALog { data };
    sota_log_service
        .upload_sota_log(user_id, reqs)
        .await
        .map(|_| StatusCode::CREATED)
}

async fn delete_log(
    sota_log_service: Inject<AppRegistry, dyn SotaLogService>,
    Extension(user_id): Extension<UserId>,
) -> AppResult<StatusCode> {
    sota_log_service
        .delete_sota_log(user_id)
        .await
        .map(|_| StatusCode::OK)
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
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<PagenatedResponse<SotaRefView>>> {
    let limit = param.limit.unwrap_or(500);
    let offset = param.offset.unwrap_or(0);
    let query_param = GetParam {
        limit: Some(limit),
        offset: Some(offset),
        name: param.name,
        sota_code: param.sota_code,
        association: param.association,
        ..Default::default()
    };
    let query = build_findref_query(query_param, FindRefBuilder::default().sota())?;
    let result = admin_service.show_all_sota_references(query).await?;
    Ok(Json(result.into()))
}

async fn search_sota_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
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
    ValidatedQuery(param): ValidatedQuery<GetParam>,
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
    ValidatedQuery(param): ValidatedQuery<GetParam>,
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
    State(state): State<AppState>,
    sota_log_service: Inject<AppRegistry, dyn SotaLogService>,
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
    let result = sota_log_service.judge_10th_anniversary_award(&data, service_mode)?;

    // サービス層のログタイプをAPI層の型に変換
    let log_type = match result.log_type {
        service::model::award::LogType::Activator => LogType::Activator,
        service::model::award::LogType::Chaser => LogType::Chaser,
        service::model::award::LogType::Unknown => LogType::Unknown,
    };

    // PDF証明書が利用可能かチェック
    let template_dir = PathBuf::from(&state.config.award_template_dir);
    let pdf_available = match log_type {
        LogType::Activator => {
            let achieved = result.activator.as_ref().is_some_and(|a| a.achieved);
            let template_exists = template_dir.join("activator_template.pdf").exists();
            achieved && template_exists
        }
        LogType::Chaser => {
            let achieved = result.chaser.as_ref().is_some_and(|c| c.achieved);
            let template_exists = template_dir.join("chaser_template.pdf").exists();
            achieved && template_exists
        }
        LogType::Unknown => false,
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
        pdf_available: Some(pdf_available),
    };

    Ok(Json(response))
}

/// PDF証明書生成リクエスト
#[derive(Debug, serde::Deserialize)]
pub struct GeneratePdfQuery {
    /// アワード種別: activator または chaser
    pub award_type: String,
    /// コールサイン
    pub callsign: String,
    /// 達成サミット数
    pub summits: u32,
    /// チェイサー用: 最多アクティベータ数（Best）
    pub best_activators: Option<u32>,
    /// チェイサー用: 最多アクティベータのサミットコード
    pub best_summit: Option<String>,
}

/// PDF証明書生成エンドポイント
async fn generate_award_pdf(
    State(state): State<AppState>,
    Query(query): Query<GeneratePdfQuery>,
) -> impl IntoResponse {
    let award_type = match query.award_type.as_str() {
        "activator" => AwardType::Activator,
        "chaser" => AwardType::Chaser,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "無効なアワード種別です（activator または chaser）"
                })),
            )
                .into_response()
        }
    };

    // 設定読み込み
    let config_path = PathBuf::from(&state.config.award_config_path);
    let config = match AwardTemplateConfig::load_from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("設定の読み込みに失敗: {}", e)
                })),
            )
                .into_response()
        }
    };

    // 発行日を生成（固定発行日が設定されていればそれを使用）
    let issue_date = config
        .fixed_issue_date
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().format("%Y %b. %-d").to_string());

    let generator = AwardPdfGenerator::new(state.config.award_template_dir.clone(), config);

    // テンプレート存在チェック
    if !generator.template_exists(award_type) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "テンプレートが設定されていません"
            })),
        )
            .into_response();
    }

    // 達成内容のテキスト生成
    let (achievement_text, achievement_line2, description) = match award_type {
        AwardType::Activator => (
            format!("{} Summits Activated", query.summits),
            None,
            "SOTA Japan Branch 10th Anniversary Award - Activator".to_string(),
        ),
        AwardType::Chaser => {
            let line2 = match (&query.best_activators, &query.best_summit) {
                (Some(activators), Some(summit)) => {
                    Some(format!("Best: {} from {}", activators, summit))
                }
                _ => None,
            };
            (
                format!("{} Summits Chased", query.summits),
                line2,
                "SOTA Japan Branch 10th Anniversary Award - Chaser".to_string(),
            )
        }
    };

    let info = CertificateInfo {
        callsign: query.callsign.clone(),
        achievement_text,
        achievement_line2,
        description: Some(description),
        issue_date: Some(issue_date),
    };

    // PDF生成
    match generator.generate(award_type, &info) {
        Ok(pdf_bytes) => {
            let filename = format!(
                "sota_10th_anniversary_{}_{}.pdf",
                query.award_type, query.callsign
            );

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/pdf")
                .header(
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                )
                .body(Body::from(pdf_bytes))
                .unwrap()
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("PDF生成に失敗: {}", e)
            })),
        )
            .into_response(),
    }
}

pub fn build_sota_routers(auth: &FireAuth) -> Router<AppState> {
    let protected = with_auth(
        Router::new()
            .route("/import", post(import_summit_list))
            .route("/import/ja", post(import_sota_opt_reference))
            .route("/import/pota", post(import_summit_pota))
            .route("/import/maps", post(import_summit_maps))
            .route("/log", post(upload_log))
            .route("/log", delete(delete_log))
            .route("/update", post(update_summit_list))
            .route("/summits/export", get(export_sota_reference_ja))
            .route("/summits/{summit_code}", put(update_sota_reference))
            .route("/summits/{summit_code}", delete(delete_sota_reference)),
        auth,
    );

    let public = Router::new()
        .route("/spots", get(show_sota_spots))
        .route("/alerts", get(show_sota_alerts))
        .route("/summits", get(show_all_sota_reference))
        .route("/summits/{summit_code}", get(show_sota_reference))
        .route("/summits/{summit_code}/pota", get(show_summit_pota))
        .route("/summits/{summit_code}/map", get(show_summit_map))
        // 地域ごとにまとめると 1 MB ほどになるので、圧縮して返す
        .route(
            "/maps",
            get(show_summit_maps).layer(CompressionLayer::new()),
        )
        .route("/summits/search", get(search_sota_reference))
        .route(
            "/award/10th-anniversary/judge",
            post(judge_10th_anniversary_award),
        )
        .route(
            "/award/10th-anniversary/certificate",
            get(generate_award_pdf),
        );

    let routers = Router::new().merge(protected).merge(public);

    Router::new().nest("/sota", routers)
}

#[cfg(test)]
mod tests {
    use super::region_prefix;

    #[test]
    fn test_region_prefix() {
        assert_eq!(region_prefix("JA/KN").as_deref(), Some("JA/KN-"));
        assert_eq!(region_prefix("JA5/KC-0").as_deref(), Some("JA5/KC-0"));
        // 協会だけ、全件、LIKE の特殊文字は受け付けない
        assert_eq!(region_prefix("JA"), None);
        assert_eq!(region_prefix("JA/"), None);
        assert_eq!(region_prefix("JA/K"), None);
        assert_eq!(region_prefix("JA/%"), None);
        assert_eq!(region_prefix("JA/kn"), None);
        assert_eq!(region_prefix("JA/KN-0a"), None);
    }
}
