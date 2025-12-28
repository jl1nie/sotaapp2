//! ログ変換ハンドラー
//!
//! HAMLOG CSV / ADIF形式からSOTA/POTA/WWFF形式への変換エンドポイント

use axum::{
    extract::Multipart,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use registry::AppState;
use serde::{Deserialize, Serialize};
use service::implement::logconv::{
    convert_to_adif, convert_to_sota_activator, create_zip, decode_adif, decode_auto, parse_adif,
    ConversionOptions, ConversionResult, QsoRecord,
};

/// Logconvルーターを作成
pub fn logconv_router() -> Router<AppState> {
    Router::new()
        .route("/hamlog", post(hamlog_handler))
        .route("/pota", post(pota_handler))
}

/// リクエストパラメータ
#[derive(Debug, Deserialize)]
struct LogconvRequest {
    /// 出力形式 (sota_a, sota_c, pota, wwff, airham)
    format: String,
    /// コールサイン
    callsign: Option<String>,
    /// オプション
    #[serde(flatten)]
    options: ConversionOptionsRequest,
}

#[derive(Debug, Deserialize, Default)]
struct ConversionOptionsRequest {
    /// 自局QTHソース (rmks1, rmks2, user_defined)
    #[serde(rename = "myQTH")]
    my_qth: Option<String>,
    /// 相手局QTHソース (rmks1, rmks2, qth, none)
    #[serde(rename = "QTH")]
    his_qth: Option<String>,
    /// ユーザー定義ロケーション
    #[serde(rename = "Location")]
    location: Option<String>,
    /// SOTAサミットリファレンス
    #[serde(rename = "Summit")]
    summit: Option<String>,
    /// POTAパークリファレンス
    #[serde(rename = "Park")]
    park: Option<String>,
    /// SOTAアクティベーターコールサイン
    #[serde(rename = "SOTAActivator")]
    sota_activator: Option<String>,
    /// POTAアクティベーターコールサイン
    #[serde(rename = "POTAActivator")]
    pota_activator: Option<String>,
    /// POTAオペレーターコールサイン
    #[serde(rename = "POTAOperator")]
    pota_operator: Option<String>,
}

impl From<ConversionOptionsRequest> for ConversionOptions {
    fn from(req: ConversionOptionsRequest) -> Self {
        ConversionOptions {
            my_qth: req.my_qth.unwrap_or_else(|| "rmks1".to_string()),
            his_qth: req.his_qth.unwrap_or_else(|| "qth".to_string()),
            location: req.location.unwrap_or_default(),
            summit: req.summit.unwrap_or_default(),
            park: req.park.map(|p| vec![p]).unwrap_or_default(),
            sota_activator: req.sota_activator.unwrap_or_default(),
            pota_activator: req.pota_activator.unwrap_or_default(),
            pota_operator: req.pota_operator.unwrap_or_default(),
            ..Default::default()
        }
    }
}

/// レスポンス
#[derive(Debug, Serialize)]
struct LogconvResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    error_log: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    log_text: Vec<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    file_list: Vec<String>,
}

