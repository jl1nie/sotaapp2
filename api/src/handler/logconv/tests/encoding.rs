//! 文字コード・改行・末尾のバイト(計画の D-1〜5)
//!
//! 入力は `decode_text`(`input.rs`): 先頭の UTF-8 の BOM を外し、UTF-8 として正しければ UTF-8、
//! そうでなければ Shift_JIS(cp932)として読む。日本語版 HAMLOG の CSV は Shift_JIS。
//!
//! フィクスチャは `test_data/logconv/encoding`(`generate.py` で作る。一覧は同じ場所の README)。
//! 名前・QTH・備考に日本語があっても列がずれないことは、相手・自局の参照(ASCII)が正しい列から取れることで確かめる。
//! 日本語そのものが正しく読めたかは、日付の列に日本語を入れた行の誤りの知らせ(その文字がそのまま出る)で確かめる。

use super::support::*;
use axum::http::StatusCode;
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_data/logconv/encoding")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// 確認(`/logconv/pota`)の表(`log_text`)。1行は
/// `[相手, 日付, 時刻, バンド, モード, RST送, RST受, 相手の参照, 自局の参照, 運用局, 運用者]`
fn table(json: &serde_json::Value) -> Vec<Vec<String>> {
    json["log_text"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            r.as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_str().unwrap().to_string())
                .collect()
        })
        .collect()
}

/// 表の、相手のコールサインと参照(相手・自局)
fn calls_and_refs(json: &serde_json::Value) -> Vec<(String, String, String)> {
    table(json)
        .into_iter()
        .map(|r| (r[0].clone(), r[7].clone(), r[8].clone()))
        .collect()
}

fn refs(c: &str, his: &str, my: &str) -> (String, String, String) {
    (c.to_string(), his.to_string(), my.to_string())
}

/// `generate.py` の NAMES と同じ内容の期待(日本語の名前・QTH・備考があっても、参照は正しい列から取れる)
fn names_expected() -> Vec<(String, String, String)> {
    let my = "JA/TK-001/JA-0001";
    vec![
        refs("JA1AAA", "JA/KN-006/JA-0002", my),
        refs("JA1BBB", "JA-0003", my),
        refs("JA1CCC", "JA-0004", my),
        refs("JA1DDD", "JA-0005", my),
    ]
}

// ---- D-1: Shift_JIS(cp932) ----

// 2バイト目が 0x5C の字(表・能・ソ・十・予・﨑)、半角カナ、cp932 の拡張(①・㈱・髙・﨑・№)が、
// 名前・QTH・備考1・備考2のどこにあっても、列がずれない
#[tokio::test]
async fn sjis_japanese_in_any_column_does_not_shift_columns() {
    let r = confirm(&fixture("sjis_names.csv"), POTA).await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    assert_eq!(j["status"], "OK", "{j}");
    assert_eq!(calls_and_refs(&j), names_expected());
    // 表の時刻・バンドなども、日本語の影響を受けない
    for row in table(&j) {
        assert_eq!(&row[1..7], ["20251018", "0100", "40m", "CW", "59", "59"]);
    }
}

// SOTA アクティベーターでも同じ(相手の山 JA/KN-006 は、1行目の QTH にだけある)
#[tokio::test]
async fn sjis_japanese_sota_a_reads_the_summit_columns() {
    let r = hamlog(&fixture("sjis_names.csv"), SOTA_A).await;
    assert_eq!(r.status, StatusCode::OK);
    let files = r.zip();
    assert_eq!(files["sota20251018.csv"].lines().count(), 4);
    assert_eq!(
        files["sota-s2s-20251018.csv"],
        "V2,JA1ZZZ/1,JA/TK-001,18/10/2025,01:00,7MHz,CW,JA1AAA,JA/KN-006, \n"
    );
}

