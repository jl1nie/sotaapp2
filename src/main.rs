use anyhow::{Error, Result};
use std::collections::HashMap;

use axum::{
    extract::{Multipart, Query, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{sqlite::SqlitePoolOptions, FromRow, Pool, QueryBuilder, Sqlite, SqlitePool};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};

#[derive(Debug, Clone)]
pub struct AppState {
    pub pool: Pool<Sqlite>,
}

impl AppState {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct PotaCSVData {
    id: Option<u32>,
    potaref: String,
    name: String,
    location: String,
    locid: String,
    parktype: String,
    namek: String,
    lat: f32,
    lng: f32,
    updates: Option<String>,
}

async fn insert_list(pool: &SqlitePool, pota_list: &[PotaCSVData]) -> Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
            DELETE FROM PotaCSVData
        "#
    )
    .execute(&mut *tx)
    .await?;

    for (id, pota) in pota_list.iter().enumerate() {
        let id = id as u32;
        sqlx::query!(
        r#"
            INSERT INTO PotaCSVData (id, potaref, name, location, locid, parktype, namek, lat, lng, updates)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        id,
        pota.potaref,
        pota.name,
        pota.location,
        pota.locid,
        pota.parktype,
        pota.namek,
        pota.lat,
        pota.lng,
        pota.updates)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn upload_csv(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, String> {
    if let Some(field) = multipart.next_field().await.unwrap() {
        let data = field.bytes().await.unwrap();
        let data = String::from_utf8(data.to_vec()).unwrap();

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(data.as_bytes());

        let mut pota_ref_list = Vec::new();
        for result in rdr.deserialize() {
            let pota_ref: PotaCSVData = result.unwrap();
            pota_ref_list.push(pota_ref);
        }

        insert_list(&state.pool, &pota_ref_list).await.unwrap();

        return Ok(Json(json!({
            "length": pota_ref_list.len()
        })));
    }
    Ok(Json(json!({
        "length": 0
    })))
}

async fn pota_list(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, String> {
    let page: Option<i32> = params
        .get("page")
        .and_then(|value| value.to_string().parse::<i32>().ok());

    let count: Option<i32> = params
        .get("count")
        .and_then(|value| value.to_string().parse::<i32>().ok());

    let update: Option<bool> = params
        .get("update")
        .and_then(|value| value.to_string().parse::<bool>().ok());

    let count = count.unwrap_or(10); // 指定されない場合は10件ページャ
    let offset = count * page.unwrap_or(0); // ページ指定がない場合は先頭ページ
    let update = update.unwrap_or(true);

    let init = if update {
        "SELECT * FROM PotaCSVData WHERE updates IS NOT NULL ORDER BY id LIMIT "
    } else {
        "SELECT * FROM PotaCSVData ORDER BY id LIMIT "
    };

    let entities: Vec<PotaCSVData> = QueryBuilder::new(init)
        .push_bind(count)
        .push(" OFFSET ")
        .push_bind(offset)
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(json!({
        "list": entities
    })))
}

async fn pota_admin(State(state): State<AppState>) -> Result<impl IntoResponse, String> {
    Ok(Html(include_str!("../views/index.html")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 環境変数初期化
    dotenvy::dotenv().unwrap();

    // logger初期化
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_ansi(true)
        .init();

    // DBへの接続プール作成
    let uri = std::env::var("DATABASE_URL")?;
    let pool = SqlitePoolOptions::new()
        //.max_connections(4)
        .connect(&uri)
        .await?;

    // Router定義
    let app = Router::new()
        .route("/api/v2/pota-upload", post(upload_csv))
        .route("/api/v2/pota-list", get(pota_list))
        .route("/api/v2/potaadmin", get(pota_admin))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
                .on_request(DefaultOnRequest::new().level(tracing::Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(tracing::Level::DEBUG)),
        )
        .with_state(AppState::new(pool));

    // サーバー起動
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
