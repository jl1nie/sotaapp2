use axum::{http::StatusCode, routing::get, Router};
use common::error::AppResult;
use shaku_axum::Inject;

use registry::{AppRegistry, AppState};
use service::interface::AdminService;
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub async fn health_check_db(worker: Inject<AppRegistry, dyn AdminService>) -> AppResult<()> {
    if worker.health_check().await? {
        Ok(())
    } else {
        Err(common::error::AppError::ConversionEntityError(
            "Health check faild".to_string(),
        ))
    }
}

pub fn build_health_chek_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", get(health_check))
        .route("/db", get(health_check_db));

    Router::new().nest("/health", routers)
}
