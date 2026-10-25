use super::cache::{cached, LIVE_SECS, SEARCH_SECS};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use bytes::Bytes;
// Note: Query is still used for log_migrate which has different param type
use chrono::{Duration, Utc};
use common::error::{AppError, AppResult};
use serde_json::{json, Value};
use shaku_axum::Inject;
use std::str::FromStr;

use crate::model::import::ImportResult;
use crate::model::pota::{
    CreateRefRequest, PagenatedResponse, PotaLogHistView, PotaLogStatView, PotaRefLogView,
    PotaRefView, UpdateRefRequest,
};
use crate::model::{
    activation::ActivationView,
    alerts::AlertView,
    param::{build_findref_query, GetParam, ValidatedQuery},
    spots::SpotView,
};
use domain::model::{
    event::{DeleteRef, FindActBuilder, FindRefBuilder},
    id::LogId,
};
use domain::{model::pota::ParkCode, repository::pota::PotaRepository};
use registry::{AppRegistry, AppState};
use service::model::pota::{UploadPOTALog, UploadPOTAReference};
use service::services::{AdminService, PotaLogService, UserService};

use super::auth::{with_auth, AdminAuth};
use super::multipart::extract_text_file;

async fn create_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Json(req): Json<CreateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .create_pota_reference(req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn export_pota_reference_ja(
    admin_service: Inject<AppRegistry, dyn AdminService>,
) -> AppResult<Response> {
    let csv = admin_service.export_pota_parks_csv().await?;
    let filename = format!("jp_parks_{}.csv", Utc::now().format("%Y%m%d"));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        csv,
    )
        .into_response())
}

async fn update_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(park_code): Path<String>,
    Json(req): Json<UpdateRefRequest>,
) -> AppResult<StatusCode> {
    admin_service
        .update_pota_reference(park_code, req.into())
        .await
        .map(|_| StatusCode::CREATED)
}

async fn import_pota_reference_ja(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    mut multipart: Multipart,
) -> AppResult<Json<ImportResult>> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadPOTAReference { data };
    let count = admin_service.import_pota_park_list_ja(reqs).await?;
    Ok(Json(ImportResult::success(count as u32, 0)))
}

async fn upload_pota_log(
    pota_log_service: Inject<AppRegistry, dyn PotaLogService>,
    Path((activator_logid, hunter_logid)): Path<(String, String)>,
    mut multipart: Multipart,
) -> AppResult<Json<PotaLogHistView>> {
    let data = extract_text_file(&mut multipart).await?;
    let reqs = UploadPOTALog {
        activator_logid,
        hunter_logid,
        data,
    };
    let loguser = pota_log_service.upload_pota_log(reqs).await?;
    Ok(Json(loguser.into()))
}

async fn get_pota_logid(
    pota_log_service: Inject<AppRegistry, dyn PotaLogService>,
    Path(log_id): Path<String>,
) -> AppResult<Json<PotaLogHistView>> {
    let log_id = LogId::from_str(&log_id)?;
    let loguser = pota_log_service.find_logid(log_id).await?;
    Ok(Json(loguser.into()))
}

async fn delete_pota_log(
    pota_log_service: Inject<AppRegistry, dyn PotaLogService>,
    Path(log_id): Path<String>,
) -> AppResult<StatusCode> {
    let log_id = LogId::from_str(&log_id)?;

    pota_log_service
        .delete_pota_log(log_id)
        .await
        .map(|_| StatusCode::OK)
}

async fn delete_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(park_code): Path<String>,
) -> AppResult<StatusCode> {
    let req = DeleteRef::Delete(ParkCode::new(park_code));

    admin_service
        .delete_pota_reference(req)
        .await
        .map(|_| StatusCode::OK)
}

async fn show_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    Path(park_code): Path<String>,
) -> AppResult<Json<PotaRefView>> {
    let query = FindRefBuilder::default()
        .pota()
        .pota_code(park_code.clone())
        .build();

    match admin_service.show_pota_reference(query).await {
        Ok(result) => return Ok(Json(result.into())),
        Err(AppError::RowNotFound { .. }) => {}
        Err(e) => return Err(e),
    }

    // JAFF-only レコードは wwff_code で検索
    let query = FindRefBuilder::default()
        .pota()
        .wwff_code(park_code)
        .build();
    let result = admin_service.show_pota_reference(query).await?;
    Ok(Json(result.into()))
}

