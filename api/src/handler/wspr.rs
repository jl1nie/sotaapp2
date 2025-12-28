//! WSPRハンドラー
//!
//! WSPRスポットデータからSVGグラフを生成するエンドポイント

use axum::{
    extract::Form,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use registry::AppState;
use serde::Deserialize;
use service::implement::wspr_service::{generate_wspr_svg, WsprRequest};

/// WSPRルーターを作成
pub fn wspr_router() -> Router<AppState> {
    Router::new().route("/svg", post(wspr_svg_handler))
}

/// フォームリクエスト
#[derive(Debug, Deserialize)]
struct WsprFormRequest {
    arg: String,
}

/// WSPRスポットからSVGを生成
///
/// フォームで`arg`パラメータにJSONを受け取る
async fn wspr_svg_handler(Form(form): Form<WsprFormRequest>) -> Response {
    // JSONをパース
    let request: WsprRequest = match serde_json::from_str(&form.arg) {
        Ok(req) => req,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid JSON: {}", e)})),
            )
                .into_response();
        }
    };

    // SVG生成
    match generate_wspr_svg(&request) {
        Ok(svg) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "image/svg+xml")],
            svg,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("SVG generation failed: {}", e)})),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wspr_router_exists() {
        let _router = wspr_router();
        // ルーターが正常に作成されることを確認
    }
}