// 日本語そのものが読めていること: 日付の列に入れた日本語が、誤りの知らせにそのまま出る
// (cp932 の拡張・半角カナ・0x5C の字・全角の波ダッシュなど)
#[tokio::test]
async fn sjis_japanese_text_is_decoded_in_error_messages() {
    let want = [
        "2行:日付フォーマット不正:令和7年10月18日①㈱髙﨑№",
        "3行:日付フォーマット不正:表能ソ十予/ｱｲｳ",
    ];
    for name in [
        "sjis_error_text.csv",
        "utf8_error_text.csv",
        "utf8_bom_error_text.csv",
    ] {
        let r = confirm(&fixture(name), POTA).await;
        let j = r.json();
        assert_eq!(j["status"], "NG", "{name}");
        assert_eq!(j["error_log"], serde_json::json!(want), "{name}");

        // SOTA の CSV では、誤りの行が sota000000.csv に集まる(ZIP の中は UTF-8)
        let files = hamlog(&fixture(name), SOTA_A).await.zip();
        assert_eq!(
            files["sota000000.csv"],
            "HamLog format error at Line 2. : 日付フォーマット不正:令和7年10月18日①㈱髙﨑№,,,,,,,,,\n\
             HamLog format error at Line 3. : 日付フォーマット不正:表能ソ十予/ｱｲｳ,,,,,,,,,\n",
            "{name}"
        );
    }
}

// 同じ内容を Shift_JIS・UTF-8・UTF-8(BOM つき)で送ると、確認も変換の出力も同じ
#[tokio::test]
async fn sjis_utf8_and_utf8_bom_give_the_same_output() {
    let base = fixture("sjis_names.csv");
    let want_check = confirm(&base, POTA).await.json();
    let want_zip = hamlog(&base, POTA).await.zip();
    assert!(!want_zip.is_empty());
    for name in ["utf8_names.csv", "utf8_bom_names.csv"] {
        let bytes = fixture(name);
        assert_eq!(confirm(&bytes, POTA).await.json(), want_check, "{name}");
        assert_eq!(hamlog(&bytes, POTA).await.zip(), want_zip, "{name}");
        let (s, u) = (
            hamlog(&base, SOTA_A).await.zip(),
            hamlog(&bytes, SOTA_A).await.zip(),
        );
        assert_eq!(s, u, "{name}: SOTA");
    }
}

// ---- D-2: UTF-8 の BOM ----

// BOM が最初のコールサインに混ざらない(`\u{feff}JA1AAA` にならない)
#[tokio::test]
async fn utf8_bom_does_not_leak_into_the_first_callsign() {
    let j = confirm(&fixture("utf8_bom_names.csv"), POTA).await.json();
    let rows = table(&j);
    assert_eq!(rows[0][0], "JA1AAA");
    assert!(rows.iter().all(|r| !r[0].contains('\u{feff}')));
    // 変換の出力にも混ざらない
    let files = hamlog(&fixture("utf8_bom_names.csv"), SOTA_A).await.zip();
    assert!(files.values().all(|b| !b.contains('\u{feff}')));
    assert!(files["sota20251018.csv"].contains(",JA1AAA,"));
}

// BOM だけのファイル・BOM と改行だけのファイルは、読める交信がなく 400
#[tokio::test]
async fn utf8_bom_only_is_a_bad_request() {
    for bytes in [&b"\xEF\xBB\xBF"[..], b"\xEF\xBB\xBF\r\n"] {
        let r = hamlog(bytes, SOTA_A).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
        assert_well_formed("BOM だけ", &r);
    }
}

// ---- D-3: 判定の境目 ----

// ASCII だけのファイルは、どちらの文字コードでも同じ(UTF-8 として読む)
#[tokio::test]
async fn ascii_only_file_is_read() {
    let j = confirm(&fixture("ascii_only.csv"), POTA).await.json();
    assert_eq!(j["status"], "OK");
    assert_eq!(
        calls_and_refs(&j),
        vec![
            refs("JA1AAA", "JA-0002", "JA-0001"),
            refs("JA1BBB", "JA-0003", "JA-0001")
        ]
    );
}

