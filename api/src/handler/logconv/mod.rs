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
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use registry::{AppRegistry, AppState};
use serde::Deserialize;
use service::implement::logconv::{
    convert_to_adif, convert_to_sota_activator, convert_to_sota_chaser,
};
use service::services::AdminService;
use shaku_axum::Inject;

use input::{Loaded, Upload};
use options::{ConversionOptionsRequest, OutputFormat};
use reply::Failure;

/// Logconvルーターを作成
pub fn logconv_router() -> Router<AppState> {
    Router::new()
        .route("/hamlog", post(hamlog_handler))
        .route("/pota", post(pota_handler))
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
    let Loaded { records, options } = upload.load(request.options, admin).await?;
    let callsign = request
        .callsign
        .unwrap_or_else(|| options.sota_activator.clone());

    Ok(match request.format {
        OutputFormat::SotaA => reply::zip_or_empty(
            convert_to_sota_activator(&records, &callsign, &options),
            "sota.zip",
            "No SOTA references found",
        ),
        OutputFormat::SotaC => reply::zip_or_empty(
            convert_to_sota_chaser(&records, &callsign, &options),
            "sota-chaser.zip",
            "No SOTA references found",
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

    let result = convert_to_adif(&records, &options);
    Ok((StatusCode::OK, Json(result)).into_response())
}

#[cfg(test)]
mod tests;
