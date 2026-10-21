//! ログ変換ハンドラー
//!
//! HAMLOG CSV / ADIF形式からSOTA/POTA/WWFF形式への変換エンドポイント
//!
//! - `POST /logconv/hamlog`: ログを変換して、ZIP を返す(`format` で形式を選ぶ)
//! - `POST /logconv/pota`: POTA の ADIF にしたときの確認(誤りの一覧と、画面に出す表)を JSON で返す

mod input;
mod options;
mod reply;
mod states;

use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use registry::{AppRegistry, AppState};
use serde::Deserialize;
use service::implement::logconv::{
    check_adif, convert_to_adif, convert_to_sota_activator, convert_to_sota_chaser, error_log,
    normalize_callsign,
};
use service::services::AdminService;
use shaku_axum::Inject;

use input::{Loaded, Upload};
use options::{ConversionOptionsRequest, OutputFormat};
use reply::Failure;

/// アップロードの上限(MB)。v2 の全体の上限(`multipart::USER_UPLOAD_MB`)と同じ大きさを、logconv でも明示しておく。
///
/// 変換は、読み取った交信と出力を全部メモリに持つので、入力の十数倍のメモリを使う(0.8MB・1万行で 20MB 前後)。
/// 本番のマシン(256MB。アプリの常駐は 80MB ほど)が落ちないよう、実際のログに足りる大きさ(1万 QSO に備考を書いても収まる)に抑える
pub(crate) const MAX_UPLOAD_MB: usize = 2;

/// アップロードの上限。v2 の全体の上限(`v2.rs`)より内側でかける
pub(crate) fn upload_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(MAX_UPLOAD_MB * 1024 * 1024)
}

/// Logconvルーターを作成
pub fn logconv_router() -> Router<AppState> {
    Router::new()
        .route("/hamlog", post(hamlog_handler))
        .route("/pota", post(pota_handler))
        .layer(upload_limit())
}

/// `/hamlog` のオプション(`options` の JSON)
#[derive(Debug, Deserialize)]
struct LogconvRequest {
    /// 出力形式
    format: OutputFormat,
    /// コールサイン(チェイサーの自局など)
    callsign: Option<String>,
    #[serde(flatten)]
    options: ConversionOptionsRequest,
}

/// HAMLOGフォーマット変換ハンドラー
async fn hamlog_handler(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    multipart: Multipart,
) -> Response {
    hamlog_convert(Some(&*admin_service), multipart).await
}

/// ログを変換して ZIP を返す。`admin` は公園の所在地(MY_STATE)を引くのに使う(なければ入れない)
async fn hamlog_convert(admin: Option<&dyn AdminService>, multipart: Multipart) -> Response {
    convert(admin, multipart)
        .await
        .unwrap_or_else(IntoResponse::into_response)
}

async fn convert(
    admin: Option<&dyn AdminService>,
    multipart: Multipart,
) -> Result<Response, Failure> {
    let upload = Upload::read(multipart).await?;
    let request: LogconvRequest = upload.options_strict()?;
    // コールサインは、前後の空白を除いて大文字にそろえる。必要なものが空なら断る
    let sota_callsign = [
        request.callsign.as_deref(),
        request.options.sota_activator.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(normalize_callsign)
    .find(|c| !c.is_empty())
    .unwrap_or_default();
    let pota_activator = normalize_callsign(
        request
            .options
            .pota_activator
            .as_deref()
            .unwrap_or_default(),
    );
    match request.format {
        OutputFormat::SotaA if sota_callsign.is_empty() => {
            return Err(Failure::bad_request(
                "運用局のコールサインを入力してください",
            ))
        }
        OutputFormat::SotaC if sota_callsign.is_empty() => {
            return Err(Failure::bad_request("自局のコールサインを入力してください"))
        }
        OutputFormat::Pota if pota_activator.is_empty() => {
            return Err(Failure::bad_request(
                "運用局のコールサインを入力してください",
            ))
        }
        _ => {}
    }
    let Loaded { records, options } = upload.load(request.options, admin).await?;

    Ok(match request.format {
        OutputFormat::SotaA => sota_reply(
            &records,
            convert_to_sota_activator(&records, &sota_callsign, &options),
            "sota.zip",
        ),
        OutputFormat::SotaC => sota_reply(
            &records,
            convert_to_sota_chaser(&records, &sota_callsign, &options),
            "sota-chaser.zip",
        ),
        OutputFormat::Pota => {
            let result = convert_to_adif(&records, &options);
            // 読み取りに失敗した行があるログは、ADIF を生成しない(その交信が黙って抜けるのを防ぐ)。
            // 画面は、確認(/logconv/pota)でエラーを見せ、直してから生成する
            if result.status == "NG" {
                return Err(
                    Failure::bad_request("The log has errors").with_error_log(result.error_log)
                );
            }
            reply::zip_or_empty(result.files, "pota.zip", "No POTA references found")
        }
    })
}

/// SOTA の応答。読める交信が1つもなければ 400 と誤りの一覧、あれば ZIP
fn sota_reply(
    records: &[service::implement::logconv::QsoRecord],
    files: std::collections::HashMap<String, String>,
    zip_name: &str,
) -> Response {
    if records.iter().all(|r| r.has_error()) {
        return Failure::bad_request("The log has errors")
            .with_error_log(error_log(records))
            .into_response();
    }
    reply::zip_or_empty(files, zip_name, "No SOTA references found")
}

/// POTA ADIF変換ハンドラー (JSON応答版)
async fn pota_handler(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    multipart: Multipart,
) -> Response {
    pota_check(Some(&*admin_service), multipart).await
}

/// POTA の ADIF にしたときの確認。誤りの一覧(`error_log`)と、画面に出す表(`log_text`)を返す
async fn pota_check(admin: Option<&dyn AdminService>, multipart: Multipart) -> Response {
    check(admin, multipart)
        .await
        .unwrap_or_else(IntoResponse::into_response)
}

async fn check(
    admin: Option<&dyn AdminService>,
    multipart: Multipart,
) -> Result<Response, Failure> {
    let upload = Upload::read(multipart).await?;
    // 確認は、オプションが読めなくても、既定の指定で進める
    let request: ConversionOptionsRequest = upload.options_or_default();
    let Loaded { records, options } = upload.load(request, admin).await?;

    let result = check_adif(&records, &options);
    Ok((StatusCode::OK, Json(result)).into_response())
}

#[cfg(test)]
mod tests;
