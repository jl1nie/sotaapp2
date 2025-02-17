use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::{routing::post, Router};
use axum::{Extension, Json};
use firebase_auth_sdk::FireAuth;

use crate::model::auth::{AuthRequest, AuthResponse};
use registry::AppState;

pub async fn sign_in(
    service: Extension<FireAuth>,
    Json(creds_request): Json<AuthRequest>,
) -> impl IntoResponse {
    match service
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

pub async fn sign_up(
    service: Extension<FireAuth>,
    Json(creds_request): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    match service
        .sign_up_email(
            creds_request.email.as_str(),
            creds_request.password.as_str(),
            false,
        )
        .await
    {
        Ok(_) => {
            let msg = AuthResponse {
                message: String::from("Successfully Registered, Please login...!"),
            };
            Ok(Json(msg))
        }
        Err(ex) => {
            eprintln!("{:?}", ex);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

pub async fn sign_out(
    service: Extension<FireAuth>,
    Json(creds_request): Json<AuthRequest>,
) -> impl IntoResponse {
    match service
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
pub fn build_auth_routers() -> Router<AppState> {
    //let auth_service = FireAuth::new(String::from("YOUR-FIREBASE-KEY"));

    let routers = Router::new()
        .route("/signin", post(sign_in))
        .route("/signup", post(sign_up))
        .route("/signout", post(sign_out));

    Router::new().nest("/auth", routers)
}
