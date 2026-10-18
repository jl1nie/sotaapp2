use axum::{response::Html, routing::get, Json, Router};
use shaku_axum::Inject;
use utoipa::OpenApi;

use crate::model::map::{MapParam, MapView, NearbyRef};
use crate::model::param::ValidatedQuery;
use domain::model::event::FindRefBuilder;
use registry::{AppRegistry, AppState};
use service::services::UserService;

/// 近傍検索で取得する件数の上限(円に絞り直す前)
const NEARBY_QUERY_LIMIT: i32 = 500;

/// Map API
#[derive(OpenApi)]
#[openapi(
    paths(show_map, get_map_view),
    components(schemas(
        MapParam,
        MapView,
        crate::model::map::TileLayer,
        crate::model::map::TileLayerView,
        crate::model::map::ExternalMapLinks,
        crate::model::map::NearbyRef,
        crate::model::map::RefKind,
    )),
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

/// 指定座標を中心とした地図ページを生成する(近傍のSOTA/POTAを含む)
///
/// 座標・ズーム・タイル種別はdata属性としてHTMLに埋め込み、
/// Leaflet(CDN)を読み込んだスクリプトから参照する。
/// 利用者が指定した`label`はエスケープしてから埋め込む。
fn render_map_html(param: &MapParam, nearby: &[NearbyRef]) -> String {
    let view = MapView::from(param);
    let (lat, lon, zoom, grid) = (view.latitude, view.longitude, view.zoom, view.maidenhead);

    let label = param.label.as_deref().unwrap_or_default();
    let title = if label.is_empty() {
        format!("{:.6}, {:.6}", lat, lon)
    } else {
        label.to_string()
    };
    let title = escape_html(&title);
    let label = escape_html(label);
    let coords = format!("{:.6}, {:.6}", lat, lon);
    // 近傍のリファレンスはJSONとしてdata属性に埋め込む(属性値としてエスケープ)
    let nearby_json = escape_html(&serde_json::to_string(nearby).unwrap_or_else(|_| "[]".into()));
    let radius = param.radius_m();

    // iframe埋め込み時はヘッダを省略して地図だけを表示する
    let header = if param.is_embed() {
        String::new()
    } else {
        format!(
            r#"<header>
  <span class="title">{title}</span>
  <span class="meta">/ {coords} / GL: {grid} /
    <a href="{google}" target="_blank" rel="noopener">Google Maps</a> ·
    <a href="{osm}" target="_blank" rel="noopener">OSM</a> ·
    <a href="{gsi}" target="_blank" rel="noopener">地理院地図</a>
  </span>
</header>
"#,
            title = title,
            coords = coords,
            grid = grid,
            google = escape_html(&view.external_links.google_maps),
            osm = escape_html(&view.external_links.open_street_map),
            gsi = escape_html(&view.external_links.gsi_maps),
        )
    };

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
{header}<div id="map"
     data-lat="{lat}"
     data-lon="{lon}"
     data-zoom="{zoom}"
     data-label="{label}"
     data-nearby="{nearby_json}"
     data-radius="{radius}"
     data-refs="{refs}"
     data-tile-url="{tile_url}"
     data-tile-attribution="{tile_attribution}"
     data-tile-max-zoom="{tile_max_zoom}"></div>
<noscript><p class="fallback">地図の表示にはJavaScriptが必要です。（{coords} / GL: {grid}）</p></noscript>
<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
<script>
  (function () {{
    var el = document.getElementById('map');
    if (typeof L === 'undefined') {{
      el.innerHTML = '<p class="fallback">地図ライブラリを読み込めませんでした。（{coords} / GL: {grid}）</p>';
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

    // 近傍のSOTA山岳・POTA公園。ポップアップはtextContentで組み立てる
    if (el.dataset.refs === 'true') {{
      L.circle([lat, lon], {{
        radius: parseInt(el.dataset.radius, 10),
        color: '#6b7280', weight: 1, fill: false, dashArray: '4'
      }}).addTo(map);
    }}
    var nearby = [];
    try {{ nearby = JSON.parse(el.dataset.nearby || '[]'); }} catch (e) {{}}
    nearby.forEach(function (r) {{
      var color = r.kind === 'sota' ? '#2563eb' : '#16a34a';
      var box = document.createElement('div');
      var head = document.createElement('strong');
      head.textContent = r.code;
      box.appendChild(head);
      var lines = [r.nameJ ? r.name + ' / ' + r.nameJ : r.name];
      if (r.kind === 'sota') {{
        lines.push(r.altM + ' m / ' + r.points + ' pt');
      }} else if (r.location) {{
        lines.push(r.location);
      }}
      lines.push('距離 ' + r.distanceM + ' m');
      lines.forEach(function (t) {{
        box.appendChild(document.createElement('br'));
        box.appendChild(document.createTextNode(t));
      }});
      L.circleMarker([r.latitude, r.longitude], {{
        radius: 8, color: color, fillColor: color, fillOpacity: 0.7, weight: 2
      }}).addTo(map).bindPopup(box).bindTooltip(r.code);
    }});
  }})();
</script>
</body>
</html>
"#,
        title = title,
        header = header,
        label = label,
        nearby_json = nearby_json,
        radius = radius,
        refs = param.want_refs() && !nearby.is_empty(),
        coords = coords,
        grid = grid,
        lat = lat,
        lon = lon,
        zoom = zoom,
        tile_url = view.tile.url_template,
        tile_attribution = escape_html(&view.tile.attribution),
        tile_max_zoom = view.tile.max_zoom,
    )
}

/// 中心の近くにあるSOTA山岳・POTA公園を検索する
///
/// 近傍は補助情報なので、検索に失敗しても地図自体は返す(空として扱う)。
async fn find_nearby(user_service: &dyn UserService, param: &MapParam) -> Vec<NearbyRef> {
    if !param.want_refs() {
        return Vec::new();
    }
    let radius = param.radius_m();
    let query = FindRefBuilder::default()
        .sota()
        .pota()
        .center(param.lon, param.lat, radius as f64)
        .limit(NEARBY_QUERY_LIMIT)
        .build();
    match user_service.find_references(query).await {
        Ok(result) => NearbyRef::collect(result, param.lat, param.lon, radius),
        Err(e) => {
            tracing::warn!("近傍のリファレンス検索に失敗: {e}");
            Vec::new()
        }
    }
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
async fn show_map(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<MapParam>,
) -> Html<String> {
    let nearby = find_nearby(&*user_service, &param).await;
    Html(render_map_html(&param, &nearby))
}

/// 地図情報(JSON)を組み立てる。`get_map_view`と同じ内容を返す
fn build_map_view(param: &MapParam, nearby: Vec<NearbyRef>) -> MapView {
    MapView::from(param).with_nearby(nearby)
}

/// 地図表示に必要な情報を取得（フロントエンド連携用）
///
/// 自前の地図コンポーネントで描画する場合や、`embedUrl`をiframeに
/// 埋め込む場合に利用する。パラメータは`GET /api/v2/map`と共通。
#[utoipa::path(
    get,
    path = "/api/v2/map/view",
    params(MapParam),
    responses(
        (status = 200, description = "地図情報", body = MapView),
        (status = 400, description = "パラメータの解析に失敗"),
        (status = 422, description = "パラメータが範囲外"),
    ),
    tag = "map"
)]
async fn get_map_view(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<MapParam>,
) -> Json<MapView> {
    let nearby = find_nearby(&*user_service, &param).await;
    Json(build_map_view(&param, nearby))
}