async fn show_all_pota_reference(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<PagenatedResponse<PotaRefView>>> {
    let limit = param.limit.unwrap_or(500);
    let offset = param.offset.unwrap_or(0);
    let query_param = GetParam {
        limit: Some(limit),
        offset: Some(offset),
        name: param.name,
        pota_code: param.pota_code,
        wwff_code: param.wwff_code,
        association: param.association,
        ..Default::default()
    };
    let query = build_findref_query(query_param, FindRefBuilder::default().pota())?;
    let result = admin_service.show_all_pota_references(query).await?;
    Ok(Json(result.into()))
}

async fn find_pota_reference(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Vec<PotaRefLogView>>> {
    let query = FindRefBuilder::default().pota();
    let mut query = build_findref_query(param, query)?;

    query.limit = query.limit.map_or(Some(500), |v| Some(v.min(500)));

    let results = user_service.find_references(query).await?;

    let res = results
        .pota
        .unwrap_or(vec![])
        .into_iter()
        .map(PotaRefLogView::from)
        .collect();

    Ok(Json(res))
}

async fn show_pota_spots(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Vec<ActivationView<SpotView>>>> {
    let hours = param.hours_ago.unwrap_or(3);

    let query = FindActBuilder::default()
        .pota()
        .issued_after(Utc::now() - Duration::hours(hours))
        .build();

    let result = user_service.find_spots(query).await?;

    let spots: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(SpotView::from).collect::<Vec<_>>()))
        })
        .collect();

    Ok(Json(spots))
}

async fn show_pota_alerts(
    user_service: Inject<AppRegistry, dyn UserService>,
    ValidatedQuery(param): ValidatedQuery<GetParam>,
) -> AppResult<Json<Vec<ActivationView<AlertView>>>> {
    let hours = param.hours_ago.unwrap_or(3);

    let query = FindActBuilder::default()
        .pota()
        .issued_after(Utc::now() - Duration::hours(hours))
        .build();

    let result = user_service.find_alerts(query).await?;

    let alerts: Vec<_> = result
        .into_iter()
        .map(|(k, v)| {
            ActivationView::from((k, v.into_iter().map(AlertView::from).collect::<Vec<_>>()))
        })
        .collect();

    Ok(Json(alerts))
}

/// ログ共有コードの文字(Crockford Base32。見間違えやすい I L O U は使わない)
const SHARE_CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// ログ共有コードの長さ(8 文字 = 40 bit。約 1 兆通り)
const SHARE_CODE_LEN: usize = 8;
/// ログ共有コードの有効期限(分)。1 回取り出したらその場で消す
const SHARE_CODE_TTL_MINUTES: i64 = 10;
/// コードが既存のものとぶつかったときにやり直す回数
const SHARE_CODE_RETRIES: usize = 5;
/// 共有するログ ID の長さの上限(UUID は 36 文字)
const MAX_SHARED_LOGID_LEN: usize = 64;

/// 新しいログ共有コードを作る。UUID v4 の乱数(暗号論的に安全な RNG)の先頭 40 bit を使う
fn new_share_code() -> String {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    let mut n = bytes[..5]
        .iter()
        .fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
    (0..SHARE_CODE_LEN)
        .map(|_| {
            let c = SHARE_CODE_ALPHABET[(n & 31) as usize] as char;
            n >>= 5;
            c
        })
        .collect()
}

/// 入力されたログ共有コードを正規化する。区切り(`-`)と空白を除いて大文字にし、
/// 紛らわしい文字を読み替える(O → 0、I・L → 1)。形が違えば None
fn normalize_share_code(input: &str) -> Option<String> {
    let code: String = input
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect();
    (code.len() == SHARE_CODE_LEN && code.bytes().all(|b| SHARE_CODE_ALPHABET.contains(&b)))
        .then_some(code)
}

/// 表示用に 4 文字ずつ区切る(`K7QM-4XRT`)
fn display_share_code(code: &str) -> String {
    format!("{}-{}", &code[..4], &code[4..])
}

async fn reqeust_shareid(
    State(state): State<AppState>,
    Path((act_id, hntr_id)): Path<(String, String)>,
) -> AppResult<Json<Value>> {
    if act_id.len() > MAX_SHARED_LOGID_LEN || hntr_id.len() > MAX_SHARED_LOGID_LEN {
        return Err(AppError::UnprocessableEntity(
            "ログIDが長すぎます".to_string(),
        ));
    }

    for _ in 0..SHARE_CODE_RETRIES {
        let code = new_share_code();
        let value = json!({
            "share_id": display_share_code(&code),
            "activator_logid": act_id,
            "hunter_logid": hntr_id
        });
        let body = serde_json::to_vec(&value)
            .map(Bytes::from)
            .map_err(|e| AppError::ConversionEntityError(e.to_string()))?;
        if state
            .share_kvs
            .insert_new(code, body, Duration::minutes(SHARE_CODE_TTL_MINUTES))
            .await
        {
            return Ok(Json(value));
        }
    }

    Err(AppError::ConversionEntityError(
        "共有コードを発行できませんでした".to_string(),
    ))
}

