use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::Json;
use axum::{routing::post, Router};
use fireauth::FireAuth;
use serde::Serialize;

use super::multipart::admin_upload_limit;
use crate::model::auth::AuthRequest;
use domain::model::admin_user::AdminRole;
use domain::model::id::UserId;
use domain::repository::access::AccessTokenVerifier;
use registry::AppState;
use service::services::AdminUserService;

/// Cloudflare Access が付ける署名のヘッダーと Cookie
const ACCESS_HEADER: &str = "cf-access-jwt-assertion";
const ACCESS_COOKIE: &str = "CF_Authorization";

/// どの入り口で入ったか
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    Firebase,
    Access,
    /// 手元の開発用(デバッグビルドの `ADMIN_DEV_EMAIL`)
    Dev,
}

/// 認証を通った管理画面の人。通ったら request の extensions に入る
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminIdentity {
    pub email: String,
    pub role: AdminRole,
    pub method: AuthMethod,
}

/// 管理画面の認証に必要なもの(Firebase と、ポートを通した Access・役割の表)
#[derive(Clone)]
pub struct AdminAuth {
    pub fire: FireAuth,
    access: Arc<dyn AccessTokenVerifier>,
    users: Arc<dyn AdminUserService>,
    dev_email: Option<String>,
}

impl AdminAuth {
    pub fn new(
        fire: FireAuth,
        access: Arc<dyn AccessTokenVerifier>,
        users: Arc<dyn AdminUserService>,
        dev_email: Option<String>,
    ) -> Self {
        Self {
            fire,
            access,
            users,
            dev_email,
        }
    }

    /// 署名(Access)か Bearer(Firebase)から、誰かを決める。
    /// 返す `UserId` は、今のハンドラー(ログの取り込み・削除)のため
    pub async fn authenticate(
        &self,
        headers: &HeaderMap,
    ) -> Result<(AdminIdentity, UserId), StatusCode> {
        if let Some(email) = &self.dev_email {
            return Ok((
                AdminIdentity {
                    email: email.clone(),
                    role: AdminRole::Admin,
                    method: AuthMethod::Dev,
                },
                UserId::from(format!("access:{email}")),
            ));
        }

        if self.access.is_enabled() {
            if let Some(token) = access_token(headers) {
                let id = self
                    .access
                    .verify(&token)
                    .await
                    .map_err(|_| StatusCode::UNAUTHORIZED)?;
                // 署名が通っても、役割の表にいない人は入れない
                let user = self
                    .users
                    .find_user(&id.email)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .ok_or(StatusCode::FORBIDDEN)?;
                return Ok((
                    AdminIdentity {
                        email: user.email.clone(),
                        role: user.role,
                        method: AuthMethod::Access,
                    },
                    UserId::from(format!("access:{}", user.email)),
                ));
            }
        }

        let token = bearer_token(headers).ok_or(StatusCode::UNAUTHORIZED)?;
        let user = self
            .fire
            .get_user_info(token)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        // Firebase で入った人は、今までどおり admin
        Ok((
            AdminIdentity {
                email: user.email.to_lowercase(),
                role: AdminRole::Admin,
                method: AuthMethod::Firebase,
            },
            UserId::from(user.local_id),
        ))
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
}

/// 署名は、ヘッダー `Cf-Access-Jwt-Assertion`、なければ Cookie `CF_Authorization`
fn access_token(headers: &HeaderMap) -> Option<String> {
    let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_string());
    if let Some(t) = headers
        .get(ACCESS_HEADER)
        .and_then(|h| h.to_str().ok())
        .and_then(non_empty)
    {
        return Some(t);
    }
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|h| h.to_str().ok())
        .flat_map(|c| c.split(';'))
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == ACCESS_COOKIE)
        .and_then(|(_, v)| non_empty(v))
}

#[derive(Clone)]
struct AuthLayer {
    auth: AdminAuth,
    /// reviewer も通すか(見直しの API だけ)
    allow_reviewer: bool,
}

fn guarded<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    auth: &AdminAuth,
    allow_reviewer: bool,
) -> Router<S> {
    let layer = AuthLayer {
        auth: auth.clone(),
        allow_reviewer,
    };
    router
        .route_layer(middleware::from_fn_with_state(layer, auth_middle))
        .layer(admin_upload_limit())
}

/// 認証ミドルウェアをルーターに適用(admin だけ)
///
/// 認証ありのルートは管理者が使うので、アップロードの上限も管理者の大きさにする
pub fn with_auth<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    auth: &AdminAuth,
) -> Router<S> {
    guarded(router, auth, false)
}

/// admin と reviewer の両方を通す(見直しの API と whoami)
pub fn with_review_auth<S: Clone + Send + Sync + 'static>(
    router: Router<S>,
    auth: &AdminAuth,
) -> Router<S> {
    guarded(router, auth, true)
}

