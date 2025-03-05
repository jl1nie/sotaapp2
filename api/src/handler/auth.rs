use std::str::FromStr;

use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::Json;
use axum::{routing::post, Router};
use firebase_auth_sdk::FireAuth;

use crate::model::auth::AuthRequest;
use domain::model::id::UserId;
use registry::AppState;

pub async fn auth_middle(
    State(auth_service): State<FireAuth>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::info!("auth req {:?}", req);

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
            eprintln!("{:?}", ex);
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
