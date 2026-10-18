//! 応答の組み立て(JSON のメッセージと、ZIP)

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use service::implement::logconv::create_zip;
use std::collections::HashMap;

/// JSON の応答。空の項目は省く
#[derive(Debug, Serialize)]
pub struct LogconvResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    error_log: Vec<String>,
}

impl LogconvResponse {
    /// 失敗(`status: NG`)
    pub fn ng(error: impl Into<String>) -> Self {
        Self {
            status: "NG",
            error: Some(error.into()),
            error_log: Vec::new(),
        }
    }

    /// 失敗の理由の一覧(誤りの行ごと)
    pub fn with_error_log(mut self, error_log: Vec<String>) -> Self {
        self.error_log = error_log;
        self
    }

    pub fn reply(self, status: StatusCode) -> Response {
        (status, Json(self)).into_response()
    }
}

/// 処理を続けられない理由。`?` で呼び出し元に伝え、HTTP の応答(NG の JSON)になる
#[derive(Debug)]
pub struct Failure {
    status: StatusCode,
    body: LogconvResponse,
}

impl Failure {
    /// 400 Bad Request で、理由を返す
    pub fn bad_request(error: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: LogconvResponse::ng(error),
        }
    }

    /// 誤りの行ごとの理由を添える
    pub fn with_error_log(mut self, error_log: Vec<String>) -> Self {
        self.body = self.body.with_error_log(error_log);
        self
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        self.body.reply(self.status)
    }
}

/// ファイルがあれば ZIP で返す。1つもなければ、200 で `message` を返す(対象の参照がなかった)
pub fn zip_or_empty(files: HashMap<String, String>, zip_name: &str, message: &str) -> Response {
    if files.is_empty() {
        return LogconvResponse {
            status: "OK",
            error: Some(message.to_string()),
            error_log: Vec::new(),
        }
        .reply(StatusCode::OK);
    }
    match create_zip(&files) {
        Ok(zip) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/zip".to_string()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{zip_name}\""),
                ),
            ],
            zip,
        )
            .into_response(),
        Err(e) => LogconvResponse::ng(format!("ZIP creation failed: {e}"))
            .reply(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