/// ブラウザーが「別のサイトから」と示している、書き込みの要求か。
/// Access の入り口は Cookie で通るので、ほかのサイトのフォームからの送信(CSRF)を断つ。
/// ヘッダーがなければ(curl など、ブラウザーでないもの)Cookie も付かないので通す
fn is_cross_site_write(method: &Method, headers: &HeaderMap) -> bool {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return false;
    }
    let text = |name: &str| headers.get(name).and_then(|h| h.to_str().ok());
    if let Some(site) = text("sec-fetch-site") {
        return !matches!(site, "same-origin" | "none");
    }
    match (text("origin"), text("host")) {
        (Some(origin), Some(host)) => {
            let origin_host = origin.split_once("://").map_or(origin, |(_, h)| h);
            !origin_host.eq_ignore_ascii_case(host)
        }
        _ => false,
    }
}

async fn auth_middle(
    State(layer): State<AuthLayer>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    let (identity, user_id) = layer.auth.authenticate(req.headers()).await?;

    // 段階 2 で、Firebase がまだ使われているかを見るため
    tracing::info!(
        method = ?identity.method,
        email = %identity.email,
        path = %req.uri().path(),
        "admin request"
    );

    if identity.role != AdminRole::Admin && !layer.allow_reviewer {
        return Err(StatusCode::FORBIDDEN);
    }
    if identity.method == AuthMethod::Access && is_cross_site_write(req.method(), req.headers()) {
        tracing::warn!(email = %identity.email, "cross-site write rejected");
        return Err(StatusCode::FORBIDDEN);
    }
    req.extensions_mut().insert(user_id);
    req.extensions_mut().insert(identity);

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

pub fn build_auth_routers(auth: &AdminAuth) -> Router<AppState> {
    let routers = Router::new()
        .route("/signin", post(sign_in))
        .with_state(auth.fire.clone());
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

    // ---- 役割の分け方(偽の検証器と偽の役割の表で確かめる) ----

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::routing::get;
    use axum::Extension;
    use chrono::Utc;
    use common::error::{AppError, AppResult};
    use domain::model::admin_user::AdminUser;
    use domain::repository::access::AccessIdentity;
    use tower::ServiceExt;

    /// "admin-token"・"reviewer-token"・"unlisted-token" だけを通す
    struct FakeVerifier {
        enabled: bool,
    }

    #[async_trait]
    impl AccessTokenVerifier for FakeVerifier {
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        async fn verify(&self, token: &str) -> AppResult<AccessIdentity> {
            let email = match token {
                "admin-token" => "admin@example.com",
                "reviewer-token" => "reviewer@example.com",
                "unlisted-token" => "unlisted@example.com",
                _ => return Err(AppError::UnauthenticatedError),
            };
            Ok(AccessIdentity {
                email: email.into(),
            })
        }
    }

    struct FakeUsers;

    #[async_trait]
    impl AdminUserService for FakeUsers {
        async fn list_users(&self) -> AppResult<Vec<AdminUser>> {
            Ok(vec![])
        }
        async fn find_user(&self, email: &str) -> AppResult<Option<AdminUser>> {
            let role = match email {
                "admin@example.com" => AdminRole::Admin,
                "reviewer@example.com" => AdminRole::Reviewer,
                _ => return Ok(None),
            };
            Ok(Some(AdminUser {
                email: email.into(),
                role,
                created_at: Utc::now(),
            }))
        }
        async fn put_user(&self, _: &str, _: AdminRole) -> AppResult<AdminUser> {
            unreachable!()
        }
        async fn delete_user(&self, _: &str, _: Option<&str>) -> AppResult<()> {
            unreachable!()
        }
    }

    fn auth(enabled: bool, dev_email: Option<&str>) -> AdminAuth {
        AdminAuth::new(
            FireAuth::new("dummy-key".to_string()),
            Arc::new(FakeVerifier { enabled }),
            Arc::new(FakeUsers),
            dev_email.map(str::to_string),
        )
    }

    fn app(auth: &AdminAuth) -> Router {
        let who = |Extension(i): Extension<AdminIdentity>| async move {
            format!("{}:{}:{:?}", i.email, i.role.as_str(), i.method)
        };
        Router::new()
            .merge(with_auth(
                Router::new().route("/admin-only", get(who)),
                auth,
            ))
            .merge(with_review_auth(
                Router::new().route("/review", get(who)),
                auth,
            ))
    }

    async fn call(app: &Router, path: &str, headers: &[(&str, &str)]) -> (StatusCode, String) {
        let mut req = Request::builder().uri(path);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        let res = app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn no_credentials_is_401() {
        let app = app(&auth(true, None));
        assert_eq!(call(&app, "/review", &[]).await.0, StatusCode::UNAUTHORIZED);
        assert_eq!(
            call(&app, "/admin-only", &[]).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn access_admin_passes_both() {
        let app = app(&auth(true, None));
        let h = [("Cf-Access-Jwt-Assertion", "admin-token")];
        let (s, body) = call(&app, "/admin-only", &h).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body, "admin@example.com:admin:Access");
        assert_eq!(call(&app, "/review", &h).await.0, StatusCode::OK);
    }

    #[tokio::test]
    async fn access_reviewer_passes_review_only() {
        let app = app(&auth(true, None));
        let h = [("Cf-Access-Jwt-Assertion", "reviewer-token")];
        let (s, body) = call(&app, "/review", &h).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body, "reviewer@example.com:reviewer:Access");
        assert_eq!(call(&app, "/admin-only", &h).await.0, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn access_user_not_in_table_is_403_and_bad_token_is_401() {
        let app = app(&auth(true, None));
        let unlisted = [("Cf-Access-Jwt-Assertion", "unlisted-token")];
        assert_eq!(
            call(&app, "/review", &unlisted).await.0,
            StatusCode::FORBIDDEN
        );
        let bad = [("Cf-Access-Jwt-Assertion", "forged")];
        assert_eq!(
            call(&app, "/review", &bad).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn access_token_from_cookie() {
        let app = app(&auth(true, None));
        let h = [(
            "Cookie",
            "theme=dark; CF_Authorization=admin-token; other=1",
        )];
        assert_eq!(call(&app, "/review", &h).await.0, StatusCode::OK);
        let other = [("Cookie", "CF_AuthorizationX=admin-token")];
        assert_eq!(
            call(&app, "/review", &other).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn access_header_is_ignored_when_access_is_not_configured() {
        // Access の設定がない(Firebase だけ)とき、署名のヘッダーを信用しない
        let app = app(&auth(false, None));
        let h = [("Cf-Access-Jwt-Assertion", "admin-token")];
        assert_eq!(call(&app, "/review", &h).await.0, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn dev_email_passes_as_admin() {
        let app = app(&auth(false, Some("dev@example.com")));
        let (s, body) = call(&app, "/admin-only", &[]).await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(body, "dev@example.com:admin:Dev");
    }

    #[test]
    fn access_token_prefers_header_over_cookie() {
        let mut h = HeaderMap::new();
        h.insert("cf-access-jwt-assertion", "from-header".parse().unwrap());
        h.insert(
            header::COOKIE,
            "CF_Authorization=from-cookie".parse().unwrap(),
        );
        assert_eq!(access_token(&h).as_deref(), Some("from-header"));
        h.remove("cf-access-jwt-assertion");
        assert_eq!(access_token(&h).as_deref(), Some("from-cookie"));
        h.insert(header::COOKIE, "CF_Authorization=".parse().unwrap());
        assert_eq!(access_token(&h), None);
    }

    // ---- CSRF ----

    async fn post(app: &Router, path: &str, headers: &[(&str, &str)]) -> StatusCode {
        let mut req = Request::builder().method("POST").uri(path);
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        app.clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    fn app_with_post(auth: &AdminAuth) -> Router {
        with_auth(
            Router::new().route("/write", axum::routing::post(|| async { "ok" })),
            auth,
        )
    }

    #[tokio::test]
    async fn access_write_from_another_site_is_rejected() {
        let app = app_with_post(&auth(true, None));
        let token = ("Cf-Access-Jwt-Assertion", "admin-token");
        // ブラウザーが cross-site と示す(フォームの自動送信など)
        for site in ["cross-site", "same-site"] {
            let h = [token, ("Sec-Fetch-Site", site)];
            assert_eq!(
                post(&app, "/write", &h).await,
                StatusCode::FORBIDDEN,
                "{site}"
            );
        }
        // 画面からの要求
        assert_eq!(
            post(&app, "/write", &[token, ("Sec-Fetch-Site", "same-origin")]).await,
            StatusCode::OK
        );
        // Sec-Fetch-Site がなくても、Origin が Host と違えば断る
        let h = [
            token,
            ("Origin", "https://evil.example"),
            ("Host", "admin.sotalive.net"),
        ];
        assert_eq!(post(&app, "/write", &h).await, StatusCode::FORBIDDEN);
        let h = [
            token,
            ("Origin", "https://admin.sotalive.net"),
            ("Host", "admin.sotalive.net"),
        ];
        assert_eq!(post(&app, "/write", &h).await, StatusCode::OK);
        // ブラウザーでないもの(ヘッダーなし)は通す
        assert_eq!(post(&app, "/write", &[token]).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn cross_site_reads_are_not_blocked() {
        let app = app(&auth(true, None));
        let h = [
            ("Cf-Access-Jwt-Assertion", "admin-token"),
            ("Sec-Fetch-Site", "cross-site"),
        ];
        assert_eq!(call(&app, "/review", &h).await.0, StatusCode::OK);
    }
}
