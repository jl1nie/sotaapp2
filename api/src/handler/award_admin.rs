//! アワード証明書管理ハンドラー
//!
//! PDFテンプレートのアップロードと設定管理

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use common::award_config::AwardTemplateConfig;
use firebase_auth_sdk::FireAuth;
use registry::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::auth::with_auth;

/// テンプレートの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateType {
    Activator,
    Chaser,
}

impl TemplateType {
    fn filename(&self) -> &'static str {
        match self {
            TemplateType::Activator => "activator_template.pdf",
            TemplateType::Chaser => "chaser_template.pdf",
        }
    }
}

/// テンプレートステータスレスポンス
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateStatus {
    pub activator_available: bool,
    pub chaser_available: bool,
}

/// 設定更新リクエスト
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfigRequest {
    pub activator: Option<TemplateConfigUpdate>,
    pub chaser: Option<TemplateConfigUpdate>,
}

/// 単一テンプレートの設定更新
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateConfigUpdate {
    pub callsign_x: Option<f32>,
    pub callsign_y: Option<f32>,
    pub callsign_font_size: Option<f32>,
    pub callsign_color: Option<[u8; 3]>,
    pub achievement_x: Option<f32>,
    pub achievement_y: Option<f32>,
    pub achievement_font_size: Option<f32>,
    pub achievement_color: Option<[u8; 3]>,
}

/// テンプレートのステータス取得
async fn get_template_status(State(state): State<AppState>) -> Json<TemplateStatus> {
    let template_dir = PathBuf::from(&state.config.award_template_dir);

    Json(TemplateStatus {
        activator_available: template_dir.join("activator_template.pdf").exists(),
        chaser_available: template_dir.join("chaser_template.pdf").exists(),
    })
}

/// 設定取得
async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let config_path = PathBuf::from(&state.config.award_config_path);

    match AwardTemplateConfig::load_from_file(&config_path) {
        Ok(config) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("設定の読み込みに失敗: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 設定更新
async fn update_config(
    State(state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    let config_path = PathBuf::from(&state.config.award_config_path);

    // 既存設定を読み込み
    let mut config = match AwardTemplateConfig::load_from_file(&config_path) {
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

    // アクティベータ設定の更新
    if let Some(update) = req.activator {
        apply_config_update(&mut config.activator, update);
    }

    // チェイサー設定の更新
    if let Some(update) = req.chaser {
        apply_config_update(&mut config.chaser, update);
    }

    // 保存
    match config.save_to_file(&config_path) {
        Ok(_) => (StatusCode::OK, Json(config)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("設定の保存に失敗: {}", e)
            })),
        )
            .into_response(),
    }
}

fn apply_config_update(
    config: &mut common::award_config::TemplateConfig,
    update: TemplateConfigUpdate,
) {
    if let Some(x) = update.callsign_x {
        config.callsign.x = x;
    }
    if let Some(y) = update.callsign_y {
        config.callsign.y = y;
    }
    if let Some(size) = update.callsign_font_size {
        config.callsign.font_size = size;
    }
    if let Some(color) = update.callsign_color {
        config.callsign.color = color;
    }
    if let Some(x) = update.achievement_x {
        config.achievement.x = x;
    }
    if let Some(y) = update.achievement_y {
        config.achievement.y = y;
    }
    if let Some(size) = update.achievement_font_size {
        config.achievement.font_size = size;
    }
    if let Some(color) = update.achievement_color {
        config.achievement.color = color;
    }
}

/// テンプレートアップロード
async fn upload_template(
    State(state): State<AppState>,
    Path(template_type): Path<TemplateType>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let template_dir = PathBuf::from(&state.config.award_template_dir);

    // ディレクトリ作成
    if let Err(e) = std::fs::create_dir_all(&template_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("ディレクトリの作成に失敗: {}", e)
            })),
        )
            .into_response();
    }

    // ファイル読み取り
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "ファイルが送信されていません"
                })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("マルチパートの読み込みに失敗: {}", e)
                })),
            )
                .into_response()
        }
    };

    let data = match field.bytes().await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("ファイルの読み込みに失敗: {}", e)
                })),
            )
                .into_response()
        }
    };

    // PDFかどうかを簡易チェック（マジックナンバー）
    if data.len() < 4 || &data[0..4] != b"%PDF" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "有効なPDFファイルではありません"
            })),
        )
            .into_response();
    }

    // ファイル保存
    let file_path = template_dir.join(template_type.filename());
    if let Err(e) = std::fs::write(&file_path, &data) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("ファイルの保存に失敗: {}", e)
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "テンプレートをアップロードしました",
            "type": format!("{:?}", template_type),
            "size": data.len()
        })),
    )
        .into_response()
}

/// アワード管理ルーター作成
pub fn build_award_admin_routers(auth: &FireAuth) -> Router<AppState> {
    let admin_routes = with_auth(
        Router::new()
            .route("/templates/status", get(get_template_status))
            .route("/templates/{template_type}", post(upload_template))
            .route("/config", get(get_config))
            .route("/config", post(update_config)),
        auth,
    );

    Router::new().nest("/admin/award", admin_routes)
}
