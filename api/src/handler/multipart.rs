//! マルチパートファイルアップロードのヘルパー関数

use axum::extract::{DefaultBodyLimit, Multipart};
use common::error::{AppError, AppResult};

/// 利用者(認証なし)のアップロードの上限(MB)。v2 の全ルートの既定値。
///
/// 本番のマシン(256MB。アプリの常駐は 80MB ほど)では、解析で入力の十数倍のメモリを使っても落ちない大きさに抑える
pub const USER_UPLOAD_MB: usize = 2;

/// 管理者(認証あり)のアップロードの上限(MB)。参照リストのインポートやテンプレート画像に使う。
///
/// 管理者だけが叩くので、同時に何本も来ない前提で利用者より大きくする(全世界の山岳リストのような
/// 数十MBのファイルはメモリに乗らないので受け付けない。それらは定期ジョブが取得する)
pub const ADMIN_UPLOAD_MB: usize = 8;

/// 利用者のアップロードの上限
pub fn user_upload_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(USER_UPLOAD_MB * 1024 * 1024)
}

/// 管理者のアップロードの上限。利用者の上限より内側でかけるので、こちらが効く
pub fn admin_upload_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(ADMIN_UPLOAD_MB * 1024 * 1024)
}

/// マルチパートからUTF-8文字列データを抽出する
pub async fn extract_text_file(multipart: &mut Multipart) -> AppResult<String> {
    let data = extract_file(multipart).await?;
    String::from_utf8(data)
        .map_err(|_| AppError::UnprocessableEntity("ファイルがUTF-8形式ではありません".to_string()))
}

/// マルチパートから最初のファイルをそのまま取り出す(gzip など文字列でないもの)
pub async fn extract_file(multipart: &mut Multipart) -> AppResult<Vec<u8>> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| {
            AppError::UnprocessableEntity(format!("マルチパートの読み込みに失敗しました: {}", e))
        })?
        .ok_or_else(|| AppError::UnprocessableEntity("ファイルが送信されていません".to_string()))?;

    let data = field.bytes().await.map_err(|e| {
        AppError::UnprocessableEntity(format!("ファイルの読み込みに失敗しました: {}", e))
    })?;
    Ok(data.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Bytes, http::StatusCode, routing::post, Router};
    use tower::ServiceExt;

    // v2 と同じく、全体に利用者の上限をかけ、その内側で管理者のルートだけ上限を広げる。
    // 本物の認証ミドルウェアは Firebase に問い合わせるので、上限のかけ方だけを同じにした最小のルーターで確かめる
    fn app() -> Router {
        let admin = Router::new()
            .route(
                "/admin",
                post(|b: Bytes| async move { b.len().to_string() }),
            )
            .layer(admin_upload_limit());
        Router::new()
            .route("/user", post(|b: Bytes| async move { b.len().to_string() }))
            .merge(admin)
            .layer(user_upload_limit())
    }

    async fn status(path: &str, len: usize) -> StatusCode {
        let req = axum::http::Request::builder()
            .method("POST")
            .uri(path)
            .body(axum::body::Body::from(vec![b'x'; len]))
            .unwrap();
        app().oneshot(req).await.unwrap().status()
    }

    #[test]
    fn with_auth_applies_admin_limit() {
        assert!(include_str!("auth.rs").contains(".layer(admin_upload_limit())"));
        assert!(include_str!("v2.rs").contains(".layer(user_upload_limit())"));
    }

    #[tokio::test]
    async fn user_routes_reject_over_user_limit() {
        let mb = 1024 * 1024;
        assert_eq!(status("/user", USER_UPLOAD_MB * mb).await, StatusCode::OK);
        assert_eq!(
            status("/user", USER_UPLOAD_MB * mb + 1).await,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn admin_routes_accept_up_to_admin_limit() {
        let mb = 1024 * 1024;
        assert_eq!(
            status("/admin", USER_UPLOAD_MB * mb + 1).await,
            StatusCode::OK
        );
        assert_eq!(status("/admin", ADMIN_UPLOAD_MB * mb).await, StatusCode::OK);
        assert_eq!(
            status("/admin", ADMIN_UPLOAD_MB * mb + 1).await,
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }
}
