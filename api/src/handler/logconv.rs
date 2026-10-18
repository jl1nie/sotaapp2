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
use common::error::AppError;
use domain::model::event::FindRefBuilder;
use registry::{AppRegistry, AppState};
use serde::{Deserialize, Serialize};
use service::implement::logconv::{
    convert_to_adif, convert_to_sota_activator, convert_to_sota_chaser, create_zip, decode_adif,
    decode_auto, get_ref, parse_adif, ConversionOptions, ConversionResult, QsoRecord,
};
use service::services::AdminService;
use shaku_axum::Inject;
use std::collections::HashMap;

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
async fn hamlog_handler(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    multipart: Multipart,
) -> Response {
    hamlog_convert(Some(&*admin_service), multipart).await
}

/// `admin` は公園の所在地(MY_STATE)を引くのに使う。なければ(テストなど)入れない
async fn hamlog_convert(admin: Option<&dyn AdminService>, mut multipart: Multipart) -> Response {
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

    let content_str = decode_text(&content);
    let (records, is_adif) = read_records(&content_str);

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

    let mut options: ConversionOptions = request.options.into();
    if is_adif {
        // ADIF の入力には、HAMLOG の QTH の列がない。相手の参照は、ADIF のコメントなどから取る
        options.his_qth = "qth".to_string();
    }
    if let Some(admin) = admin {
        options.park_states = park_states(admin, &records, &options).await;
    }
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
        "sota_c" => {
            let files = convert_to_sota_chaser(&records, &callsign, &options);
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
                            "attachment; filename=\"sota-chaser.zip\"",
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

            // 読み取りに失敗した行があるログは、ADIF を生成しない(その交信が黙って抜けるのを防ぐ)。
            // 画面は、確認(/logconv/pota)でエラーを見せ、直してから生成する
            if result.status == "NG" {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(LogconvResponse {
                        status: "NG".to_string(),
                        error: Some("The log has errors".to_string()),
                        error_log: result.error_log,
                        log_text: vec![],
                        file_list: vec![],
                    }),
                )
                    .into_response();
            }

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

/// 入力ファイルの文字コードを判定して読む。UTF-8 として正しければ UTF-8、そうでなければ Shift-JIS(cp932)
///
/// 日本語版の HAMLOG が出す CSV は Shift-JIS(旧版は常に cp932 で読んでいた)。
fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// 入力のファイルを読む。1行目に `ADIF` か `<EOH>` があれば ADIF、なければ HAMLOG CSV(HamLog iOS を含む)
fn read_records(content: &str) -> (Vec<QsoRecord>, bool) {
    let first_line = content
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .to_uppercase();
    let is_adif = first_line.contains("ADIF") || first_line.contains("<EOH>");

    let mut records: Vec<QsoRecord> = Vec::new();
    if is_adif {
        for fields in parse_adif(content) {
            // ADIFレコードを文字列に再構築
            let record_str: String = fields
                .iter()
                .map(|(k, v)| format!("<{}:{}>{}", k, v.len(), v))
                .collect::<Vec<_>>()
                .join("")
                + "<EOR>";
            match decode_adif(&record_str) {
                Ok(qso) => records.push(qso),
                Err(e) => records.push(QsoRecord {
                    has_error: true,
                    error_message: format!("エラー:{e}"),
                    ..Default::default()
                }),
            }
        }
    } else {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(content.as_bytes());

        let mut header_seen = false;
        for record in reader.records().flatten() {
            let cols: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            if cols.is_empty() {
                continue;
            }
            let line = record.position().map_or(0, |p| p.line() as usize);
            match decode_auto(&cols, header_seen) {
                Ok(mut qso) => {
                    qso.source_line = line;
                    records.push(qso);
                }
                // 見出しの行は、読み飛ばす
                Err(e) if e == "HEADER" => {}
                // 列が足りないなど、読み取れない行は、誤りの行として残す(黙って捨てない)
                Err(e) => records.push(QsoRecord {
                    source_line: line,
                    has_error: true,
                    error_message: format!("エラー:{e}"),
                    ..Default::default()
                }),
            }
            header_seen = true;
        }
    }
    (records, is_adif)
}

/// 運用した公園(POTA)の所在地を、サーバーの公園データから引く。例: `JA-0001` → `TK`
///
/// 旧版と同じく、所在地が1つに決まる公園だけ(複数の都道府県にまたがる公園は、入れない)。
async fn park_states(
    admin: &dyn AdminService,
    records: &[QsoRecord],
    options: &ConversionOptions,
) -> HashMap<String, String> {
    let mut codes: Vec<String> = Vec::new();
    for qso in records {
        let mine = match options.my_qth.as_str() {
            "rmks1" => get_ref(&qso.remarks1).pota,
            "rmks2" => get_ref(&qso.remarks2).pota,
            _ => get_ref(&options.park.join(" ")).pota,
        };
        for code in mine {
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
    }

    let mut states = HashMap::new();
    for code in codes {
        let query = FindRefBuilder::default()
            .pota()
            .pota_code(code.clone())
            .build();
        let park = match admin.show_pota_reference(query).await {
            Ok(park) => park,
            Err(AppError::RowNotFound { .. }) => continue,
            Err(e) => {
                tracing::warn!("park location lookup failed for {code}: {e}");
                continue;
            }
        };
        if let Some(state) = single_state(&park.park_locid) {
            states.insert(code, state);
        }
    }
    states
}

/// `JP-TK` → `TK`。カンマで区切った複数の所在地(`JP-TK,JP-SI`)は None
fn single_state(locid: &str) -> Option<String> {
    let mut parts = locid.split(',').map(str::trim).filter(|p| !p.is_empty());
    let first = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    Some(first.trim_start_matches("JP-").to_string())
}

/// POTA ADIF変換ハンドラー (JSON応答版)
async fn pota_handler(
    admin_service: Inject<AppRegistry, dyn AdminService>,
    multipart: Multipart,
) -> Response {
    pota_check(Some(&*admin_service), multipart).await
}

async fn pota_check(admin: Option<&dyn AdminService>, mut multipart: Multipart) -> Response {
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

    let content_str = decode_text(&content);

    let (records, is_adif) = read_records(&content_str);
    let mut options: ConversionOptions = request.into();
    if is_adif {
        options.his_qth = "qth".to_string(); // ADIFの場合はqthから取得
    }
    if let Some(admin) = admin {
        options.park_states = park_states(admin, &records, &options).await;
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

    /// multipart/form-data のリクエストを作る
    async fn multipart_of(file: &str, options: &str) -> Multipart {
        multipart_of_bytes(file.as_bytes(), options).await
    }

    async fn multipart_of_bytes(file: &[u8], options: &str) -> Multipart {
        use axum::extract::FromRequest;
        let boundary = "XBOUNDARYX";
        let mut body: Vec<u8> = format!(
            "--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"log.csv\"\r\n\r\n",
            b = boundary
        )
        .into_bytes();
        body.extend_from_slice(file);
        body.extend_from_slice(
            format!(
                "\r\n--{b}\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{options}\r\n--{b}--\r\n",
                b = boundary
            )
            .as_bytes(),
        );
        let request = axum::http::Request::builder()
            .method("POST")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(axum::body::Body::from(body))
            .unwrap();
        Multipart::from_request(request, &()).await.unwrap()
    }

    async fn post_hamlog(csv: &str, options: &str) -> Response {
        hamlog_convert(None, multipart_of(csv, options).await).await
    }

    async fn post_pota_check(csv: &str, options: &str) -> serde_json::Value {
        let res = pota_check(None, multipart_of(csv, options).await).await;
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // HAMLOG CSV: コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,
    const CSV: &str = "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,JA/KN-001,,,\n\
                       JA1BBB,25/10/18,01:05U,59,59,7.030,CW,,,N,,Tokyo,,,\n";

    #[tokio::test]
    async fn test_hamlog_sota_chaser_returns_a_zip() {
        let res = post_hamlog(
            CSV,
            r#"{"format":"sota_c","QTH":"qth","callsign":"JA1ZZZ"}"#,
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CONTENT_TYPE], "application/zip");
    }

    #[tokio::test]
    async fn test_hamlog_unknown_format_is_a_bad_request() {
        let res = post_hamlog(CSV, r#"{"format":"nope"}"#).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // 相手(JA1BBB)の SOTA 参照は QTH の列にある。「指定しない」なら、読まない
    #[tokio::test]
    async fn test_pota_check_respects_his_qth_none_for_csv() {
        let opts = |qth: &str| {
            format!(
                r#"{{"command":"ADIFCSVCheck","myQTH":"rmks1","QTH":"{qth}","POTAActivator":"JA1ZZZ/1"}}"#
            )
        };
        let csv = "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,JA/KN-001,JA-0001,,\n";
        let none = post_pota_check(csv, &opts("none")).await;
        assert_eq!(none["log_text"][0][7], "");
        let qth = post_pota_check(csv, &opts("qth")).await;
        assert_eq!(qth["log_text"][0][7], "JA/KN-001");
    }

    #[test]
    fn test_decode_text_reads_shift_jis_and_utf8() {
        // HAMLOG(Shift-JIS)の行: 名前と QTH に日本語、備考1に参照と日本語
        let (sjis, _, _) = encoding_rs::SHIFT_JIS
            .encode("JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,太郎,東京,JA/TK-001 高尾山,,\n");
        let text = decode_text(&sjis);
        assert!(text.contains("太郎") && text.contains("JA/TK-001 高尾山"));
        // UTF-8 は、そのまま(BOM は外す)
        assert_eq!(decode_text("高尾山".as_bytes()), "高尾山");
        assert_eq!(decode_text(b"\xEF\xBB\xBFabc"), "abc");
    }

    #[test]
    fn test_single_state_from_a_location_id() {
        assert_eq!(single_state("JP-TK").as_deref(), Some("TK"));
        assert_eq!(single_state("JP-TK,JP-SI"), None);
        assert_eq!(single_state(""), None);
    }

    // 読み取りに失敗した行(存在しない時刻)があるログは、ADIF を生成しない。理由は error_log で返す
    #[tokio::test]
    async fn test_hamlog_pota_refuses_a_log_with_errors() {
        let csv = "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,\n\
                   JA1BBB,25/10/18,99:99U,59,59,7.030,CW,,,N,,,JA-0001,,\n";
        let opts = r#"{"format":"pota","myQTH":"rmks1","POTAActivator":"JA1ZZZ/1","POTAOperator":"JA1ZZZ"}"#;
        let res = post_hamlog(csv, opts).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["status"], "NG");
        assert!(body["error_log"][0].as_str().unwrap().starts_with("2行:"));
        // エラーのない行だけのログは、生成できる
        let ok = post_hamlog(csv.lines().next().unwrap(), opts).await;
        assert_eq!(ok.status(), StatusCode::OK);
    }

    // ADIF ファイルを入力にして、POTA の ADIF を作れる
    #[tokio::test]
    async fn test_hamlog_pota_accepts_an_adif_file() {
        let adif = "ADIF Export\n<EOH>\n\
            <CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:4>0100<BAND:3>40m<MODE:2>CW\
            <MY_SIG_INFO:7>JA-0001<SIG_INFO:7>JA-0123<EOR>\n";
        let res = post_hamlog(
            adif,
            r#"{"format":"pota","myQTH":"rmks1","POTAActivator":"JA1ZZZ/1","POTAOperator":"JA1ZZZ"}"#,
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CONTENT_TYPE], "application/zip");
    }

    // ---- パターンを網羅したフィクスチャ(test_data/logconv/patterns)と、旧版の出力(expected/)との一致 ----

    fn patterns_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_data/logconv/patterns")
    }

    fn unzip(bytes: &[u8]) -> std::collections::BTreeMap<String, String> {
        use std::io::Read;
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut files = std::collections::BTreeMap::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).unwrap();
            let mut body = String::new();
            f.read_to_string(&mut body).unwrap();
            files.insert(f.name().to_string(), body.replace("\r\n", "\n"));
        }
        files
    }

    fn read_dir_files(dir: &std::path::Path) -> std::collections::BTreeMap<String, String> {
        let mut files = std::collections::BTreeMap::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if name == "check.json" {
                    continue;
                }
                files.insert(name, std::fs::read_to_string(e.path()).unwrap());
            }
        }
        files
    }

    #[tokio::test]
    async fn test_patterns_match_the_legacy_output() {
        let dir = patterns_dir();
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap())
                .unwrap();
        let mut diffs: Vec<String> = Vec::new();

        for case in &cases {
            if case.get("expect").is_some() {
                continue; // 誤りのある行などは、別のテスト(test_patterns_with_expectations)
            }
            let name = case["name"].as_str().unwrap();
            let format = case["format"].as_str().unwrap();
            let file = std::fs::read(dir.join(case["file"].as_str().unwrap())).unwrap();
            let mut options = case["options"].clone();
            options["format"] = format.into();
            let options = options.to_string();

            let res = hamlog_convert(None, multipart_of_bytes(&file, &options).await).await;
            if res.status() != StatusCode::OK {
                diffs.push(format!("{name}: HTTP {}", res.status()));
                continue;
            }
            let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                .await
                .unwrap();
            let got = unzip(&bytes);
            let want = read_dir_files(&dir.join("expected").join(name));

            let got_names: Vec<&String> = got.keys().collect();
            let want_names: Vec<&String> = want.keys().collect();
            if got_names != want_names {
                diffs.push(format!(
                    "{name}: ファイルの組が違う\n  新: {got_names:?}\n  旧: {want_names:?}"
                ));
                continue;
            }
            // ADIF は、確認(CSV読込)の結果も、旧版と同じ
            if format == "pota" {
                let mut check_options: serde_json::Value = serde_json::from_str(&options).unwrap();
                check_options["command"] = "ADIFCSVCheck".into();
                let res = pota_check(
                    None,
                    multipart_of_bytes(&file, &check_options.to_string()).await,
                )
                .await;
                let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                    .await
                    .unwrap();
                let got_check: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                let want_check: serde_json::Value = serde_json::from_str(
                    &std::fs::read_to_string(dir.join("expected").join(name).join("check.json"))
                        .unwrap(),
                )
                .unwrap();
                if got_check["status"] != want_check["status"] {
                    diffs.push(format!(
                        "{name}: 確認の status {}(旧 {})",
                        got_check["status"], want_check["status"]
                    ));
                }
                let (g, w) = (&got_check["log_text"], &want_check["logtext"]);
                if g != w {
                    let (ga, wa) = (
                        g.as_array().cloned().unwrap_or_default(),
                        w.as_array().cloned().unwrap_or_default(),
                    );
                    let i = ga
                        .iter()
                        .zip(wa.iter())
                        .position(|(a, b)| a != b)
                        .unwrap_or(ga.len().min(wa.len()));
                    diffs.push(format!(
                        "{name}: 確認の表の {}行目から違う\n  新: {}\n  旧: {}",
                        i + 1,
                        ga.get(i).map_or("(なし)".to_string(), |v| v.to_string()),
                        wa.get(i).map_or("(なし)".to_string(), |v| v.to_string())
                    ));
                }
            }
            for (file_name, want_body) in &want {
                if &got[file_name] != want_body {
                    let (g, w) = (&got[file_name], want_body);
                    let line = g
                        .lines()
                        .zip(w.lines())
                        .position(|(a, b)| a != b)
                        .map(|i| i + 1)
                        .unwrap_or_else(|| g.lines().count().min(w.lines().count()) + 1);
                    diffs.push(format!(
                        "{name}/{file_name}: {line}行目から違う\n  新: {}\n  旧: {}",
                        g.lines().nth(line - 1).unwrap_or("(なし)"),
                        w.lines().nth(line - 1).unwrap_or("(なし)")
                    ));
                }
            }
        }
        assert!(
            diffs.is_empty(),
            "旧版との差 {} 件:\n{}",
            diffs.len(),
            diffs.join("\n")
        );
    }

    /// 旧版と比べない(新版の決めた動作を確かめる)ケース。cases.json の `expect`
    #[tokio::test]
    async fn test_patterns_with_expectations() {
        let dir = patterns_dir();
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap())
                .unwrap();
        let mut problems: Vec<String> = Vec::new();

        // live_only(本番の公園データが要るケース)は、verify.py で確かめる
        for case in cases
            .iter()
            .filter(|c| c.get("expect").is_some() && c.get("live_only").is_none())
        {
            let name = case["name"].as_str().unwrap();
            let expect = &case["expect"];
            let file = std::fs::read(dir.join(case["file"].as_str().unwrap())).unwrap();
            let mut options = case["options"].clone();
            options["format"] = case["format"].clone();
            let options = options.to_string();

            // 確認(/logconv/pota)か、変換(/logconv/hamlog)か
            let (status, body): (StatusCode, Vec<u8>) = if case["endpoint"] == "check" {
                let res = pota_check(None, multipart_of_bytes(&file, &options).await).await;
                let status = res.status();
                let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                    .await
                    .unwrap();
                (status, bytes.to_vec())
            } else {
                let res = hamlog_convert(None, multipart_of_bytes(&file, &options).await).await;
                let status = res.status();
                let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
                    .await
                    .unwrap();
                (status, bytes.to_vec())
            };

            if let Some(http) = expect.get("http") {
                if http.as_u64() != Some(status.as_u16() as u64) {
                    problems.push(format!("{name}: HTTP {status}(期待 {http})"));
                    continue;
                }
            }
            // JSON の応答(確認・拒否)
            if expect.get("error_lines").is_some() || expect.get("status").is_some() {
                let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
                if let Some(st) = expect.get("status") {
                    if &json["status"] != st {
                        problems.push(format!("{name}: status {}(期待 {st})", json["status"]));
                    }
                }
                if let Some(lines) = expect.get("error_lines") {
                    let got: Vec<u64> = match &json["error_log"] {
                        serde_json::Value::Array(v) => v
                            .iter()
                            .filter_map(|l| l.as_str()?.split('行').next()?.parse().ok())
                            .collect(),
                        serde_json::Value::String(s) => s
                            .lines()
                            .filter_map(|l| l.split('行').next()?.parse().ok())
                            .collect(),
                        _ => vec![],
                    };
                    let want: Vec<u64> = lines
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|v| v.as_u64())
                        .collect();
                    if got != want {
                        problems.push(format!("{name}: 誤りの行 {got:?}(期待 {want:?})"));
                    }
                }
            }
            // ZIP の中身
            if expect.get("files").is_some()
                || expect.get("lines").is_some()
                || expect.get("records").is_some()
            {
                let got = unzip(&body);
                if let Some(files) = expect.get("files") {
                    let want: Vec<&str> = files
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect();
                    let names: Vec<&str> = got.keys().map(String::as_str).collect();
                    if names != want {
                        problems.push(format!("{name}: ファイル {names:?}(期待 {want:?})"));
                    }
                }
                if let Some(lines) = expect.get("lines").and_then(|v| v.as_object()) {
                    for (file_name, n) in lines {
                        let count = got.get(file_name).map_or(0, |b| b.lines().count());
                        if Some(count as u64) != n.as_u64() {
                            problems.push(format!("{name}/{file_name}: {count} 行(期待 {n})"));
                        }
                    }
                }
                if let Some(records) = expect.get("records").and_then(|v| v.as_object()) {
                    for (file_name, n) in records {
                        let count = got.get(file_name).map_or(0, |b| b.matches("<EOR>").count());
                        if Some(count as u64) != n.as_u64() {
                            problems.push(format!("{name}/{file_name}: {count} 記録(期待 {n})"));
                        }
                    }
                }
            }
        }
        assert!(
            problems.is_empty(),
            "期待との差 {} 件:\n{}",
            problems.len(),
            problems.join("\n")
        );
    }
}
