//! 例外系(計画の C): 誤った入力・欠けたリクエスト・誤りのある行・日時
//!
//! 決まり(どのテストにも通じるもの):
//! - どの入力でも、500 やパニックにならない。
//! - 失敗(4xx)は、利用者に読めるメッセージ(JSON の `status: NG` と `error`)を返す。
//! - ログの一部の行だけが誤りのとき、`pota` は ADIF を作らず 400 と誤りの一覧(`error_log`)を返す。
//!   `sota_a`・`sota_c` は、誤りの行を `sota000000.csv` に集めて、残りを変換する(200)。
//!
//! フィクスチャは `test_data/logconv/errors`(`generate.py` で作る)。
//! 仕様の確認が要る点(いまの振る舞いを固定したもの)は、各テストのコメントに書いた。

use super::support::*;
use super::*;
use axum::http::StatusCode;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn fixture(name: &str) -> Vec<u8> {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_data/logconv/errors");
    std::fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// 3つの形式の指定(SOTA アクティベーター・SOTA チェイサー・POTA)
const FORMATS: [(&str, &str); 3] = [("sota_a", SOTA_A), ("sota_c", SOTA_C), ("pota", POTA)];

/// 参照のある正常な行(POTA でも SOTA でも変換できる)
fn ok_row(call: &str) -> String {
    row(
        call,
        "25/10/18",
        "01:00U",
        "",
        "JA-0002",
        "JA/TK-001 JA-0001",
        "",
    )
}

fn ok_row_at(call: &str, date: &str, time: &str) -> String {
    row(call, date, time, "", "JA-0002", "JA/TK-001 JA-0001", "")
}

/// 日時が存在しない行(誤り)
fn bad_row(call: &str) -> String {
    row(
        call,
        "25/10/18",
        "99:99U",
        "",
        "JA-0002",
        "JA/TK-001 JA-0001",
        "",
    )
}

// ================= C1. 入力ファイル =================

// 空・見出しだけ・空行だけ: 読める交信がないので 400(どの形式でも、確認でも)
#[tokio::test]
async fn files_without_any_record_are_a_bad_request() {
    for name in ["empty.csv", "header_only_ios.csv"] {
        for (format, opts) in FORMATS {
            let r = hamlog(&fixture(name), opts).await;
            assert_eq!(r.status, StatusCode::BAD_REQUEST, "{name} {format}");
            assert_well_formed(name, &r);
            assert_eq!(
                r.json()["error"],
                "No valid records found",
                "{name} {format}"
            );
        }
        let r = confirm(&fixture(name), POTA).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{name} check");
        assert_well_formed(name, &r);
    }
    // 改行だけの行(空白・タブだけの行は別。下のテスト)
    for bytes in [&b"\n\n\r\n"[..], b"\r\n", b"\n"] {
        let r = hamlog(bytes, SOTA_A).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bytes:?}");
        assert_well_formed("空行だけ", &r);
    }
}

