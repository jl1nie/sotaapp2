use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Serialize;
use thiserror::Error;

/// Error response returned as JSON
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    UnprocessableEntity(String),
    #[error("{0}")]
    EntityNotFound(String),
    #[error("トランザクションを実行できませんでした。")]
    TransactionError(#[source] sqlx::Error),
    #[error("データベース処理実行中にエラーが発生しました。")]
    SpecificOperationError(#[source] sqlx::Error),
    #[error("指定された行が見つかりません。{location}")]
    RowNotFound {
        #[source]
        source: sqlx::Error,
        location: String,
    },
    #[error("No rows affected: {0}")]
    NoRowsAffectedError(String),
    #[error("CSVの読み込みに失敗しました。")]
    CSVReadError(#[source] csv::Error),
    #[error("POSTに失敗しました。")]
    PostError(#[source] reqwest::Error),
    #[error("HTTP-GETに失敗しました。")]
    GetError(#[source] reqwest::Error),
    #[error("JSON変換に失敗しました。")]
    JsonError(#[source] serde_json::Error),
    #[error("時刻変換に失敗しました。")]
    ParseError(#[source] chrono::ParseError),
    #[error("APRSにエラーが発生しました")]
    APRSError,
    #[error("JOBSchedulerにエラーが発生しました")]
    CronjobError(#[source] tokio_cron_scheduler::JobSchedulerError),
    #[error("UUID変換エラー: {0}")]
    UuidError(#[from] uuid::Error),
    #[error("ログインに失敗しました")]
    UnauthenticatedError,
    #[error("認可情報が誤っています")]
    UnauthorizedError,
    #[error("許可されていない操作です")]
    ForbiddenOperation,
    #[error("{0}")]
    ConversionEntityError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, message, code) = match &self {
            AppError::UnprocessableEntity(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, msg.clone(), Some("UNPROCESSABLE_ENTITY"))
            }
            AppError::CSVReadError(e) => {
                tracing::error!("CSV Error {:?}", e);
                (StatusCode::UNPROCESSABLE_ENTITY, "CSVの読み込みに失敗しました".to_string(), Some("CSV_READ_ERROR"))
            }
            AppError::EntityNotFound(msg) => {
                tracing::error!("Not found {:?}", msg);
                (StatusCode::NOT_FOUND, msg.clone(), Some("NOT_FOUND"))
            }
            AppError::RowNotFound { source, location } => {
                tracing::error!("Not found {:?} at {}", source, location);
                (StatusCode::NOT_FOUND, format!("指定された行が見つかりません: {}", location), Some("ROW_NOT_FOUND"))
            }
            AppError::UuidError(e) => {
                tracing::error!("UUID Error {:?}", e);
                (StatusCode::BAD_REQUEST, "UUIDの変換に失敗しました".to_string(), Some("UUID_ERROR"))
            }
            AppError::UnauthenticatedError => {
                (StatusCode::UNAUTHORIZED, "ログインに失敗しました".to_string(), Some("UNAUTHENTICATED"))
            }
            AppError::UnauthorizedError => {
                (StatusCode::FORBIDDEN, "認可情報が誤っています".to_string(), Some("UNAUTHORIZED"))
            }
            AppError::ForbiddenOperation => {
                (StatusCode::FORBIDDEN, "許可されていない操作です".to_string(), Some("FORBIDDEN"))
            }
            AppError::TransactionError(e) => {
                tracing::error!(error.cause_chain = ?e, "Transaction error");
                (StatusCode::INTERNAL_SERVER_ERROR, "データベーストランザクションエラー".to_string(), Some("TRANSACTION_ERROR"))
            }
            AppError::SpecificOperationError(e) => {
                tracing::error!(error.cause_chain = ?e, "Database operation error");
                (StatusCode::INTERNAL_SERVER_ERROR, "データベース処理エラー".to_string(), Some("DB_OPERATION_ERROR"))
            }
            AppError::NoRowsAffectedError(msg) => {
                tracing::error!("No rows affected: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("更新対象がありません: {}", msg), Some("NO_ROWS_AFFECTED"))
            }
            AppError::PostError(e) => {
                tracing::error!(error.cause_chain = ?e, "POST error");
                (StatusCode::INTERNAL_SERVER_ERROR, "POSTリクエストに失敗しました".to_string(), Some("POST_ERROR"))
            }
            AppError::GetError(e) => {
                tracing::error!(error.cause_chain = ?e, "GET error");
                (StatusCode::INTERNAL_SERVER_ERROR, "GETリクエストに失敗しました".to_string(), Some("GET_ERROR"))
            }
            AppError::JsonError(e) => {
                tracing::error!(error.cause_chain = ?e, "JSON error");
                (StatusCode::INTERNAL_SERVER_ERROR, "JSON変換に失敗しました".to_string(), Some("JSON_ERROR"))
            }
            AppError::ParseError(e) => {
                tracing::error!(error.cause_chain = ?e, "Parse error");
                (StatusCode::INTERNAL_SERVER_ERROR, "時刻変換に失敗しました".to_string(), Some("PARSE_ERROR"))
            }
            AppError::APRSError => {
                tracing::error!("APRS error");
                (StatusCode::INTERNAL_SERVER_ERROR, "APRSにエラーが発生しました".to_string(), Some("APRS_ERROR"))
            }
            AppError::CronjobError(e) => {
                tracing::error!(error.cause_chain = ?e, "Cronjob error");
                (StatusCode::INTERNAL_SERVER_ERROR, "JOBスケジューラにエラーが発生しました".to_string(), Some("CRONJOB_ERROR"))
            }
            AppError::ConversionEntityError(msg) => {
                tracing::error!("Conversion error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), Some("CONVERSION_ERROR"))
            }
        };

        let body = ErrorResponse {
            success: false,
            message,
            code: code.map(|c| c.to_string()),
        };

        (status_code, Json(body)).into_response()
    }
}

// エラー型が `AppError` なものを扱える `Result` 型
pub type AppResult<T> = Result<T, AppError>;

/// データベース操作エラー用のコンテキスト付きクロージャを生成
///
/// # 使用例
/// ```ignore
/// sqlx::query("SELECT * FROM users")
///     .fetch_one(&pool)
///     .await
///     .map_err(db_error("fetch user by id"))?;
/// ```
pub fn db_error(context: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |e| {
        tracing::debug!("DB error in {}: {:?}", context, e);
        AppError::SpecificOperationError(e)
    }
}

/// トランザクションエラー用のコンテキスト付きクロージャを生成
///
/// # 使用例
/// ```ignore
/// tx.commit()
///     .await
///     .map_err(tx_error("commit user update"))?;
/// ```
pub fn tx_error(context: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |e| {
        tracing::debug!("Transaction error in {}: {:?}", context, e);
        AppError::TransactionError(e)
    }
}

/// 行が見つからないエラー用のコンテキスト付きクロージャを生成
///
/// # 使用例
/// ```ignore
/// sqlx::query("SELECT * FROM users WHERE id = ?")
///     .fetch_one(&pool)
///     .await
///     .map_err(row_not_found("user lookup"))?;
/// ```
pub fn row_not_found(location: &'static str) -> impl Fn(sqlx::Error) -> AppError {
    move |source| AppError::RowNotFound {
        source,
        location: location.to_string(),
    }
}
