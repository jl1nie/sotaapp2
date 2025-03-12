use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    UnprocessableEntity(String),
    #[error("{0}")]
    EntityNotFound(String),
    // #[error("{0}")]
    //ValidationError(#[from] garde::Report),
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
    // #[error("{0}")]
    // KeyValueStoreError(#[from] redis::RedisError),
    // #[error("{0}")]
    // BcryptError(#[from] bcrypt::BcryptError),
    #[error("{0}")]
    UuidError(uuid::Error),
    #[error("{0}")]
    ConvertToUuidError(#[from] uuid::Error),
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
        let status_code = match self {
            AppError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::CSVReadError(e) =>{tracing::error!("CSV Error {:?}",e); StatusCode::UNPROCESSABLE_ENTITY},
            AppError::EntityNotFound(e) => {tracing::error!("Not found {:?}",e); StatusCode::NOT_FOUND},
            AppError::RowNotFound{source,location} =>  {tracing::error!("Not found {:?}at {}",source,location);StatusCode::NOT_FOUND}
            AppError::UuidError(e) =>  {tracing::error!("Not found {:?}",e); StatusCode::NOT_FOUND},
            /* AppError::ValidationError(_) |*/
            AppError::ConvertToUuidError(_) => { StatusCode::BAD_REQUEST},
            AppError::UnauthenticatedError | AppError::ForbiddenOperation => StatusCode::FORBIDDEN,
            AppError::UnauthorizedError => StatusCode::UNAUTHORIZED,
            e @ (AppError::TransactionError(_)
            | AppError::SpecificOperationError(_)
            | AppError::APRSError
            | AppError::CronjobError(_)
            | AppError::NoRowsAffectedError(_)
            | AppError::PostError(_)
            | AppError::GetError(_)
            | AppError::JsonError(_)
            | AppError::ParseError(_)
            /* 
            | AppError::KeyValueStoreError(_)
            | AppError::BcryptError(_)
            */
            | AppError::ConversionEntityError(_)) => {
                tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Unexpected error happened"
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };
        status_code.into_response()
    }
}

// エラー型が `AppError` なものを扱える `Result` 型
pub type AppResult<T> = Result<T, AppError>;
