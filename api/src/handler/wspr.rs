//! WSPRハンドラー
//!
//! WSPRスポットデータからSVGグラフを生成するエンドポイント

use axum::{
    extract::Multipart,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use registry::AppState;
use service::implement::wspr_service::{generate_wspr_svg, WsprRequest};

/// WSPRルーターを作成
pub fn wspr_router() -> Router<AppState> {
    Router::new().route("/svg", post(wspr_svg_handler))
}

/// WSPRスポットからSVGを生成
///
/// マルチパートフォームで`arg`パラメータにJSONを受け取る
async fn wspr_svg_handler(mut multipart: Multipart) -> Response {
    let mut arg_json: Option<String> = None;

    // マルチパートフォームから`arg`フィールドを取得
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("arg") {
            if let Ok(text) = field.text().await {
                arg_json = Some(text);
                break;
            }
        }
    }

    let arg_json = match arg_json {
        Some(json) => json,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'arg' parameter"})),
            )
                .into_response();
        }
    };

    // JSONをパース
    let request: WsprRequest = match serde_json::from_str(&arg_json) {
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
