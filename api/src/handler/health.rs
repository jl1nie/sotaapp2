use axum::{http::StatusCode, routing::get, Router};
use common::error::AppResult;
use shaku_axum::Inject;

use registry::{AppRegistry, AppState};
use service::services::AdminService;
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn health_check_db(admin_service: Inject<AppRegistry, dyn AdminService>) -> AppResult<()> {
    if admin_service.health_check().await? {
        Ok(())
    } else {
        Err(common::error::AppError::ConversionEntityError(
            "health check faild".to_string(),
        ))
    }
}

pub fn build_health_chek_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", get(health_check))
        .route("/db", get(health_check_db));

    Router::new().nest("/health", routers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use tower::ServiceExt;

    /// 単純なhealthエンドポイントのテスト（DIを必要としない）
    #[tokio::test]
    async fn test_health_check_endpoint() {
        // シンプルなルーターを構築（Stateなし）
        let app = Router::new().route("/health", get(health_check));

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    /// health_check関数の直接テスト
    #[tokio::test]
    async fn test_health_check_returns_ok() {
        let status = health_check().await;
        assert_eq!(status, StatusCode::OK);
    }
}