pub fn build_map_routers() -> Router<AppState> {
    let routers = Router::new()
        .route("/", get(show_map))
        .route("/view", get(get_map_view));
    Router::new().nest("/map", routers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::map::{RefKind, TileLayer};
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
            embed: None,
            refs: None,
            radius: None,
        }
    }

    fn sample_nearby() -> Vec<NearbyRef> {
        vec![NearbyRef {
            kind: RefKind::Sota,
            code: "JA/YN-001".to_string(),
            name: "Mt.Fuji".to_string(),
            name_j: Some("富士山".to_string()),
            latitude: 35.360_556,
            longitude: 138.727_778,
            distance_m: 0,
            alt_m: Some(3776),
            points: Some(10),
            location: None,
        }]
    }

    // 本番のハンドラはDI(Inject)が必要なので、テストでは同じ描画関数を
    // 固定の近傍データで呼ぶハンドラを使う(近傍検索自体は別途テスト)
    async fn test_show_map(ValidatedQuery(param): ValidatedQuery<MapParam>) -> Html<String> {
        Html(render_map_html(&param, &sample_nearby()))
    }

    async fn test_map_view(ValidatedQuery(param): ValidatedQuery<MapParam>) -> Json<MapView> {
        Json(build_map_view(&param, sample_nearby()))
    }

    fn test_router() -> Router {
        Router::new()
            .route("/api/v2/map", get(test_show_map))
            .route("/api/v2/map/view", get(test_map_view))
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
        let html = render_map_html(&param(None, None), &[]);
        assert!(html.contains(r#"data-lat="35.360556""#));
        assert!(html.contains(r#"data-lon="138.727778""#));
        // 既定のズームレベル
        assert!(html.contains(r#"data-zoom="15""#));
        // グリッドロケータ(メイデンヘッド)
        assert!(html.contains("PM95"));
    }

    #[test]
    fn test_render_uses_osm_tile_by_default() {
        let html = render_map_html(&param(None, None), &[]);
        assert!(html.contains("tile.openstreetmap.org"));
        assert!(!html.contains("cyberjapandata.gsi.go.jp/xyz"));
    }

    #[test]
    fn test_render_uses_gsi_tile_when_requested() {
        let html = render_map_html(&param(None, Some(TileLayer::Gsi)), &[]);
        assert!(html.contains("cyberjapandata.gsi.go.jp/xyz/std/{z}/{x}/{y}.png"));
    }

    #[test]
    fn test_render_escapes_label() {
        let html = render_map_html(&param(Some("<script>alert(1)</script>"), None), &[]);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    }

    #[test]
    fn test_render_escapes_label_breaking_out_of_attribute() {
        let html = render_map_html(&param(Some(r#"" onload="alert(1)"#), None), &[]);
        assert!(!html.contains(r#"onload="alert(1)"#));
    }

    #[test]
    fn test_embed_mode_omits_header() {
        let mut p = param(Some("富士山"), None);
        p.embed = Some(true);
        let html = render_map_html(&p, &[]);

        assert!(!html.contains("<header>"));
        // 地図本体とフォールバック時の座標表示は残る
        assert!(html.contains(r#"id="map""#));
        assert!(html.contains("35.360556, 138.727778"));
    }

    #[test]
    fn test_default_mode_includes_header() {
        let html = render_map_html(&param(Some("富士山"), None), &[]);
        assert!(html.contains("<header>"));
        assert!(html.contains("https://www.google.com/maps?q=35.360556,138.727778"));
    }

    #[tokio::test]
    async fn test_map_view_returns_json_for_frontend() {
        let (status, body) =
            get_map("/api/v2/map/view?lat=35.360556&lon=138.727778&zoom=12&label=富士山&tile=gsi")
                .await;
        assert_eq!(status, StatusCode::OK);

        let view: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(view["latitude"], 35.360556);
        assert_eq!(view["longitude"], 138.727778);
        assert_eq!(view["zoom"], 12);
        assert_eq!(view["label"], "富士山");
        assert_eq!(view["maidenhead"], "PM95ii76");
        assert_eq!(view["tile"]["kind"], "gsi");
        assert_eq!(view["tile"]["maxZoom"], 18);
        assert!(view["tile"]["urlTemplate"]
            .as_str()
            .unwrap()
            .contains("cyberjapandata.gsi.go.jp"));
        // フロントエンドがそのまま使えるURL
        assert_eq!(
            view["embedUrl"].as_str().unwrap(),
            "/api/v2/map?lat=35.360556&lon=138.727778&zoom=12&tile=gsi&label=%E5%AF%8C%E5%A3%AB%E5%B1%B1&embed=true"
        );
        assert!(!view["mapUrl"].as_str().unwrap().contains("embed"));
        assert!(view["externalLinks"]["googleMaps"]
            .as_str()
            .unwrap()
            .starts_with("https://www.google.com/maps?q="));
    }

    #[tokio::test]
    async fn test_map_view_rejects_invalid_params() {
        let (status, _) = get_map("/api/v2/map/view?lat=91&lon=139").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// `embedUrl`をそのまま叩けば埋め込み用ページが返ること
    #[tokio::test]
    async fn test_embed_url_round_trip() {
        let (_, body) = get_map("/api/v2/map/view?lat=35.360556&lon=138.727778&label=富士山").await;
        let view: serde_json::Value = serde_json::from_str(&body).unwrap();
        let embed_url = view["embedUrl"].as_str().unwrap();

        let (status, html) = get_map(embed_url).await;
        assert_eq!(status, StatusCode::OK);
        assert!(!html.contains("<header>"));
        assert!(html.contains("富士山"));
    }

    /// フロントエンド(`buildMapUrl`)が生成するURLをそのまま受け付けること
    ///
    /// `URLSearchParams`は空白を`+`、`/`を`%2F`にエンコードするため、
    /// サーバ側で正しくデコードされることを確認する。
    #[tokio::test]
    async fn test_accepts_url_built_by_frontend_helper() {
        let url = "/api/v2/map?lat=35.360556&lon=138.727778&zoom=12\
                   &label=%E5%AF%8C%E5%A3%AB%E5%B1%B1+JA%2FSO-001&tile=gsi";
        let (status, body) = get_map(url).await;

        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("富士山 JA/SO-001"));
        assert!(body.contains(r#"data-zoom="12""#));
        assert!(body.contains("cyberjapandata.gsi.go.jp"));
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
            Router::new().nest("/map", Router::new().route("/", get(test_show_map))),
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

    #[test]
    fn test_render_embeds_nearby_refs_as_escaped_json() {
        let mut nearby = sample_nearby();
        nearby[0].name = r#"<b>"x"</b>"#.to_string();
        let html = render_map_html(&param(None, None), &nearby);
        assert!(html.contains("JA/YN-001"));
        // 属性値の中に生の引用符・タグが出ない
        assert!(!html.contains(r#"<b>"x"</b>"#));
        assert!(html.contains(r#"data-refs="true""#));
    }

    #[test]
    fn test_render_without_nearby_has_refs_disabled() {
        let html = render_map_html(&param(None, None), &[]);
        assert!(html.contains(r#"data-refs="false""#));
        assert!(html.contains(r#"data-nearby="[]""#));
    }

    #[tokio::test]
    async fn test_view_includes_nearby() {
        let (status, body) = get_map("/api/v2/map/view?lat=35.360556&lon=138.727778").await;
        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["nearby"][0]["code"], "JA/YN-001");
        assert_eq!(json["nearby"][0]["kind"], "sota");
        assert_eq!(json["nearby"][0]["altM"], 3776);
    }

    #[tokio::test]
    async fn test_invalid_radius_is_rejected() {
        for uri in [
            "/api/v2/map?lat=35&lon=139&radius=0",
            "/api/v2/map?lat=35&lon=139&radius=10001",
        ] {
            let (status, body) = get_map(uri).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
            assert!(body.contains("radius"), "{uri}");
        }
    }
}