// 空白・タブだけの行は、列の足りない行(誤り)になる。いまの振る舞いを固定
// (仕様の確認が要る: 空行と同じく読み飛ばすか。HAMLOG が空白だけの行を出すことはない)
#[tokio::test]
async fn whitespace_only_lines_are_error_rows() {
    let r = hamlog(&fixture("blank_lines.csv"), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_well_formed("空白だけの行", &r);
    assert_eq!(r.error_lines(), [3, 4]);
}

// 画像などのバイナリ: 500 やパニックにならない。
// SOTA も POTA も 400 と誤りの一覧(読める交信が1つもない)
#[tokio::test]
async fn binary_file_is_not_a_crash() {
    let png = fixture("png_like.bin");
    for (format, opts) in FORMATS {
        let r = try_hamlog(&png, opts).await.expect("パニック");
        assert_well_formed(format, &r);
    }
    for opts in [SOTA_A, SOTA_C] {
        let r = hamlog(&png, opts).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
        assert!(!r.error_lines().is_empty());
    }
    assert_eq!(hamlog(&png, POTA).await.status, StatusCode::BAD_REQUEST);
    let c = confirm(&png, POTA).await;
    assert_eq!(c.status, StatusCode::OK);
    assert_eq!(c.json()["status"], "NG");
}

// 拡張子(ファイル名)は見ない: 中身で CSV か ADIF かを決める
#[tokio::test]
async fn the_file_name_does_not_matter() {
    let adif = "ADIF Export\n<EOH>\n<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:4>0100<BAND:3>40m<MODE:2>CW\
                <MY_SIG_INFO:7>JA-0001<EOR>\n";
    let csv = ok_row("JA1AAA") + "\n";
    for (name, body) in [
        ("log.csv", adif.to_string()),
        ("log.adi", csv.clone()),
        ("log.png", csv),
        ("", adif.into()),
    ] {
        let parts = [
            Part {
                name: "file",
                filename: Some(name),
                body: body.as_bytes(),
            },
            Part::field("options", POTA),
        ];
        let res = hamlog_convert(None, multipart_parts(&parts).await).await;
        let r = Reply::of(res).await;
        assert_eq!(r.status, StatusCode::OK, "{name}: {}", r.text());
        assert!(r.is_zip, "{name}");
    }
}

// 1行目に `ADIF` か `<EOH>` を含む行があれば ADIF とみなす(`read_records` の決まり)。
// 仕様の確認が要る: HAMLOG CSV の1行目の備考に「ADIF」と書くと、CSV が ADIF とみなされ、全部の交信が読めず 400 になる
#[tokio::test]
async fn a_csv_whose_first_line_mentions_adif_is_taken_for_adif() {
    let first = row(
        "JA1AAA",
        "25/10/18",
        "01:00U",
        "",
        "JA-0002",
        "JA-0001",
        "ADIF export",
    );
    let r = hamlog(format!("{first}\n").as_bytes(), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["error"], "No valid records found");
}

// 15列より多い行は、余りの列を無視して読む(エラーにしない)
#[tokio::test]
async fn extra_columns_are_ignored() {
    let r = confirm(&fixture("too_many_columns.csv"), POTA).await;
    let j = r.json();
    assert_eq!(j["status"], "OK", "{j}");
    assert_eq!(j["log_text"].as_array().unwrap().len(), 2);
}

// 引用符が閉じない行: その行から後ろが1つの項目に飲み込まれる。
// 誤りは、引用符の行(2行目)として知らせる。500 にならず、POTA は ADIF を作らない。
// 仕様の確認が要る: 飲み込まれた3行目は、誤りの一覧に出ない(2行目を直せば読める)
#[tokio::test]
async fn unclosed_quote_is_reported_at_its_line() {
    let bytes = fixture("unclosed_quote.csv");
    let r = hamlog(&bytes, POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.error_lines(), [2]);

    let files = hamlog(&bytes, SOTA_C).await.zip();
    assert_eq!(files["other20251018.csv"].lines().count(), 1); // 1行目だけ
    assert_eq!(files["sota000000.csv"].lines().count(), 1);
    assert!(files["sota000000.csv"].starts_with("\u{feff}HamLog format error at Line 2. :"));
}

// 1000 行と 1万行: 時間を出力する(`--nocapture` で見える)。時間の条件は付けず、出力の数だけ確かめる。
// 1万行は、行の開始位置を先に作って二分探索で引くので、debug ビルドでも数秒(以前は行数の2乗で、1つの変換が約 40 秒かかった)。
// 計測用なので、普段は流さない
#[tokio::test]
async fn a_thousand_rows_are_converted() {
    converts_n_rows(1_000).await;
}

#[tokio::test]
#[ignore = "計測用: 1万行(810KB)の変換時間を出す(行番号を二分探索にしたので、debug でも数秒)"]
async fn ten_thousand_rows_are_converted() {
    converts_n_rows(10_000).await;
}

async fn converts_n_rows(n_rows: usize) {
    let days = n_rows.div_ceil(1440);
    let n = n_rows;
    // 1日 1440 分を割りふり、日にまたがる。1行おきに相手の山(S2S)
    let mut csv = String::new();
    for i in 0..n {
        let (day, minute) = (18 + i / 1440, i % 1440);
        let time = format!("{:02}:{:02}U", minute / 60, minute % 60);
        let qth = if i % 2 == 0 {
            "JA/KN-006 JA-0002"
        } else {
            "JA-0003"
        };
        csv += &row(
            &format!("JA1{:03}", i % 1000),
            &format!("25/10/{day}"),
            &time,
            "山田",
            qth,
            "JA/TK-001 JA-0001",
            "",
        );
        csv += "\r\n";
    }
    let bytes = csv.as_bytes();

    let t = Instant::now();
    let a = hamlog(bytes, SOTA_A).await;
    let t_a = t.elapsed();
    assert_eq!(a.status, StatusCode::OK);
    let files = a.zip();
    let total: usize = files
        .iter()
        .filter(|(k, _)| k.ends_with(".csv") && !k.contains("s2s"))
        .map(|(_, v)| v.lines().count())
        .sum();
    let s2s: usize = files
        .iter()
        .filter(|(k, _)| k.contains("s2s"))
        .map(|(_, v)| v.lines().count())
        .sum();
    assert_eq!((total, s2s), (n, n / 2));
    assert_eq!(files.len(), days * 2 + 1); // 日数 × (CSV, S2S) + ADIF 1つ
    assert_eq!(
        files
            .values()
            .map(|b| b.matches("<EOR>").count())
            .sum::<usize>(),
        n
    );

    let t = Instant::now();
    let c = hamlog(bytes, SOTA_C).await;
    let t_c = t.elapsed();
    assert_eq!(c.status, StatusCode::OK);
    let got: usize = c.zip().values().map(|b| b.lines().count()).sum();
    assert_eq!(got, n);

    let t = Instant::now();
    let p = hamlog(bytes, POTA).await;
    let t_p = t.elapsed();
    assert_eq!(p.status, StatusCode::OK);
    let got: usize = p.zip().values().map(|b| b.matches("<EOR>").count()).sum();
    // 自局 JA/TK-001(山)と JA-0001(公園)の2つ、相手の公園は 1 件ずつ
    assert_eq!(got, n * 2);

    let t = Instant::now();
    let k = confirm(bytes, POTA).await;
    let t_k = t.elapsed();
    assert_eq!(k.json()["log_text"].as_array().unwrap().len(), n);

    println!(
        "{n} 行({} KB): sota_a {t_a:?} / sota_c {t_c:?} / pota {t_p:?} / 確認 {t_k:?}(出力 ZIP: sota_a {} KB、pota {} KB)",
        bytes.len() / 1024,
        a.body.len() / 1024,
        p.body.len() / 1024
    );
}

// 上限(8MB)を超える入力は、ハンドラーより手前の `DefaultBodyLimit`(`upload_limit`)が弾く。
// 本物のルーターは DB などの状態(AppState)が要るので、同じ上限をかけた最小のルーターに、同じ処理を載せて確かめる。
// v2 と同じく、外側に全体の上限(64MB)をかけて入れ子にし、内側の logconv の上限が効くことも確かめる。
// 上限を超えると、413 と「ファイルが大きすぎます(上限 8MB)」を返す
#[tokio::test]
async fn body_over_the_limit_is_rejected_before_the_handler() {
    use axum::{extract::DefaultBodyLimit, extract::Multipart, routing::post, Router};
    use tower::ServiceExt;

    let v2 = include_str!("../../v2.rs");
    assert!(v2.contains(r#".nest("/logconv", logconv_router())"#));
    assert!(include_str!("../mod.rs").contains(".layer(upload_limit())"));

    let app = || {
        let logconv = Router::new()
            .route(
                "/hamlog",
                post(|mp: Multipart| async move { hamlog_convert(None, mp).await }),
            )
            .layer(upload_limit());
        Router::new()
            .nest("/logconv", logconv)
            .layer(DefaultBodyLimit::max(1024 * 1024 * 64))
    };
    let send = |file_len: usize| {
        let mut body =
            b"--B\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.csv\"\r\n\r\n"
                .to_vec();
        // 1行の備考に大きな文字列を入れる(本文の大きさだけを変え、変換は軽くする)
        let filler = "x".repeat(file_len.saturating_sub(body.len() + 200));
        let line = row(
            "JA1AAA",
            "25/10/18",
            "01:00U",
            "",
            "JA-0002",
            "JA/TK-001 JA-0001",
            &filler,
        );
        body.extend_from_slice(line.as_bytes());
        body.extend_from_slice(format!("\r\n--B\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{SOTA_C}\r\n--B--\r\n").as_bytes());
        axum::http::Request::builder()
            .method("POST")
            .uri("/logconv/hamlog")
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=B")
            .body(axum::body::Body::from(body))
            .unwrap()
    };

    // 上限の手前は通る
    let t = Instant::now();
    let r = Reply::of(
        app()
            .oneshot(send(MAX_UPLOAD_MB * 1024 * 1024 - 4096))
            .await
            .unwrap(),
    )
    .await;
    println!("上限の手前: {} {:?}", r.status, t.elapsed());
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        r.text().chars().take(200).collect::<String>()
    );
    // 上限を超えると、500 ではなく 4xx
    let t = Instant::now();
    let r = Reply::of(
        app()
            .oneshot(send(MAX_UPLOAD_MB * 1024 * 1024 + 1024))
            .await
            .unwrap(),
    )
    .await;
    println!("上限を超える: {} {:?}", r.status, t.elapsed());
    assert!(
        r.status.is_client_error(),
        "{}: {}",
        r.status,
        r.text().chars().take(200).collect::<String>()
    );
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(r.json()["status"], "NG");
    assert_eq!(r.json()["error"], "ファイルが大きすぎます(上限 8MB)");
}

// multipart でないリクエスト(Content-Type が違う・本文がただの JSON)は、axum の拒否(4xx)で、500 にならない。
// 仕様の確認が要る: その応答は JSON ではなく平文(画面は読めるか)
#[tokio::test]
async fn non_multipart_request_is_rejected_with_4xx() {
    use axum::{extract::Multipart, routing::post, Router};
    use tower::ServiceExt;
    let app = Router::new().route(
        "/hamlog",
        post(|mp: Multipart| async move { hamlog_convert(None, mp).await }),
    );
    for content_type in ["application/json", "text/plain", "multipart/form-data"] {
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/hamlog")
            .header(header::CONTENT_TYPE, content_type)
            .body(axum::body::Body::from(SOTA_C))
            .unwrap();
        let r = Reply::of(app.clone().oneshot(req).await.unwrap()).await;
        assert!(r.status.is_client_error(), "{content_type}: {}", r.status);
    }
}

// ================= C2. multipart と options =================

fn parts<'a>(file: &'a [u8], options: Option<&'a str>) -> Vec<Part<'a>> {
    let mut v = vec![Part::file(file)];
    if let Some(o) = options {
        v.push(Part::field("options", o));
    }
    v
}

async fn send_parts(parts: &[Part<'_>]) -> Reply {
    Reply::of(hamlog_convert(None, multipart_parts(parts).await).await).await
}

async fn send_check_parts(parts: &[Part<'_>]) -> Reply {
    Reply::of(pota_check(None, multipart_parts(parts).await).await).await
}

#[tokio::test]
async fn missing_file_is_a_bad_request() {
    let p = [Part::field("options", SOTA_C)];
    for r in [send_parts(&p).await, send_check_parts(&p).await] {
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
        assert_well_formed("file なし", &r);
        assert_eq!(r.json()["error"], "Missing 'file' parameter");
    }
    // パートが何もない
    let r = send_parts(&[]).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["error"], "Missing 'file' parameter");
    // 名前の違うパート(`File`・`log`)は、file ではない
    for name in ["File", "log", "files", ""] {
        let csv = ok_row("JA1AAA");
        let r = send_parts(&[
            Part {
                name,
                filename: Some("a.csv"),
                body: csv.as_bytes(),
            },
            Part::field("options", SOTA_C),
        ])
        .await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "name={name:?}");
    }
}

#[tokio::test]
async fn missing_options_is_a_bad_request_for_hamlog() {
    let csv = ok_row("JA1AAA");
    let r = send_parts(&parts(csv.as_bytes(), None)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_well_formed("options なし", &r);
    assert!(r.json()["error"].as_str().unwrap().contains("options"));
}

#[tokio::test]
async fn invalid_options_json_is_a_bad_request_for_hamlog() {
    let csv = ok_row("JA1AAA");
    let bad = [
        "",
        "{",
        "{not json}",
        "not json",
        "[]",
        "null",
        "5",
        "\"sota_a\"",
        "{\"format\":}",
        // 型の違い: format が文字列でない・値の型が違う
        r#"{"format":5}"#,
        r#"{"format":null}"#,
        r#"{"format":["sota_a"]}"#,
        r#"{"format":"sota_a","Summit":5}"#,
        r#"{"format":"sota_a","myQTH":1}"#,
        r#"{"format":"sota_a","POTAActivator":["JA1ZZZ"]}"#,
        r#"{"format":"sota_a","QTH":true}"#,
    ];
    for o in bad {
        let r = send_parts(&parts(csv.as_bytes(), Some(o))).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{o}");
        assert_well_formed(o, &r);
        assert!(
            r.json()["error"]
                .as_str()
                .unwrap()
                .starts_with("Invalid options JSON"),
            "{o}"
        );
    }
}

// 未知の `format`(大文字小文字も区別)・空・`format` がない、は 400
#[tokio::test]
async fn unknown_or_missing_format_is_a_bad_request() {
    let csv = ok_row("JA1AAA");
    for o in [
        r#"{"format":"nope"}"#,
        r#"{"format":""}"#,
        r#"{"format":"SOTA_A"}"#,
        r#"{"format":"sota"}"#,
        r#"{"format":"adif"}"#,
        r#"{"myQTH":"rmks1"}"#,
        "{}",
    ] {
        let r = send_parts(&parts(csv.as_bytes(), Some(o))).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{o}");
        assert_well_formed(o, &r);
    }
}

// 未知の項目と未知の `command` は無視する(旧版の画面は `command` を送っていた)
#[tokio::test]
async fn unknown_fields_and_commands_are_ignored() {
    let csv = ok_row("JA1AAA");
    for o in [
        r#"{"format":"sota_c","QTH":"qth","callsign":"JA1ZZZ","command":"Unknown"}"#,
        r#"{"format":"sota_c","QTH":"qth","callsign":"JA1ZZZ","command":"ADIFCSVCheck"}"#,
        r#"{"format":"sota_c","QTH":"qth","callsign":"JA1ZZZ","extra":{"a":[1,2]},"Foo":null}"#,
    ] {
        let r = send_parts(&parts(csv.as_bytes(), Some(o))).await;
        assert_eq!(r.status, StatusCode::OK, "{o}");
        assert!(r.is_zip, "{o}");
    }
}

// `options` は `arg` でもよい(旧版の名前)。パートの順番は問わず、関係のないパートは無視する。
// 同じ名前のパートが2つあれば、あとのものが使われる(いまの振る舞いを固定)
#[tokio::test]
async fn options_alias_order_and_duplicates() {
    let csv = ok_row("JA1AAA");
    let c = csv.as_bytes();
    // arg
    let r = send_parts(&[Part::file(c), Part::field("arg", SOTA_C)]).await;
    assert!(r.is_zip);
    // options が先
    let r = send_parts(&[Part::field("options", SOTA_C), Part::file(c)]).await;
    assert!(r.is_zip);
    // 関係のないパート
    let r = send_parts(&[
        Part::field("note", "x"),
        Part::file(c),
        Part::field("options", SOTA_C),
        Part::field("z", ""),
    ])
    .await;
    assert!(r.is_zip);
    // options が2つ: あとのものが使われる(1つ目が不正でも、2つ目が正しければ通る)
    let r = send_parts(&[
        Part::file(c),
        Part::field("options", "{"),
        Part::field("options", SOTA_C),
    ])
    .await;
    assert!(r.is_zip);
    let r = send_parts(&[
        Part::file(c),
        Part::field("options", SOTA_C),
        Part::field("options", "{"),
    ])
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    // file が2つ: あとのものが使われる
    let empty: &[u8] = b"";
    let r = send_parts(&[
        Part::file(c),
        Part::file(empty),
        Part::field("options", SOTA_C),
    ])
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = send_parts(&[
        Part::file(empty),
        Part::file(c),
        Part::field("options", SOTA_C),
    ])
    .await;
    assert!(r.is_zip);
}

// `file` が空のパートは、読める交信がなく 400
#[tokio::test]
async fn empty_file_part_is_a_bad_request() {
    let r = send_parts(&parts(b"", Some(SOTA_C))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["error"], "No valid records found");
}

// options が UTF-8 でないバイト列のときは、壊れた字を置き換え文字にして読む(400 にはならない)。
// 仕様の確認が要る: 値(コールサインなど)に置き換え文字がそのまま入る
#[tokio::test]
async fn options_that_are_not_utf8_are_read_lossily() {
    let csv = ok_row("JA1AAA") + "\n";
    let bad = Part {
        name: "options",
        filename: None,
        body: b"{\"format\":\"sota_c\",\"QTH\":\"qth\",\"callsign\":\"JA\xff\xfe\"}",
    };
    let r = send_parts(&[Part::file(csv.as_bytes()), bad]).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.zip()["other20251018.csv"].starts_with("V2,JA\u{FFFD}\u{FFFD},,"));
}

// 本文が途中で切れた multipart(通信の中断)は、500 にならず 400。
// 仕様の確認が要る: メッセージは「Missing 'file' parameter」(切れたことは伝わらない)
#[tokio::test]
async fn truncated_multipart_is_a_bad_request() {
    use axum::extract::FromRequest;
    let csv = ok_row("JA1AAA");
    for cut in [0usize, 30, 80, 120] {
        let body = format!(
            "--B\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.csv\"\r\n\r\n{csv}\r\n--B\r\nContent-Disposition: form-data; name=\"options\"\r\n\r\n{SOTA_C}\r\n--B--\r\n"
        );
        let body = &body.as_bytes()[..cut.min(body.len())];
        let req = axum::http::Request::builder()
            .method("POST")
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=B")
            .body(axum::body::Body::from(body.to_vec()))
            .unwrap();
        let mp = Multipart::from_request(req, &()).await.unwrap();
        let r = Reply::of(hamlog_convert(None, mp).await).await;
        assert!(r.status.is_client_error(), "cut={cut}: {}", r.status);
        assert_well_formed("途中で切れた本文", &r);
    }
}

// 確認(/logconv/pota)は、options が読めなくても、既定の指定で進める(file がなければ 400)。
// 仕様の確認が要る: 読めない options を黙って既定にする(画面は気づけない)
#[tokio::test]
async fn check_endpoint_falls_back_to_defaults() {
    let csv = ok_row("JA1AAA");
    for o in [
        None,
        Some(""),
        Some("{"),
        Some("[]"),
        Some("null"),
        Some(r#"{"format":"nope"}"#),
        Some(r#"{"Summit":5}"#),
    ] {
        let r = send_check_parts(&parts(csv.as_bytes(), o)).await;
        assert_eq!(r.status, StatusCode::OK, "{o:?}");
        assert_eq!(r.json()["status"], "OK", "{o:?}");
    }
}

// ================= C3. 応答: 500 やパニックにならない =================

/// 乱数(再現できるよう、種を固定した線形合同法)
fn noise(len: usize, mut seed: u32) -> Vec<u8> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect()
}

// 変な入力を、すべての形式・確認に送り、500 やパニックにならず、失敗には読めるメッセージがあること
// (ADIF の既知のパニックは別のテスト。ここでは除く)
#[tokio::test]
async fn odd_inputs_never_crash() {
    let ok = ok_row("JA1AAA");
    let mut inputs: Vec<(String, Vec<u8>)> = vec![
        ("NUL 1つ".into(), vec![0]),
        ("0x1A だけ".into(), vec![0x1a]),
        ("コンマだけ".into(), b",".to_vec()),
        ("15個のコンマ".into(), b",,,,,,,,,,,,,,,\n".to_vec()),
        ("引用符1つ".into(), b"\"".to_vec()),
        ("引用符3つ".into(), b"\"\"\"".to_vec()),
        ("2列".into(), b"a,b\n".to_vec()),
        ("0xFF が100".into(), vec![0xff; 100]),
        ("ランダム 4KB".into(), noise(4096, 1)),
        ("ランダム 64KB".into(), noise(65536, 2)),
        ("1行 1MB".into(), vec![b'a'; 1 << 20]),
        ("コンマ 2万個".into(), vec![b','; 20_000]),
        ("改行 2万個".into(), vec![b'\n'; 20_000]),
        ("CR 2万個".into(), vec![b'\r'; 20_000]),
        ("絵文字の行".into(), "😀,2025/10/18,01:00U,59,59,7.030,CW,,,N,😀,😀,😀,😀,\n".to_string().into_bytes()),
        ("右から左の文字".into(), "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,\u{202e}שלום,,JA-0001,,\n".to_string().into_bytes()),
        ("正常な行の途中に NUL".into(), ok.replacen("JA1AAA", "JA1\0AAA", 1).into_bytes()),
        ("全角の数字の日付".into(), "JA1AAA,２５/１０/１８,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,\n".as_bytes().to_vec()),
        ("全角の周波数".into(), "JA1AAA,25/10/18,01:00U,59,59,７.０３０,CW,,,N,,,JA-0001,,\n".as_bytes().to_vec()),
        ("巨大な数の日付".into(), b"JA1AAA,99999999999999999999/1/1,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("年 0".into(), b"JA1AAA,0/1/1,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("4桁の年 0000".into(), b"JA1AAA,0000/1/1,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("周波数 0".into(), b"JA1AAA,25/10/18,01:00U,59,59,0,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("周波数 負".into(), b"JA1AAA,25/10/18,01:00U,59,59,-7.030,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("周波数 巨大".into(), b"JA1AAA,25/10/18,01:00U,59,59,1e308,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("周波数 NaN".into(), b"JA1AAA,25/10/18,01:00U,59,59,NaN,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("周波数 inf".into(), b"JA1AAA,25/10/18,01:00U,59,59,inf,CW,,,N,,,JA-0001,,\n".to_vec()),
        ("参照が巨大な数".into(), b"JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,JA-99999999999999999999,JA/TK-99999999999999999999,,\n".to_vec()),
        ("座標が巨大".into(), b"JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,JA/TK-001 99999999999999999999N,99999999999999999999E,\n".to_vec()),
        // iOS 形式
        ("iOS 時差が巨大".into(), iosrow("2025-10-18 01:00:00 +9999")),
        ("iOS 時差が負".into(), iosrow("2025-10-18 01:00:00 -2359")),
        ("iOS 存在しない日時".into(), iosrow("2025-13-45 99:99:99 +0900")),
        ("iOS 時差なし".into(), iosrow("2025-10-18 01:00:00")),
        ("iOS 見出し2回".into(), b"TimeOn\nTimeOn\n".to_vec()),
        // ADIF(空・壊れたもの。パニックする入力は別のテスト)
        ("ADIF だけ".into(), b"ADIF".to_vec()),
        ("<EOH> だけ".into(), b"<EOH>".to_vec()),
        ("<EOH><EOR>".into(), b"<EOH><EOR>".to_vec()),
        ("ADIF 長さ 0".into(), b"ADIF\n<EOH>\n<CALL:0><EOR>".to_vec()),
        ("ADIF 長さが足りない".into(), b"ADIF\n<EOH>\n<CALL:50>ab<EOR>".to_vec()),
        ("ADIF 長さが負".into(), b"ADIF\n<EOH>\n<CALL:-1>x<EOR>".to_vec()),
        ("ADIF 長さが巨大".into(), b"ADIF\n<EOH>\n<CALL:99999999999999999999>x<EOR>".to_vec()),
        ("ADIF 閉じない<".into(), b"ADIF\n<EOH>\n<CALL:6".to_vec()),
        ("ADIF タグだけ".into(), b"ADIF\n<EOH>\n<<<<>>>><:<:>EOR>".to_vec()),
        ("ADIF 型つき".into(), b"ADIF\n<EOH>\n<CALL:6:S>JA1AAA<QSO_DATE:8:D>20251018<TIME_ON:4>0100<EOR>".to_vec()),
        ("ADIF 小文字のタグ".into(), b"adif\n<eoh>\n<call:6>JA1AAA<qso_date:8>20251018<time_on:4>0100<eor>".to_vec()),
        ("ADIF 日本語のコールサイン".into(), "ADIF\n<EOH>\n<CALL:9>日本語<QSO_DATE:8>20251018<TIME_ON:4>0100<EOR>".as_bytes().to_vec()),
        ("ADIF 日付が短い".into(), b"ADIF\n<EOH>\n<CALL:6>JA1AAA<QSO_DATE:7>2025101<TIME_ON:4>0100<EOR>".to_vec()),
        ("ADIF 時刻が短い".into(), b"ADIF\n<EOH>\n<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:3>010<EOR>".to_vec()),
        ("ADIF 日付が数字でない".into(), b"ADIF\n<EOH>\n<CALL:6>JA1AAA<QSO_DATE:8>abcdefgh<TIME_ON:4>zzzz<EOR>".to_vec()),
        ("ADIF ランダム".into(), [b"ADIF\n<EOH>\n".to_vec(), noise(4096, 3)].concat()),
    ];
    // 正常な行を、いろいろな位置で切ったもの(途中で終わる行)
    for cut in (1..ok.len()).step_by(7) {
        inputs.push((
            format!("正常な行を{cut}バイトで切る"),
            ok.as_bytes()[..cut].to_vec(),
        ));
    }

    let mut problems = Vec::new();
    for (label, bytes) in &inputs {
        for (format, opts) in FORMATS {
            match try_hamlog(bytes, opts).await {
                None => problems.push(format!("{label} / {format}: パニック")),
                Some(r) if r.status.is_server_error() => {
                    problems.push(format!("{label} / {format}: {} {}", r.status, r.text()))
                }
                Some(r) => {
                    if r.status.is_client_error()
                        && serde_json::from_slice::<serde_json::Value>(&r.body).map_or(true, |j| {
                            j["status"] != "NG" || j["error"].as_str().is_none_or(str::is_empty)
                        })
                    {
                        problems.push(format!(
                            "{label} / {format}: 4xx に読めるメッセージがない: {}",
                            r.text()
                        ));
                    }
                }
            }
        }
        match try_confirm(bytes, POTA).await {
            None => problems.push(format!("{label} / 確認: パニック")),
            Some(r) if r.status.is_server_error() => {
                problems.push(format!("{label} / 確認: {} {}", r.status, r.text()))
            }
            Some(_) => {}
        }
    }
    assert!(
        problems.is_empty(),
        "{} 件:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// HamLog iOS の1行(19列以上。日時の列だけ指定)
fn iosrow(datetime: &str) -> Vec<u8> {
    let mut cols = vec![""; 19];
    cols[0] = datetime;
    cols[2] = "7.030";
    cols[3] = "JA1AAA";
    cols[11] = "CW";
    format!("TimeOn,x\n{}\n", cols.join(",")).into_bytes()
}

// ADIF の既知のパニック(service/src/implement/logconv/adif.rs)。パニックは、どの入力でも不具合

// 長さ(`<COMMENT:4>`)が、読み込み後の UTF-8 のバイト数と合わないと、文字の途中で切ってパニックする。
// 日本語の ADIF を Shift_JIS のバイト数で書いた場合(日本のロガーが出す)に起きる
#[tokio::test]
async fn adif_length_inside_a_multibyte_char_does_not_panic() {
    let bytes = fixture("adif_sjis_comment_len.adi");
    let r = try_hamlog(&bytes, POTA).await.expect("パニックした");
    assert_well_formed("Shift_JIS の ADIF", &r);
    // 期待: 交信は読める(COMMENT は変換に使わない)
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.zip().contains_key("JA1ZZZ-1@JA-0001-20251018.adi"));
}

// QSO_DATE・TIME_ON が ASCII でないと、`[0..4]` が文字の途中にあたってパニックする
#[tokio::test]
async fn adif_non_ascii_date_or_time_does_not_panic() {
    for name in ["adif_fullwidth_date.adi", "adif_fullwidth_time.adi"] {
        let r = try_hamlog(&fixture(name), POTA)
            .await
            .unwrap_or_else(|| panic!("{name}: パニックした"));
        assert_well_formed(name, &r);
        // 期待: 誤りの記録として知らせる(400 と誤りの一覧)
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{name}");
        assert_eq!(r.error_lines(), [1], "{name}");
    }
}

// ヘッダーに、大文字にすると長さの変わる字(ŉ)があると、`<EOH>` の位置がずれて、最初の記録が欠ける
#[tokio::test]
async fn adif_header_with_special_characters_keeps_the_first_record() {
    let r = hamlog(&fixture("adif_header_special_char.adi"), POTA).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text());
    assert!(r.zip().contains_key("JA1ZZZ-1@JA-0001-20251018.adi"));
}

// ADIF の存在しない日時(13月45日・99:99)は、HAMLOG CSV と違って検査されず、そのまま出力に入る
#[tokio::test]
async fn adif_nonexistent_datetime_is_an_error_like_csv() {
    let r = hamlog(&fixture("adif_nonexistent_datetime.adi"), POTA).await;
    assert_eq!(
        r.status,
        StatusCode::BAD_REQUEST,
        "{}",
        r.text().chars().take(300).collect::<String>()
    );
    assert_eq!(r.error_lines(), [1]);
}

// <EOR> のない最後の記録・CALL のない記録は、黙って捨てられる(誤りにも出ない)。
// 途中で切れた ADIF の最後の交信が、知らせなしに消える
#[tokio::test]
async fn adif_record_without_eor_or_call_is_not_dropped_silently() {
    let good = "<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:4>0100<BAND:3>40m<MODE:2>CW<MY_SIG_INFO:7>JA-0001<EOR>\n";
    let cases = [
        // 2つ目に <EOR> がない
        format!("ADIF\n<EOH>\n{good}<CALL:6>JA1BBB<QSO_DATE:8>20251018<TIME_ON:4>0105<MY_SIG_INFO:7>JA-0001"),
        // 2つ目に CALL がない
        format!("ADIF\n<EOH>\n{good}<QSO_DATE:8>20251018<TIME_ON:4>0105<MY_SIG_INFO:7>JA-0001<EOR>\n"),
    ];
    for adif in cases {
        let j = confirm(adif.as_bytes(), POTA).await.json();
        let rows = j["log_text"].as_array().unwrap().len();
        // 2つ読むか、誤りとして知らせるか(1つだけが OK で返るのは、黙って捨てている)
        assert!(
            rows == 2 || j["status"] == "NG",
            "黙って捨てた: {adif}\n{j}"
        );
    }
}

// ADIF の誤りの記録は、元のファイルの行ではなく、記録の番号(何番目か)で知らせる。
// 仕様の確認が要る: 行番号にするか。いまの振る舞い(記録の番号)を固定
#[tokio::test]
async fn adif_errors_are_numbered_by_record() {
    let good = "<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:4>0100<MY_SIG_INFO:7>JA-0001<EOR>\n";
    let short = "<CALL:6>JA1BBB<QSO_DATE:4>2025<TIME_ON:4>0100<MY_SIG_INFO:7>JA-0001<EOR>\n";
    let adif = format!("ADIF\n<EOH>\n\n\n{good}\n\n{short}{good}");
    let r = hamlog(adif.as_bytes(), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.error_lines(), [2]); // 2番目の記録(ファイルでは 7 行目)
}

// ================= C3. 誤りの行番号 =================

/// 3行目だけが誤りのログ(1・2・4 行目は正常)を、改行の形を変えて作る
fn log_with_newline(newline: &str, trailing: bool) -> String {
    let mut s = [ok_row("A"), ok_row("B"), bad_row("C"), ok_row("D")].join(newline);
    if trailing {
        s += newline;
    }
    s
}

// 誤りの行番号が、元のファイルの行と合う(CRLF・LF・改行が混ざる・最後に改行がない・先頭に空行・引用符で囲む)
#[tokio::test]
async fn error_line_numbers_match_the_file() {
    let mixed = format!(
        "{}\r\n{}\n{}\r\n{}",
        ok_row("A"),
        ok_row("B"),
        bad_row("C"),
        ok_row("D")
    );
    let quoted_multiline = {
        // 2行目の備考に、引用符で囲んだ改行がある(物理的には 2〜3 行目)。誤りは 4 行目
        let multi = row(
            "B",
            "25/10/18",
            "01:00U",
            "",
            "JA-0002",
            "\"JA/TK-001\nJA-0001\"",
            "",
        );
        format!(
            "{}\n{}\n{}\n{}\n",
            ok_row("A"),
            multi,
            bad_row("C"),
            ok_row("D")
        )
    };
    let cases: Vec<(&str, String, u64)> = vec![
        ("LF", log_with_newline("\n", true), 3),
        ("LF・最後に改行なし", log_with_newline("\n", false), 3),
        ("CRLF", log_with_newline("\r\n", true), 3),
        ("CRLF・最後に改行なし", log_with_newline("\r\n", false), 3),
        ("混在", mixed, 3),
        (
            "先頭に空行",
            format!("\n\n{}", log_with_newline("\n", true)),
            5,
        ),
        (
            "空行が間にある",
            format!("{}\n\n\n{}\n{}\n", ok_row("A"), bad_row("C"), ok_row("D")),
            4,
        ),
        (
            "CRLF・先頭に BOM",
            format!("\u{feff}{}", log_with_newline("\r\n", true)),
            3,
        ),
        ("引用符の中の改行", quoted_multiline, 4),
    ];
    for (name, csv, want) in cases {
        // POTA の生成(400)
        let r = hamlog(csv.as_bytes(), POTA).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{name}");
        assert_eq!(r.error_lines(), [want], "{name}: 生成");
        // 確認
        assert_eq!(
            confirm(csv.as_bytes(), POTA).await.error_lines(),
            [want],
            "{name}: 確認"
        );
        // SOTA の誤りのファイル
        for opts in [SOTA_A, SOTA_C] {
            let files = hamlog(csv.as_bytes(), opts).await.zip();
            let err = &files["sota000000.csv"];
            assert!(
                err.starts_with(&format!(
                    "\u{feff}HamLog format error at Line {want}. : 日時不正:25/10/18 99:99U,"
                )),
                "{name}: {err}"
            );
        }
    }
}

// 改行が CR だけのファイル(古い Mac)で、誤りの行番号が 1 になる(csv は CR で行を分けるが、行を数えるのは LF だけ)
#[tokio::test]
async fn error_line_numbers_match_the_file_with_cr_only_newlines() {
    let csv = log_with_newline("\r", true);
    let r = hamlog(csv.as_bytes(), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.error_lines(), [3]);
}

// ================= C4. 一部の行だけが誤り / 全行が誤り =================

fn partial_log() -> String {
    // 2行目が日時、4行目が周波数の誤り。1・3・5 行目は正常
    let bad_freq = row(
        "D",
        "25/10/18",
        "01:30U",
        "",
        "JA-0002",
        "JA/TK-001 JA-0001",
        "",
    )
    .replace(",7.030,", ",9.999,");
    [
        ok_row("A"),
        bad_row("B"),
        ok_row_at("C", "25/10/18", "01:20U"),
        bad_freq,
        ok_row_at("E", "25/10/18", "01:40U"),
    ]
    .join("\n")
        + "\n"
}

#[tokio::test]
async fn some_bad_rows_are_split_by_format() {
    let csv = partial_log();
    // pota: 生成しない(400)。誤りの一覧は 2・4 行目。メッセージは行ごとの理由
    let r = hamlog(csv.as_bytes(), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.json()["error"], "The log has errors");
    assert_eq!(r.error_lines(), [2, 4]);
    let log: Vec<String> = r.json()["error_log"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap().into())
        .collect();
    assert_eq!(log[0], "2行:日時不正:25/10/18 99:99U");
    assert!(log[1].starts_with("4行:周波数不正:"), "{}", log[1]);
    // 確認: 200 で NG。誤りの一覧は同じ。表には正常な行も出る(誤りの行は理由が1列)
    let c = confirm(csv.as_bytes(), POTA).await;
    assert_eq!(
        (c.status, c.json()["status"].clone()),
        (StatusCode::OK, "NG".into())
    );
    assert_eq!(c.error_lines(), [2, 4]);
    assert_eq!(c.json()["log_text"].as_array().unwrap().len(), 5);

    // sota_a・sota_c: 200 で、正常な 3 行は日付のファイルに、誤りの 2 行は sota000000.csv に
    let a = hamlog(csv.as_bytes(), SOTA_A).await.zip();
    assert_eq!(a["sota20251018.csv"].lines().count(), 3);
    assert_eq!(a["sota000000.csv"].lines().count(), 2);
    assert_eq!(a["sota20251018.adi"].matches("<EOR>").count(), 3);
    let c = hamlog(csv.as_bytes(), SOTA_C).await.zip();
    assert_eq!(c["other20251018.csv"].lines().count(), 3);
    assert_eq!(c["sota000000.csv"].lines().count(), 2);
    for files in [&a, &c] {
        let lines: Vec<&str> = files["sota000000.csv"].lines().collect();
        assert!(
            lines[0].starts_with("\u{feff}HamLog format error at Line 2. :"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("HamLog format error at Line 4. :"),
            "{}",
            lines[1]
        );
        // 正常なファイルには、誤りの行の交信が混ざらない
        assert!(files
            .iter()
            .filter(|(k, _)| *k != "sota000000.csv")
            .all(|(_, v)| !v.contains(",B,") && !v.contains(",D,")));
    }
}

// 全行が誤り: pota も sota も 400(誤りの一覧に全行)
#[tokio::test]
async fn all_rows_bad_is_an_error_listing_every_line() {
    let csv = [bad_row("A"), bad_row("B"), bad_row("C")].join("\n") + "\n";
    let r = hamlog(csv.as_bytes(), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.error_lines(), [1, 2, 3]);
    let c = confirm(csv.as_bytes(), POTA).await;
    assert_eq!(c.json()["status"], "NG");
    assert_eq!(c.error_lines(), [1, 2, 3]);
    for opts in [SOTA_A, SOTA_C] {
        let r = hamlog(csv.as_bytes(), opts).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
        assert_eq!(r.error_lines(), [1, 2, 3]);
    }
}

// HamLog iOS の CSV でも同じ決まり(誤りの行は 1 行目の見出しを数えた行番号)
#[tokio::test]
async fn ios_rows_follow_the_same_rules() {
    let ios = |datetime: &str, call: &str| {
        let mut cols = vec![""; 19];
        cols[0] = datetime;
        cols[2] = "7.030";
        cols[3] = call;
        cols[8] = "JA-0002";
        cols[11] = "CW";
        cols.join(",")
    };
    let csv = format!(
        "TimeOn,TimeOff,Freq,Callsign\r\n{}\r\n{}\r\n{}\r\n",
        ios("2025-10-18 10:00:00 +0900", "JA1AAA"),
        ios("2025-13-45 10:00:00 +0900", "JA1BBB"),
        ios("2025-10-18 10:05:00 +0900", "JA1CCC")
    );
    let r = hamlog(csv.as_bytes(), POTA).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.error_lines(), [3]);
    let files = hamlog(csv.as_bytes(), SOTA_C).await.zip();
    assert_eq!(files["other20251018.csv"].lines().count(), 2);
    assert!(files["sota000000.csv"].starts_with("\u{feff}HamLog format error at Line 3. :"));
}

// ================= C5. 日時 =================

/// 確認の表の、日付の列(`YYYYMMDD`)と時刻の列(`HHMM`)。誤りの行は None
async fn dates_of(rows: &[(&str, &str)]) -> Vec<Option<(String, String)>> {
    let csv: String = rows
        .iter()
        .map(|(d, t)| ok_row_at("JA1AAA", d, t) + "\n")
        .collect();
    let j = confirm(csv.as_bytes(), POTA).await.json();
    j["log_text"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let r = r.as_array().unwrap();
            (r.len() > 2).then(|| {
                (
                    r[1].as_str().unwrap().to_string(),
                    r[2].as_str().unwrap().to_string(),
                )
            })
        })
        .collect()
}

fn ymd(d: &str, t: &str) -> Option<(String, String)> {
    Some((d.to_string(), t.to_string()))
}

// 存在しない日付・時刻は誤り(2/30・平年の 2/29・31日のない月・0月・0日・13月・24:00・23:60)
#[tokio::test]
async fn nonexistent_dates_and_times_are_errors() {
    let bad = [
        ("25/02/29", "01:00U"),
        ("25/02/30", "01:00U"),
        ("25/04/31", "01:00U"),
        ("25/06/31", "01:00U"),
        ("25/00/10", "01:00U"),
        ("25/10/00", "01:00U"),
        ("25/13/01", "01:00U"),
        ("25/10/32", "01:00U"),
        ("99/02/29", "01:00U"),   // 1999 年は平年
        ("2100/02/29", "01:00U"), // 100 で割れる年は平年
        ("25/10/18", "24:00U"),
        ("25/10/18", "23:60U"),
        ("25/10/18", "99:99U"),
    ];
    let got = dates_of(&bad).await;
    assert_eq!(got.len(), bad.len());
    for (case, g) in bad.iter().zip(&got) {
        assert_eq!(g, &None, "{case:?} は誤りのはず");
    }
}

// うるう日・年またぎ・UTC への換算で日付が変わる
#[tokio::test]
async fn leap_days_and_date_changes_by_timezone() {
    let rows = [
        ("24/02/29", "12:00U"),   // 2024 年はうるう年
        ("00/02/29", "12:00U"),   // 2000 年は 400 で割れるのでうるう年
        ("2400/02/29", "12:00U"), // 同じく
        ("24/03/01", "08:00J"),   // JST 3/1 08:00 = UTC 2/29 23:00(うるう日にもどる)
        ("24/02/29", "23:00J"),   // JST 2/29 23:00 = UTC 2/29 14:00
        ("25/12/31", "23:30U"),   // 年末
        ("26/01/01", "00:30U"),   // 年始
        ("26/01/01", "08:30J"),   // JST 年始 08:30 = UTC 前年末 23:30(年をまたいで戻る)
        ("25/12/31", "23:30J"),   // JST 年末 23:30 = UTC 14:30
        ("25/03/01", "08:59"),    // 付記なしは JST: UTC 2/28 23:59
    ];
    let want = [
        ymd("20240229", "1200"),
        ymd("20000229", "1200"),
        ymd("24000229", "1200"),
        ymd("20240229", "2300"),
        ymd("20240229", "1400"),
        ymd("20251231", "2330"),
        ymd("20260101", "0030"),
        ymd("20251231", "2330"),
        ymd("20251231", "1430"),
        ymd("20250228", "2359"),
    ];
    assert_eq!(dates_of(&rows).await, want);
}

// 年またぎのログは、UTC の日付ごとにファイルが分かれる
#[tokio::test]
async fn a_log_over_new_year_is_split_by_utc_date() {
    let csv = [
        ok_row_at("A", "25/12/31", "23:30U"),
        ok_row_at("B", "26/01/01", "00:30U"),
        ok_row_at("C", "26/01/01", "08:30J"), // UTC では前年末
    ]
    .join("\n");
    let files = hamlog(csv.as_bytes(), SOTA_A).await.zip();
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        ["sota20251231.adi", "sota20251231.csv", "sota20260101.csv"]
    );
    assert_eq!(files["sota20251231.csv"].lines().count(), 2);
    assert_eq!(files["sota20260101.csv"].lines().count(), 1);
    // POTA: 山はその交信の日付のファイル、公園は最初の交信の日付の1つのファイル
    let files = hamlog(csv.as_bytes(), POTA).await.zip();
    let names: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        [
            "JA1ZZZ-1@JA-0001-20251231.adi",
            "JA1ZZZ-1@JA-TK-001-20251231.adi",
            "JA1ZZZ-1@JA-TK-001-20260101.adi"
        ]
    );
}

// 日付が戻るログ(新しい日付のあとに古い日付)でも、交信は消えない(全部のファイルに入る)
#[tokio::test]
async fn dates_going_backwards_lose_nothing() {
    let csv = [
        ok_row_at("A", "25/10/20", "01:00U"),
        ok_row_at("B", "25/10/18", "01:00U"),
        ok_row_at("C", "25/10/20", "02:00U"),
        ok_row_at("D", "25/10/18", "02:00U"),
    ]
    .join("\n");
    let files = hamlog(csv.as_bytes(), SOTA_C).await.zip();
    assert_eq!(files["other20251020.csv"].lines().count(), 2);
    assert_eq!(files["other20251018.csv"].lines().count(), 2);
    let files = hamlog(csv.as_bytes(), POTA).await.zip();
    assert_eq!(
        files
            .values()
            .map(|b| b.matches("<EOR>").count())
            .sum::<usize>(),
        8
    ); // 4 交信 × (山 + 公園)
}

// 同じ交信(同じ相手・日時・周波数)が2度あっても、取り除かず、2件とも出す。
// 仕様の確認が要る: 重複を警告するか。いまの振る舞いを固定
#[tokio::test]
async fn duplicate_qsos_are_kept() {
    let csv = [ok_row("JA1AAA"), ok_row("JA1AAA")].join("\n");
    let files = hamlog(csv.as_bytes(), SOTA_C).await.zip();
    assert_eq!(files["other20251018.csv"].lines().count(), 2);
    let files = hamlog(csv.as_bytes(), SOTA_A).await.zip();
    assert_eq!(files["sota20251018.csv"].lines().count(), 2);
    assert_eq!(files["sota20251018.adi"].matches("<EOR>").count(), 2);
}

// 未来の日付(2099 年)は誤りにしない。いまの振る舞いを固定
// (仕様の確認が要る: 時計の誤りや打ち間違いを、未来の日付として知らせるか)
#[tokio::test]
async fn future_dates_are_accepted() {
    let got = dates_of(&[("2099/01/01", "01:00U"), ("99/12/31", "23:59U")]).await;
    // 2桁の 99 は 1999 年(65 以上は 19xx)
    assert_eq!(got, vec![ymd("20990101", "0100"), ymd("19991231", "2359")]);
}

// 確認(/logconv/pota)の応答は、画面が使う status・error_log・log_text・file_list だけ。
// ADIF の本文(files)は入れない(画面は使わない。大きなログでは、応答とメモリの大半になる)
#[tokio::test]
async fn confirm_does_not_return_the_adif_files() {
    let csv = [ok_row("JA1AAA"), ok_row("JA1BBB")].join("\r\n");
    let j = confirm(csv.as_bytes(), POTA).await.json();
    assert_eq!(j["status"], "OK");
    assert_eq!(j["log_text"].as_array().unwrap().len(), 2);
    assert!(!j["file_list"].as_array().unwrap().is_empty());
    assert!(j.get("files").is_none(), "{j}");
}