async fn obtain_shareid(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> AppResult<Json<Value>> {
    let not_found = || AppError::EntityNotFound("invalid share_id".to_string());
    let code = normalize_share_code(&share_id).ok_or_else(not_found)?;
    let body = state.share_kvs.take(&code).await.ok_or_else(not_found)?;
    serde_json::from_slice(&body)
        .map(Json)
        .map_err(|e| AppError::ConversionEntityError(e.to_string()))
}

async fn log_stat(
    pota_repo: Inject<AppRegistry, dyn PotaRepository>,
) -> AppResult<Json<PotaLogStatView>> {
    let stat = pota_repo.log_statistics().await?;
    Ok(Json(stat.into()))
}

async fn log_migrate(
    pota_repo: Inject<AppRegistry, dyn PotaRepository>,
    Query(param): Query<GetParam>,
) -> AppResult<StatusCode> {
    if let Some(dbname) = param.name {
        pota_repo
            .migrate_legacy_log(dbname)
            .await
            .map(|_| StatusCode::OK)?;
    }
    Ok(StatusCode::NOT_FOUND)
}

pub fn build_pota_routers(auth: &AdminAuth) -> Router<AppState> {
    let protected = with_auth(
        Router::new()
            .route("/import", post(import_pota_reference_ja))
            .route("/parks", post(create_pota_reference))
            .route("/parks/export", get(export_pota_reference_ja))
            .route("/parks/{park_code}", put(update_pota_reference))
            .route("/parks/{park_code}", delete(delete_pota_reference))
            .route("/log-migrate", get(log_migrate)),
        auth,
    );

    let public = Router::new()
        .route("/log/{act_id}/{hntr_id}", post(upload_pota_log))
        .route("/log/{log_id}", get(get_pota_logid))
        .route("/log/{log_id}", delete(delete_pota_log))
        .route("/log-share/{act_id}/{hntr_id}", get(reqeust_shareid))
        .route("/log-share/{share_id}", get(obtain_shareid))
        .route("/log-stat", get(log_stat))
        .route("/parks", get(show_all_pota_reference))
        .route("/parks/{park_code}", get(show_pota_reference))
        // 数十秒ごとに変わるスポット・アラートと、ほとんど変わらない検索は、共有キャッシュに覚えさせる
        .merge(cached(
            Router::new()
                .route("/spots", get(show_pota_spots))
                .route("/alerts", get(show_pota_alerts)),
            LIVE_SECS,
        ))
        .merge(cached(
            Router::new().route("/parks/search", get(find_pota_reference)),
            SEARCH_SECS,
        ));

    let routers = Router::new().merge(protected).merge(public);

    Router::new().nest("/pota", routers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_code_has_expected_shape() {
        for _ in 0..1000 {
            let code = new_share_code();
            assert_eq!(code.len(), SHARE_CODE_LEN);
            assert!(code.bytes().all(|b| SHARE_CODE_ALPHABET.contains(&b)));
            assert_eq!(normalize_share_code(&code), Some(code));
        }
    }

    // 1 万個作って重複しない(40 bit なので、ぶつかる確率は 1 万分の 1 未満)
    #[test]
    fn share_codes_are_not_repeated() {
        let codes: std::collections::HashSet<_> = (0..10_000).map(|_| new_share_code()).collect();
        assert_eq!(codes.len(), 10_000);
    }

    #[test]
    fn share_code_is_normalized_from_user_input() {
        assert_eq!(normalize_share_code("K7QM-4XRT"), Some("K7QM4XRT".into()));
        assert_eq!(normalize_share_code(" k7qm 4xrt "), Some("K7QM4XRT".into()));
        // 紛らわしい文字を読み替える
        assert_eq!(normalize_share_code("O1IL-oil0"), Some("01110110".into()));
        assert_eq!(display_share_code("K7QM4XRT"), "K7QM-4XRT");
    }

    #[test]
    fn malformed_share_code_is_rejected() {
        assert_eq!(normalize_share_code("1234"), None);
        assert_eq!(normalize_share_code("K7QM-4XRTX"), None);
        assert_eq!(normalize_share_code("K7QM-4XRU"), None);
        assert_eq!(normalize_share_code("K7QM-4XR!"), None);
        assert_eq!(normalize_share_code(""), None);
    }
}
