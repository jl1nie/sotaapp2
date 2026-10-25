//! 公開の読み取り API に `Cache-Control` を付ける。
//!
//! 更新の周期を知っているのは API 自身なので、「この応答は何秒新しい」と宣言するのも API の役目。
//! Cloudflare は、キャッシュルールを足すと、この宣言に従って応答を覚え、同じ要求を Fly.io まで届けない
//! (JSON の API は、既定ではキャッシュされない)。MyACT を開いている人が増えても、
//! 小さなマシン(shared-cpu-1x)が、同じスポットを毎分組み立て直さずに済む。
//!
//! - 付けるのは、GET で成功(2xx)した応答だけ。エラーは覚えさせない。
//! - すでに `Cache-Control` がある応答、`Set-Cookie` がある応答には付けない。
//! - 利用者ごとの応答(POTA ハンターのログ `log_id=` つきの要求)は、共有のキャッシュに載せない(`private`)。
//! - 管理系・ログ・認証・地図の本体(`/sota/maps`)には付けない。地図の本体を覚えさせると、版の問い合わせだけが
//!   新しく、古い地図を新しい版として端末に保存してしまう恐れがある。

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method},
    middleware::{self, Next},
    response::Response,
    Router,
};

/// スポット・アラートなど、数十秒ごとに変わるもの(取得は `SPOT_INTERVAL` 90〜120 秒ごと)
pub const LIVE_SECS: u32 = 30;
/// 地図の版の問い合わせ(管理画面で直したときだけ変わる)
pub const VERSION_SECS: u32 = 60;
/// 地磁気(3 時間ごとの更新)
pub const GEOMAG_SECS: u32 = 600;
/// 山・公園の検索(一覧はほとんど変わらない)
pub const SEARCH_SECS: u32 = 300;
/// 市区町村と JCC/JCG の表(管理者が CSV を取り込んだときだけ変わる)
pub const LOCATOR_SECS: u32 = 3600;

/// このルーターのルートに、`max_age` 秒の `Cache-Control` を付ける(`route_layer`。先に足したルートだけが対象)
pub fn cached<S: Clone + Send + Sync + 'static>(router: Router<S>, max_age: u32) -> Router<S> {
    router.route_layer(middleware::from_fn_with_state(max_age, apply))
}

/// 利用者ごとの応答になる要求(POTA ハンターのログの番号を渡している)
fn is_personal(req: &Request) -> bool {
    req.uri()
        .query()
        .is_some_and(|q| q.split('&').any(|kv| kv.starts_with("log_id=")))
}

async fn apply(State(max_age): State<u32>, req: Request, next: Next) -> Response {
    let is_get = req.method() == Method::GET;
    let personal = is_personal(&req);
    let mut res = next.run(req).await;
    let headers = res.headers();
    if is_get
        && res.status().is_success()
        && !headers.contains_key(header::CACHE_CONTROL)
        && !headers.contains_key(header::SET_COOKIE)
    {
        let scope = if personal { "private" } else { "public" };
        if let Ok(v) = HeaderValue::from_str(&format!("{scope}, max-age={max_age}")) {
            res.headers_mut().insert(header::CACHE_CONTROL, v);
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        response::IntoResponse,
        routing::{get, post},
    };
    use tower::ServiceExt;

    fn app() -> Router {
        cached(
            Router::new()
                .route("/ok", get(|| async { "ok" }).post(|| async { "posted" }))
                .route("/missing", get(|| async { StatusCode::NOT_FOUND }))
                .route("/boom", get(|| async { StatusCode::INTERNAL_SERVER_ERROR }))
                .route(
                    "/own",
                    get(|| async {
                        ([(header::CACHE_CONTROL, "no-store")], "own").into_response()
                    }),
                )
                .route(
                    "/cookie",
                    get(|| async { ([(header::SET_COOKIE, "a=b")], "c").into_response() }),
                ),
            30,
        )
        .route("/uncached", post(|| async { "x" }).get(|| async { "y" }))
    }

    async fn call(method: &str, uri: &str) -> (StatusCode, Option<String>) {
        let req = HttpRequest::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let res = app().oneshot(req).await.unwrap();
        let cc = res
            .headers()
            .get(header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string());
        (res.status(), cc)
    }

    #[tokio::test]
    async fn successful_get_is_public_with_max_age() {
        assert_eq!(
            call("GET", "/ok").await,
            (StatusCode::OK, Some("public, max-age=30".into()))
        );
        // クエリが違っても付く(キャッシュの単位は URL 全体)
        assert_eq!(
            call("GET", "/ok?pat_ref=%5EJA").await.1,
            Some("public, max-age=30".into())
        );
    }

    #[tokio::test]
    async fn requests_with_a_hunter_log_id_are_private() {
        assert_eq!(
            call("GET", "/ok?log_id=abc&by_ref=null").await.1,
            Some("private, max-age=30".into())
        );
        assert_eq!(
            call("GET", "/ok?x=1&log_id=abc").await.1,
            Some("private, max-age=30".into())
        );
        // log_id を名前の一部に含むだけの別の項目は、個人のものとはしない
        assert_eq!(
            call("GET", "/ok?xlog_id=abc").await.1,
            Some("public, max-age=30".into())
        );
    }

    #[tokio::test]
    async fn errors_and_non_get_are_not_cached() {
        assert_eq!(call("GET", "/missing").await, (StatusCode::NOT_FOUND, None));
        assert_eq!(
            call("GET", "/boom").await,
            (StatusCode::INTERNAL_SERVER_ERROR, None)
        );
        assert_eq!(call("POST", "/ok").await, (StatusCode::OK, None));
    }

    #[tokio::test]
    async fn existing_cache_control_and_cookies_are_respected() {
        assert_eq!(call("GET", "/own").await.1, Some("no-store".into()));
        assert_eq!(call("GET", "/cookie").await.1, None);
    }

    #[tokio::test]
    async fn routes_added_after_the_layer_are_not_affected() {
        assert_eq!(call("GET", "/uncached").await, (StatusCode::OK, None));
    }
}
