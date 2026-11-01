//! 管理画面(SvelteKit の静的ファイル)の配信と、そのキャッシュの指示。
//!
//! - `/_app/immutable/*` はファイル名にハッシュが入っているので、長く覚えさせる。
//! - それ以外(`index.html`・`/_app/version.json`・`robots.txt`)は `no-cache`。毎回サーバーに確かめさせる。
//!   ヘッダーがないと、ブラウザーは `Last-Modified` から有効期間を推測して古い HTML を使い回し、
//!   デプロイで消えた古い JS を指したまま画面が止まる。
//! - `/_app/*` にないファイルは、`index.html` にせず 404 を返す。JS の要求に HTML を 200 で返すと、
//!   読み込みの失敗が原因の分かりにくい画面の停止になる。

use std::path::Path;

use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::{self, Next},
    response::Response,
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

const IMMUTABLE_PREFIX: &str = "/_app/immutable/";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const REVALIDATE: &str = "no-cache";

/// `dir`(`static`)を配信する。見つからない道は `index.html`(SPA)。ただし `/_app/*` は 404
pub fn static_router(dir: impl AsRef<Path>) -> Router {
    let dir = dir.as_ref();
    let spa = ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")));
    Router::new()
        .nest_service("/_app", ServeDir::new(dir.join("_app")))
        .fallback_service(spa)
        .layer(middleware::from_fn(cache_control))
}

async fn cache_control(req: Request, next: Next) -> Response {
    let immutable = req.uri().path().starts_with(IMMUTABLE_PREFIX);
    let mut res = next.run(req).await;
    if !res.headers().contains_key(header::CACHE_CONTROL) {
        // 見つかったハッシュ付きのファイルだけ長く覚えさせる。404 は覚えさせない
        let v = if immutable && res.status().is_success() {
            IMMUTABLE
        } else {
            REVALIDATE
        };
        res.headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(v));
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::StatusCode};
    use tower::ServiceExt;

    fn site() -> (std::path::PathBuf, Router) {
        let dir = std::env::temp_dir().join(format!("sotaapp2-static-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("_app/immutable/entry")).unwrap();
        std::fs::write(dir.join("index.html"), "<html>spa</html>").unwrap();
        std::fs::write(dir.join("robots.txt"), "User-agent: *").unwrap();
        std::fs::write(dir.join("_app/version.json"), "{\"version\":\"1\"}").unwrap();
        std::fs::write(dir.join("_app/immutable/entry/app.AAAA.js"), "export{}").unwrap();
        let app = static_router(&dir);
        (dir, app)
    }

    async fn get(app: &Router, path: &str) -> (StatusCode, Option<String>, String) {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let cc = res
            .headers()
            .get(header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string());
        let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        (status, cc, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn html_and_unhashed_files_must_be_revalidated() {
        let (dir, app) = site();
        for path in ["/", "/index.html", "/robots.txt", "/_app/version.json"] {
            let (s, cc, _) = get(&app, path).await;
            assert_eq!(s, StatusCode::OK, "{path}");
            assert_eq!(cc.as_deref(), Some("no-cache"), "{path}");
        }
        // SPA の道は index.html を返し、これも毎回確かめさせる
        let (s, cc, body) = get(&app, "/admin-console/summit-maps").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(cc.as_deref(), Some("no-cache"));
        assert_eq!(body, "<html>spa</html>");
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn hashed_assets_are_cached_for_a_long_time() {
        let (dir, app) = site();
        let (s, cc, body) = get(&app, "/_app/immutable/entry/app.AAAA.js").await;
        assert_eq!(s, StatusCode::OK);
        assert_eq!(cc.as_deref(), Some(IMMUTABLE));
        assert_eq!(body, "export{}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn missing_app_files_are_404_not_index_html() {
        let (dir, app) = site();
        // デプロイで消えた古い JS。HTML を 200 で返してはいけない。404 は覚えさせない
        let (s, cc, body) = get(&app, "/_app/immutable/entry/app.OLD0.js").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        assert_eq!(cc.as_deref(), Some("no-cache"));
        assert!(!body.contains("spa"));
        let (s, _, _) = get(&app, "/_app/nothing.js").await;
        assert_eq!(s, StatusCode::NOT_FOUND);
        std::fs::remove_dir_all(dir).ok();
    }
}
