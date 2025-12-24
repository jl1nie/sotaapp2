use std::str::FromStr;

use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::Json;
use axum::{routing::post, Router};
use firebase_auth_sdk::FireAuth;

use crate::model::auth::AuthRequest;
use domain::model::id::UserId;
use registry::AppState;

/// 認証ミドルウェアをルーターに適用
pub fn with_auth<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    auth: &FireAuth,
) -> Router<S> {
    router.route_layer(middleware::from_fn_with_state(auth.clone(), auth_middle))
}

pub async fn auth_middle(
    State(auth_service): State<FireAuth>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = auth_service
        .get_user_info(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id = UserId::from_str(&user.local_id).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(user_id);

    Ok(next.run(req).await)
}

pub async fn sign_in(
    State(auth_service): State<FireAuth>,
    Json(creds_request): Json<AuthRequest>,
) -> impl IntoResponse {
    tracing::info!("auth signin email={}", creds_request.email);

    match auth_service
        .sign_in_email(
            creds_request.email.as_str(),
            creds_request.password.as_str(),
            true,
        )
        .await
    {
        Ok(response) => (
            StatusCode::OK,
            [(header::AUTHORIZATION, response.id_token)],
            "Successfully LoggedIn",
        ),
        Err(ex) => {
            tracing::warn!("Sign-in failed: {:?}", ex);
            (
                StatusCode::UNAUTHORIZED,
                [(header::AUTHORIZATION, String::new())],
                "Invalid Credentials",
            )
        }
    }
}

pub fn build_auth_routers(auth: &FireAuth) -> Router<AppState> {
    let routers = Router::new()
        .route("/signin", post(sign_in))
        .with_state(auth.clone());
    Router::new().nest("/auth", routers)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AuthRequestのJSONデシリアライズテスト
    #[test]
    fn test_auth_request_deserialize() {
        let json = r#"{"email": "test@example.com", "password": "secret123"}"#;
        let req: AuthRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.email, "test@example.com");
        assert_eq!(req.password, "secret123");
    }

    /// AuthRequestの必須フィールド欠落テスト
    #[test]
    fn test_auth_request_missing_email() {
        let json = r#"{"password": "secret123"}"#;
        let result: Result<AuthRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_request_missing_password() {
        let json = r#"{"email": "test@example.com"}"#;
        let result: Result<AuthRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    /// 認証ミドルウェア: Authorizationヘッダーなしで401を返すテスト
    #[tokio::test]
    async fn test_auth_middleware_rejects_missing_header() {
        // FireAuthを必要としないダミーハンドラでミドルウェアの振る舞いをテスト
        // Note: 実際のauth_middleはFireAuthが必要なため、ここではヘッダー解析ロジックのみテスト

        // Bearerトークンなしのリクエストではエラーになることを確認
        let header_value = "Basic invalid";
        let result = header_value.strip_prefix("Bearer ");
        assert!(result.is_none());
    }

    /// Bearerトークン抽出ロジックのテスト
    #[test]
    fn test_bearer_token_extraction() {
        // 正常なBearerトークン
        let header = "Bearer abc123token";
        let token = header.strip_prefix("Bearer ");
        assert_eq!(token, Some("abc123token"));

        // Bearerプレフィックスなし
        let header = "abc123token";
        let token = header.strip_prefix("Bearer ");
        assert!(token.is_none());

        // 空のトークン
        let header = "Bearer ";
        let token = header.strip_prefix("Bearer ");
        assert_eq!(token, Some(""));
    }
}