/// HAMLOGフォーマット変換ハンドラー
async fn hamlog_handler(mut multipart: Multipart) -> Response {
    let mut file_content: Option<Vec<u8>> = None;
    let mut options_json: Option<String> = None;

    // マルチパートフォームを処理
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Ok(bytes) = field.bytes().await {
                    file_content = Some(bytes.to_vec());
                }
            }
            Some("options") | Some("arg") => {
                if let Ok(text) = field.text().await {
                    options_json = Some(text);
                }
            }
            _ => {}
        }
    }

    let content = match file_content {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(LogconvResponse {
                    status: "NG".to_string(),
                    error: Some("Missing 'file' parameter".to_string()),
                    error_log: vec![],
                    log_text: vec![],
                    file_list: vec![],
                }),
            )
                .into_response();
        }
    };

    // オプションをパース
    let request: LogconvRequest = match options_json {
        Some(json) => match serde_json::from_str(&json) {
            Ok(req) => req,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(LogconvResponse {
                        status: "NG".to_string(),
                        error: Some(format!("Invalid options JSON: {}", e)),
                        error_log: vec![],
                        log_text: vec![],
                        file_list: vec![],
                    }),
                )
                    .into_response();
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(LogconvResponse {
                    status: "NG".to_string(),
                    error: Some("Missing 'options' or 'arg' parameter".to_string()),
                    error_log: vec![],
                    log_text: vec![],
                    file_list: vec![],
                }),
            )
                .into_response();
        }
    };

    // ファイル内容をUTF-8で読み込み (Shift-JISフォールバックなし、簡略化)
    let content_str = String::from_utf8_lossy(&content).to_string();

    // CSVをパース
    let mut records: Vec<QsoRecord> = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content_str.as_bytes());

    let mut header_seen = false;
    for record in reader.records().flatten() {
        let cols: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        if cols.is_empty() {
            continue;
        }
        match decode_auto(&cols, header_seen) {
            Ok(qso) => {
                records.push(qso);
                header_seen = true;
            }
            Err(e) if e == "HEADER" => {
                header_seen = true;
            }
            Err(_) => {
                header_seen = true;
            }
        }
    }

    if records.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(LogconvResponse {
                status: "NG".to_string(),
                error: Some("No valid records found".to_string()),
                error_log: vec![],
                log_text: vec![],
                file_list: vec![],
            }),
        )
            .into_response();
    }

    let options: ConversionOptions = request.options.into();
    let callsign = request
        .callsign
        .unwrap_or_else(|| options.sota_activator.clone());

    // 形式に応じて変換
    match request.format.as_str() {
        "sota_a" => {
            let files = convert_to_sota_activator(&records, &callsign, &options);
            if files.is_empty() {
                return (
                    StatusCode::OK,
                    Json(LogconvResponse {
                        status: "OK".to_string(),
                        error: Some("No SOTA references found".to_string()),
                        error_log: vec![],
                        log_text: vec![],
                        file_list: vec![],
                    }),
                )
                    .into_response();
            }

            match create_zip(&files) {
                Ok(zip_data) => (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "application/zip"),
                        (
                            header::CONTENT_DISPOSITION,
                            "attachment; filename=\"sota.zip\"",
                        ),
                    ],
                    zip_data,
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LogconvResponse {
                        status: "NG".to_string(),
                        error: Some(format!("ZIP creation failed: {}", e)),
                        error_log: vec![],
                        log_text: vec![],
                        file_list: vec![],
                    }),
                )
                    .into_response(),
            }
        }
        "pota" => {
            let result = convert_to_adif(&records, &options);

            if result.files.is_empty() {
                return (
                    StatusCode::OK,
                    Json(LogconvResponse {
                        status: "OK".to_string(),
                        error: Some("No POTA references found".to_string()),
                        error_log: vec![],
                        log_text: vec![],
                        file_list: vec![],
                    }),
                )
                    .into_response();
            }

            match create_zip(&result.files) {
                Ok(zip_data) => (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "application/zip"),
                        (
                            header::CONTENT_DISPOSITION,
                            "attachment; filename=\"pota.zip\"",
                        ),
                    ],
                    zip_data,
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(LogconvResponse {
                        status: "NG".to_string(),
                        error: Some(format!("ZIP creation failed: {}", e)),
                        error_log: vec![],
                        log_text: vec![],
                        file_list: vec![],
                    }),
                )
                    .into_response(),
            }
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(LogconvResponse {
                status: "NG".to_string(),
                error: Some(format!("Unsupported format: {}", request.format)),
                error_log: vec![],
                log_text: vec![],
                file_list: vec![],
            }),
        )
            .into_response(),
    }
}

/// POTA ADIF変換ハンドラー (JSON応答版)
async fn pota_handler(mut multipart: Multipart) -> Response {
    let mut file_content: Option<Vec<u8>> = None;
    let mut options_json: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Ok(bytes) = field.bytes().await {
                    file_content = Some(bytes.to_vec());
                }
            }
            Some("options") | Some("arg") => {
                if let Ok(text) = field.text().await {
                    options_json = Some(text);
                }
            }
            _ => {}
        }
    }

    let content = match file_content {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(LogconvResponse {
                    status: "NG".to_string(),
                    error: Some("Missing 'file' parameter".to_string()),
                    error_log: vec![],
                    log_text: vec![],
                    file_list: vec![],
                }),
            )
                .into_response();
        }
    };

    let request: ConversionOptionsRequest = match options_json {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => ConversionOptionsRequest::default(),
    };

    let content_str = String::from_utf8_lossy(&content).to_string();

    // ADIFかCSVか判定
    let is_adif =
        content_str.to_uppercase().contains("<EOH>") || content_str.to_uppercase().contains("ADIF");

    let mut records: Vec<QsoRecord> = Vec::new();
    let mut options: ConversionOptions = request.into();
    options.his_qth = "qth".to_string(); // ADIFの場合はqthから取得

    if is_adif {
        // ADIF形式
        let adif_records = parse_adif(&content_str);
        for fields in adif_records {
            // ADIFレコードを文字列に再構築
            let record_str: String = fields
                .iter()
                .map(|(k, v)| format!("<{}:{}>{}", k, v.len(), v))
                .collect::<Vec<_>>()
                .join("")
                + "<EOR>";

            match decode_adif(&record_str) {
                Ok(qso) => records.push(qso),
                Err(_) => continue,
            }
        }
    } else {
        // CSV形式
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content_str.as_bytes());

        let mut header_seen = false;
        for record in reader.records().flatten() {
            let cols: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            if cols.is_empty() {
                continue;
            }
            match decode_auto(&cols, header_seen) {
                Ok(qso) => {
                    records.push(qso);
                    header_seen = true;
                }
                Err(e) if e == "HEADER" => {
                    header_seen = true;
                }
                Err(_) => {
                    header_seen = true;
                }
            }
        }
    }

    if records.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(LogconvResponse {
                status: "NG".to_string(),
                error: Some("No valid records found".to_string()),
                error_log: vec![],
                log_text: vec![],
                file_list: vec![],
            }),
        )
            .into_response();
    }

    let result = convert_to_adif(&records, &options);

    (
        StatusCode::OK,
        Json(ConversionResult {
            status: result.status,
            error_log: result.error_log,
            log_text: result.log_text,
            file_list: result.file_list,
            files: result.files,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logconv_router_exists() {
        let _router = logconv_router();
    }
}