// Shift_JIS の並びが、偶然 UTF-8 としても正しいファイル(日付の列が Shift_JIS の `縺ｱ` = E3 81 B1)は、
// 「UTF-8 として正しければ UTF-8」の決まりで UTF-8 の `ぱ` と読まれる。
// 仕様の確認が要る: 判定が外れる例(ごく稀。Shift_JIS の半角カナが続く並びなど)。いまの振る舞いを固定する
#[tokio::test]
async fn sjis_that_is_also_valid_utf8_is_read_as_utf8() {
    let j = confirm(&fixture("sjis_utf8_lookalike.csv"), POTA)
        .await
        .json();
    assert_eq!(j["status"], "NG");
    assert_eq!(
        j["error_log"],
        serde_json::json!(["1行:日付フォーマット不正:ぱ"])
    );
    // もう1行は読める
    assert_eq!(table(&j).len(), 2);
}

// 壊れたバイトを含む Shift_JIS: 壊れたバイトのある行も、ほかの行も読める。壊れた字は置き換え文字になる
// (名前の 0xFF、名前の終わりの 2バイト文字の1バイト目、最後の列の 0x81)。日付が壊れた行だけが誤り
#[tokio::test]
async fn broken_bytes_in_sjis_only_affect_their_own_row() {
    let r = confirm(&fixture("sjis_invalid_bytes.csv"), POTA).await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    assert_eq!(j["status"], "NG");
    assert_eq!(
        j["error_log"],
        serde_json::json!(["5行:日付フォーマット不正:25\u{FFFD}10\u{FFFD}18"])
    );
    let rows = table(&j);
    assert_eq!(rows.len(), 5);
    let ok: Vec<_> = rows[..4]
        .iter()
        .map(|r| (r[0].clone(), r[7].clone(), r[8].clone()))
        .collect();
    assert_eq!(
        ok,
        vec![
            refs("JA1AAA", "JA-0002", "JA-0001"),
            refs("JA1BBB", "JA-0003", "JA-0001"),
            refs("JA1CCC", "JA-0004", "JA-0001"),
            refs("JA1DDD", "JA-0005", "JA-0001"),
        ]
    );

    // SOTA: 読めた4行はそのまま変換され、壊れた行だけが sota000000.csv に入る
    let files = hamlog(&fixture("sjis_invalid_bytes.csv"), SOTA_C)
        .await
        .zip();
    assert_eq!(files["other20251018.csv"].lines().count(), 4);
    assert_eq!(files["sota000000.csv"].lines().count(), 1);
    assert!(files["sota000000.csv"].starts_with("HamLog format error at Line 5. :"));
}

// UTF-8 の日本語に壊れたバイトが1つでもあると、ファイル全体が Shift_JIS として読まれる(日本語は化ける)。
// 仕様の確認が要る: 行ごとに判定する/壊れたバイトを知らせる、などにするか。
// いまは、ASCII の列(コールサイン・参照)は無事なので、交信は読める
#[tokio::test]
async fn utf8_with_one_broken_byte_is_read_as_sjis_but_columns_survive() {
    let j = confirm(&fixture("utf8_invalid_byte.csv"), POTA)
        .await
        .json();
    assert_eq!(j["status"], "OK", "{j}");
    assert_eq!(
        calls_and_refs(&j),
        vec![
            refs("JA1AAA", "JA-0002", "JA-0001"),
            refs("JA1BBB", "JA-0003", "JA-0001"),
            refs("JA1CCC", "JA-0004", "JA-0001"),
        ]
    );
}

// ---- D-4: UTF-16 ----
// 仕様の確認が要る。いまの振る舞い: BOM つきの UTF-16(LE・BE)は読める(Shift_JIS の復号が BOM を見て UTF-16 にする)。
// BOM のない UTF-16 は読めず、全行が誤りになる。どちらも 500 やパニックにはならない

#[tokio::test]
async fn utf16_with_bom_is_read_like_utf8() {
    let want = confirm(&fixture("utf8_names.csv"), POTA).await.json();
    for name in ["utf16le_bom_names.csv", "utf16be_bom_names.csv"] {
        let bytes = fixture(name);
        assert_eq!(confirm(&bytes, POTA).await.json(), want, "{name}");
        assert_eq!(
            hamlog(&bytes, POTA).await.zip(),
            hamlog(&fixture("utf8_names.csv"), POTA).await.zip(),
            "{name}"
        );
    }
}

