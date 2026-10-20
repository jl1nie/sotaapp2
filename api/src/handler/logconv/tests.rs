use super::input::{decode_text, read_log};
use super::states::single_state;
use super::*;
use axum::http::{header, StatusCode};

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
fn trailing_sub_eof_is_dropped() {
    assert_eq!(decode_text(b"a,b\r\n\x1a"), "a,b\r\n");
    assert_eq!(decode_text(b"a,b\x1a"), "a,b");
    assert_eq!(decode_text(b"a,b\n\x1a\x1a\r\n"), "a,b\n");
    assert_eq!(decode_text(b"a,b\n"), "a,b\n");
}

#[test]
fn line_numbers_count_crlf_lf_and_cr() {
    let starts = super::input::line_starts("a\r\nb\nc\rd");
    assert_eq!(starts, [0, 3, 5, 7]);
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

// 誤りの知らせの行番号は、入力ファイルの行。改行が CRLF(Windows の HAMLOG)でも、値を `"` で囲んでも、空行があっても
#[test]
fn test_source_lines_are_the_lines_of_the_file() {
    let row = |call: &str| format!("{call},25/10/18,01:00U,59,59,7.030,CW,,,N,,,,,");
    let quoted = |call: &str| {
        row(call)
            .split(',')
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(",")
    };
    for (name, csv) in [
        ("LF", format!("{}\n{}\n{}\n", row("A"), row("B"), row("C"))),
        (
            "CRLF",
            format!("{}\r\n{}\r\n{}\r\n", row("A"), row("B"), row("C")),
        ),
        (
            "CRLF・引用符",
            format!("{}\r\n{}\r\n{}\r\n", quoted("A"), quoted("B"), quoted("C")),
        ),
        (
            "空行",
            format!("{}\r\n\r\n{}\r\n{}", row("A"), row("B"), row("C")),
        ),
    ] {
        let (records, _) = read_log(csv.as_bytes());
        let lines: Vec<usize> = records.iter().map(|q| q.source_line).collect();
        let want = if name == "空行" {
            [1, 3, 4]
        } else {
            [1, 2, 3]
        };
        assert_eq!(lines, want, "{name}");
    }
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
    let opts =
        r#"{"format":"pota","myQTH":"rmks1","POTAActivator":"JA1ZZZ/1","POTAOperator":"JA1ZZZ"}"#;
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
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
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
        serde_json::from_str(&std::fs::read_to_string(dir.join("cases.json")).unwrap()).unwrap();
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

// ---- e2e: FLE Online の HAMLOG の CSV(test_data/logconv/fle)----
mod fle;

// ---- 追加のテスト(担当2): 画面の options・例外系・文字コード ----
mod encoding;
mod errors;
mod options;
mod support;
