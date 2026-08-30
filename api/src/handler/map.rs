use axum::{response::Html, routing::get, Router};
use utoipa::OpenApi;

use crate::model::map::MapParam;
use crate::model::param::ValidatedQuery;
use common::utils::maidenhead;
use registry::AppState;

/// Map API
#[derive(OpenApi)]
#[openapi(
    paths(show_map),
    components(schemas(MapParam, crate::model::map::TileLayer)),
    tags((name = "map", description = "座標を中心とした地図表示API"))
)]
pub struct MapApi;

/// HTMLに埋め込む文字列をエスケープする
fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// 指定座標を中心とした地図ページを生成する
///
/// 座標・ズーム・タイル種別はdata属性としてHTMLに埋め込み、
/// Leaflet(CDN)を読み込んだスクリプトから参照する。
/// 利用者が指定した`label`はエスケープしてから埋め込む。
fn render_map_html(param: &MapParam) -> String {
    let MapParam { lat, lon, .. } = param;
    let tile = param.tile_layer();
    let zoom = param.effective_zoom();
    let grid = maidenhead(*lon, *lat);

    let label = param.label.as_deref().unwrap_or_default();
    let title = if label.is_empty() {
        format!("{:.6}, {:.6}", lat, lon)
    } else {
        label.to_string()
    };
    let title = escape_html(&title);
    let label = escape_html(label);
    let coords = format!("{:.6}, {:.6}", lat, lon);

    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>{title} - SOTAApp2</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css">
<style>
  html, body {{ height: 100%; margin: 0; font-family: system-ui, sans-serif; }}
  body {{ display: flex; flex-direction: column; }}
  header {{ padding: 8px 12px; background: #1f2937; color: #f9fafb; font-size: 14px; }}
  header .title {{ font-weight: 600; }}
  header .meta {{ opacity: 0.85; }}
  header a {{ color: #93c5fd; }}
  #map {{ flex: 1 1 auto; }}
  .fallback {{ padding: 12px; font-size: 14px; }}
</style>
</head>
<body>
<header>
  <span class="title">{title}</span>
  <span class="meta">/ {coords} / GL: {grid} /
    <a href="https://www.google.com/maps?q={lat},{lon}" target="_blank" rel="noopener">Google Maps</a> ·
    <a href="https://www.openstreetmap.org/?mlat={lat}&amp;mlon={lon}#map={zoom}/{lat}/{lon}" target="_blank" rel="noopener">OSM</a> ·
    <a href="https://maps.gsi.go.jp/#{zoom}/{lat}/{lon}/" target="_blank" rel="noopener">地理院地図</a>
  </span>
</header>
<div id="map"
     data-lat="{lat}"
     data-lon="{lon}"
     data-zoom="{zoom}"
     data-label="{label}"
     data-tile-url="{tile_url}"
     data-tile-attribution="{tile_attribution}"
     data-tile-max-zoom="{tile_max_zoom}"></div>
<noscript><p class="fallback">地図の表示にはJavaScriptが必要です。上記のリンクから外部地図サービスで位置を確認できます。</p></noscript>
<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
<script>
  (function () {{
    var el = document.getElementById('map');
    if (typeof L === 'undefined') {{
      el.innerHTML = '<p class="fallback">地図ライブラリを読み込めませんでした。上記のリンクから外部地図サービスで位置を確認できます。</p>';
      return;
    }}
    var lat = parseFloat(el.dataset.lat);
    var lon = parseFloat(el.dataset.lon);
    var map = L.map(el).setView([lat, lon], parseInt(el.dataset.zoom, 10));
    L.tileLayer(el.dataset.tileUrl, {{
      attribution: el.dataset.tileAttribution,
      maxZoom: parseInt(el.dataset.tileMaxZoom, 10)
    }}).addTo(map);
    var popup = (el.dataset.label ? el.dataset.label + '<br>' : '') +
      lat.toFixed(6) + ', ' + lon.toFixed(6) + '<br>GL: {grid}';
    L.marker([lat, lon]).addTo(map).bindPopup(popup).openPopup();
  }})();
</script>
</body>
</html>
"#,
        title = title,
        label = label,
        coords = coords,
        grid = grid,
        lat = lat,
        lon = lon,
        zoom = zoom,
        tile_url = tile.url_template(),
        tile_attribution = escape_html(tile.attribution()),
        tile_max_zoom = tile.max_zoom(),
    )
}

/// 指定座標の地図を表示
#[utoipa::path(
    get,
    path = "/api/v2/map",
    params(MapParam),
    responses(
        (status = 200, description = "地図ページ(HTML)", content_type = "text/html"),
        (status = 400, description = "パラメータの解析に失敗"),
        (status = 422, description = "パラメータが範囲外"),
    ),
    tag = "map"
)]
async fn show_map(ValidatedQuery(param): ValidatedQuery<MapParam>) -> Html<String> {
    Html(render_map_html(&param))
}

pub fn build_map_routers() -> Router<AppState> {
    let routers = Router::new().route("/", get(show_map));
    Router::new().nest("/map", routers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::map::TileLayer;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn param(label: Option<&str>, tile: Option<TileLayer>) -> MapParam {
        MapParam {
            lat: 35.360_556,
            lon: 138.727_778,
            zoom: None,
            label: label.map(|s| s.to_string()),
            tile,
        }
    }

    fn test_router() -> Router {
        Router::new().route("/api/v2/map", get(show_map))
    }

    async fn get_map(uri: &str) -> (StatusCode, String) {
        let response = test_router()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html(r#"<script>alert("x&y")</script>"#),
            "&lt;script&gt;alert(&quot;x&amp;y&quot;)&lt;/script&gt;"
        );
        assert_eq!(escape_html("it's"), "it&#39;s");
        assert_eq!(escape_html("富士山"), "富士山");
    }

    #[test]
    fn test_render_contains_coordinates_and_grid_locator() {
        let html = render_map_html(&param(None, None));
        assert!(html.contains(r#"data-lat="35.360556""#));
        assert!(html.contains(r#"data-lon="138.727778""#));
        // 既定のズームレベル
        assert!(html.contains(r#"data-zoom="15""#));
        // グリッドロケータ(メイデンヘッド)
        assert!(html.contains("PM95"));
    }

    #[test]
    fn test_render_uses_osm_tile_by_default() {
        let html = render_map_html(&param(None, None));
        assert!(html.contains("tile.openstreetmap.org"));
        assert!(!html.contains("cyberjapandata.gsi.go.jp/xyz"));
    }

    #[test]
    fn test_render_uses_gsi_tile_when_requested() {
        let html = render_map_html(&param(None, Some(TileLayer::Gsi)));
        assert!(html.contains("cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png"));
    }

    #[test]
    fn test_render_escapes_label() {
        let html = render_map_html(&param(Some("<script>alert(1)</script>"), None));
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn test_render_escapes_label_breaking_out_of_attribute() {
        let html = render_map_html(&param(Some(r#"" onload="alert(1)"#), None));
        assert!(!html.contains(r#"onload="alert(1)"#));
    }

    #[tokio::test]
    async fn test_show_map_returns_html() {
        let response = test_router()
            .oneshot(
                Request::builder()
                    .uri("/api/v2/map?lat=35.360556&lon=138.727778")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn test_show_map_with_label_and_zoom() {
        let (status, body) =
            get_map("/api/v2/map?lat=35.360556&lon=138.727778&zoom=12&label=富士山").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(r#"data-zoom="12""#));
        assert!(body.contains("富士山"));
    }

    /// build_map_routersと同じネスト構成で`/api/v2/map`に到達できること
    #[tokio::test]
    async fn test_nested_route_matches_without_trailing_slash() {
        let app = Router::new().nest(
            "/api/v2",
            Router::new().nest("/map", Router::new().route("/", get(show_map))),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v2/map?lat=35&lon=139")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_show_map_requires_coordinates() {
        let (status, _) = get_map("/api/v2/map").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_show_map_rejects_out_of_range_coordinates() {
        let (status, _) = get_map("/api/v2/map?lat=91&lon=139").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_show_map_rejects_non_finite_coordinates() {
        let (status, _) = get_map("/api/v2/map?lat=NaN&lon=139").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, _) = get_map("/api/v2/map?lat=35&lon=inf").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_show_map_rejects_out_of_range_zoom() {
        let (status, _) = get_map("/api/v2/map?lat=35&lon=139&zoom=30").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}