#[tokio::test]
async fn utf16_without_bom_is_all_errors_but_not_a_crash() {
    let bytes = fixture("utf16le_nobom_names.csv");
    // 確認: 200 で、NG と誤りの一覧
    let c = confirm(&bytes, POTA).await;
    assert_eq!(c.status, StatusCode::OK);
    assert_eq!(c.json()["status"], "NG");
    assert!(!c.json()["error_log"].as_array().unwrap().is_empty());
    // POTA の生成: 400(ADIF は作らない)
    let p = hamlog(&bytes, POTA).await;
    assert_eq!(p.status, StatusCode::BAD_REQUEST);
    assert_well_formed("UTF-16(BOMなし)POTA", &p);
    // SOTA: 200 の ZIP で、中身は誤りの行(sota000000.csv)だけ
    let s = hamlog(&bytes, SOTA_A).await;
    assert_eq!(s.status, StatusCode::OK);
    let files = s.zip();
    assert_eq!(files.keys().collect::<Vec<_>>(), ["sota000000.csv"]);
}

// ---- D-5: 改行と終わり ----

// 改行が CRLF・LF・CR だけ・混在・最後の行に改行がない、のどれでも、同じ交信を読む
#[tokio::test]
async fn every_newline_style_reads_the_same_records() {
    let cases = [
        ("newline_crlf.csv", 2),
        ("newline_lf.csv", 2),
        ("newline_cr.csv", 2),
        ("newline_none_at_end.csv", 2),
        ("newline_mixed.csv", 3),
    ];
    for (name, n) in cases {
        let j = confirm(&fixture(name), POTA).await.json();
        assert_eq!(j["status"], "OK", "{name}: {j}");
        let rows = calls_and_refs(&j);
        assert_eq!(rows.len(), n, "{name}");
        assert_eq!(rows[0], refs("JA1AAA", "JA-0002", "JA-0001"), "{name}");
        assert_eq!(rows[1].0, "JA1BBB", "{name}");
        // 変換でも同じ数
        let files = hamlog(&fixture(name), SOTA_C).await.zip();
        assert_eq!(files["other20251018.csv"].lines().count(), n, "{name}");
    }
}

// 末尾の 0x1A(古い DOS の EOF)が、最後の行の後ろにあっても、行の途中にあっても、交信を壊さない。
// 最後の行の改行が無く、0x1A が最後の項目に付くときは、読める(交信の中身に影響しない)
#[tokio::test]
async fn sub_eof_at_the_end_of_the_last_line_is_harmless() {
    let j = confirm(&fixture("sub_eof_no_newline.csv"), POTA)
        .await
        .json();
    assert_eq!(j["status"], "OK", "{j}");
    assert_eq!(table(&j).len(), 2);
}

// 0x1A が最後の改行の後ろに単独であると、その行が「HAMLOG CSV形式ではありません」の誤りになり、
// POTA の ADIF が作れなくなる(古い DOS 時代のファイルの終わりの印。無視すべき)。
#[tokio::test]
#[ignore = "不具合: 末尾の 0x1A(改行の後ろ)が誤りの行になり、POTA の ADIF を作れない"]
async fn sub_eof_after_the_last_newline_is_ignored() {
    let bytes = fixture("sub_eof_after_newline.csv");
    let j = confirm(&bytes, POTA).await.json();
    assert_eq!(j["status"], "OK", "{j}");
    assert_eq!(table(&j).len(), 2);
    assert_eq!(hamlog(&bytes, POTA).await.status, StatusCode::OK);
    // SOTA でも、誤りの行(sota000000.csv)ができない
    let files = hamlog(&bytes, SOTA_C).await.zip();
    assert!(!files.contains_key("sota000000.csv"), "{files:?}");
}
