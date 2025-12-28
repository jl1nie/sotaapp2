//! FLE (Fast Log Entry) ハンドラー
//!
//! FLE形式テキストをパースし、各種出力形式に変換するエンドポイント

use axum::{
    extract::{Form, Multipart},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use registry::AppState;
use serde::{Deserialize, Serialize};
use service::implement::fle::{compile_fle, generate_fle_output, FleCompileResult};

/// FLEルーターを作成
pub fn fle_router() -> Router<AppState> {
    Router::new()
        .route("/compile", post(compile_handler))
        .route("/generate", post(generate_handler))
}

/// FLEコンパイルリクエスト (フォーム形式 - フロントエンド互換)
#[derive(Debug, Deserialize)]
struct FleCompileFormRequest {
    /// コマンド (interp) - 互換性のため受け取るが使用しない
    #[serde(default)]
    #[allow(dead_code)]
    command: String,
    /// FLE形式テキスト
    arg: String,
}

/// FLEコンパイルレスポンス
#[derive(Debug, Serialize)]
struct FleCompileResponse {
    status: String,
    #[serde(rename = "logtype")]
    log_type: String,
    mycall: String,
    operator: String,
    mysota: String,
    mywwff: String,
    mypota: Vec<String>,
    qslmsg: String,
    #[serde(rename = "logtext", skip_serializing_if = "Vec::is_empty")]
    log_text: Vec<Vec<String>>,
    #[serde(rename = "hamlogtext", skip_serializing_if = "Vec::is_empty")]
    hamlog_text: Vec<Vec<String>>,
}

impl From<FleCompileResult> for FleCompileResponse {
    fn from(result: FleCompileResult) -> Self {
        // QSOレコードを表形式に変換
        let log_text: Vec<Vec<String>> = if result.errors.is_empty() {
            result
                .records
                .iter()
                .enumerate()
                .map(|(i, qso)| {
                    vec![
                        (i + 1).to_string(),
                        qso.mycall.clone(),
                        format!("{:04}-{:02}-{:02}", qso.year, qso.month, qso.day),
                        format!("{:02}:{:02}", qso.hour, qso.min),
                        qso.callsign.clone(),
                        qso.band.clone(),
                        qso.mode.clone(),
                        qso.rst_sent.clone(),
                        qso.rst_rcvd.clone(),
                        qso.mysota.clone(),
                        qso.hissota.clone(),
                        qso.mypota.join("/"),
                        qso.hispota.join("/"),
                        format!("{}{}", qso.qsomsg, qso.qsormks),
                        qso.operator.clone(),
                    ]
                })
                .collect()
        } else {
            // エラー時はエラーログを返す
            result
                .errors
                .iter()
                .map(|e| vec![e.line.to_string(), e.message.clone(), String::new()])
                .collect()
        };

        // HAMLOG形式のログテキスト
        let hamlog_text: Vec<Vec<String>> = if result.errors.is_empty() {
            result
                .records
                .iter()
                .enumerate()
                .map(|(i, qso)| {
                    let rmks = service::implement::logconv::get_ref(&qso.qsormks);
                    vec![
                        (i + 1).to_string(),
                        qso.callsign.clone(),
                        format!("{:04}-{:02}-{:02}", qso.year, qso.month, qso.day),
                        format!("{:02}:{:02}U", qso.hour, qso.min),
                        qso.rst_sent.clone(),
                        qso.rst_rcvd.clone(),
                        qso.freq.clone(),
                        qso.mode.clone(),
                        rmks.loc_org.to_string(),
                        qso.qsomsg.clone(),
                        String::new(),
                        String::new(),
                    ]
                })
                .collect()
        } else {
            result
                .errors
                .iter()
                .map(|e| vec![e.line.to_string(), e.message.clone(), String::new()])
                .collect()
        };

        FleCompileResponse {
            status: result.status,
            log_type: result.log_type,
            mycall: result.mycall,
            operator: result.operator,
            mysota: result.mysota,
            mywwff: result.mywwff,
            mypota: result.mypota,
            qslmsg: result.qslmsg,
            log_text,
            hamlog_text,
        }
    }
}

/// FLEコンパイルハンドラー (JSON応答)
///
/// FLE形式テキストをパースし、QSOレコードとステータスを返す
async fn compile_handler(Form(req): Form<FleCompileFormRequest>) -> Response {
    let result = compile_fle(&req.arg);
    let response: FleCompileResponse = result.into();

    (StatusCode::OK, Json(response)).into_response()
}

/// FLE生成ハンドラー (ZIP応答)
///
/// FLE形式テキストからSOTA CSV/POTA ADIF/HAMLOG CSV等を生成しZIPで返す
async fn generate_handler(mut multipart: Multipart) -> Response {
    let mut text_content: Option<String> = None;

    // マルチパートフォームを処理
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("edittext") | Some("text") | Some("file") => {
                if let Ok(text) = field.text().await {
                    text_content = Some(text);
                }
            }
            _ => {}
        }
    }

    let text = match text_content {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "status": "NG",
                    "error": "Missing 'edittext' or 'text' parameter"
                })),
            )
                .into_response();
        }
    };

    // FLEをコンパイル
    let result = compile_fle(&text);

    // 出力を生成
    match generate_fle_output(&result) {
        Ok(zip_data) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/zip"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"fle.zip\"",
                ),
            ],
            zip_data,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "NG",
                "error": e
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fle_router_exists() {
        let _router = fle_router();
    }

    #[tokio::test]
    async fn test_compile_fle_basic() {
        let input = r#"mycall JA1ABC
date 2024-01-15
mysota JA/TK-001
40m cw
0900 JA1XYZ 599 599
"#;
        let result = compile_fle(input);
        assert_eq!(result.status, "OK");
        assert_eq!(result.mycall, "JA1ABC");
        assert_eq!(result.records.len(), 1);
    }
}
