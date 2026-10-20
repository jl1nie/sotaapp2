//! e2e: FLE Online が出した HAMLOG の CSV(Shift_JIS・CRLF)を、logconv の画面と同じ指定で変換し、
//! FLE が自分で出した ADIF・SOTA の CSV と、交信ごとに同じになるかを確かめる
//!
//! フィクスチャは `test_data/logconv/fle`。`名前.fle` が FLE のログ、`名前/` が FLE Online の ZIP の中身。
//! 作り直すときは、sotalive-monorepo で
//! `cargo run -p fle-core --example logconv_fixtures -- <このリポジトリ>/test_data/logconv/fle`。
//!
//! FLE の HAMLOG の CSV は、自局の参照(QSO ごとの運用先)を備考1に、相手の参照を QTH の列に書く。
//! 画面では、運用先を「Remarks1で指定する」か入力(`form`)にし、相手を「QTHで指定する」にする。どちらでも確かめる
//! (途中で運用先を変えたログは、Remarks1 でなければ分けられない)。

use super::*;
use service::implement::logconv::parse_adif;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fle_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_data/logconv/fle")
}

/// FLE Online の出力(ファイル名 → 中身)。HAMLOG の CSV は、そのままのバイト列で別に返す
struct FleOutput {
    hamlog: Vec<u8>,
    files: BTreeMap<String, String>,
}

fn fle_output(case: &str) -> FleOutput {
    let mut hamlog = None;
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(fle_dir().join(case)).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let bytes = std::fs::read(entry.path()).unwrap();
        if name.starts_with("hamlog-") {
            hamlog = Some(bytes);
        } else if !name.starts_with("airham-") {
            files.insert(
                name,
                String::from_utf8(bytes).unwrap().replace("\r\n", "\n"),
            );
        }
    }
    let hamlog = hamlog.expect("FLE の HAMLOG の CSV");
    // 日本語版の HAMLOG と同じく Shift_JIS(UTF-8 ではない)。読み込みの文字コードの判定を通る
    assert!(std::str::from_utf8(&hamlog).is_err(), "{case}: Shift_JIS");
    FleOutput { hamlog, files }
}

/// logconv の画面と同じ指定で変換する(`/logconv/hamlog`)。ZIP の中身
async fn convert(hamlog: &[u8], options: &str) -> BTreeMap<String, String> {
    let res = hamlog_convert(None, multipart_of_bytes(hamlog, options).await).await;
    let status = res.status();
    let is_zip = res
        .headers()
        .get(header::CONTENT_TYPE)
        .is_some_and(|t| t == "application/zip");
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    // 変換できないときは、理由(JSON)を出す
    assert!(
        status == StatusCode::OK && is_zip,
        "{options}: HTTP {status} {}",
        String::from_utf8_lossy(&bytes)
    );
    unzip(&bytes)
}

/// POTA の画面の「CSV読込」(`/logconv/pota`)。誤りがなく、全交信が表に出ること
async fn check_reads_every_qso(hamlog: &[u8], options: &str, qsos: usize) {
    let mut options: serde_json::Value = serde_json::from_str(options).unwrap();
    options["command"] = "ADIFCSVCheck".into();
    let res = pota_check(None, multipart_of_bytes(hamlog, &options.to_string()).await).await;
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let check: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(check["status"], "OK", "{check}");
    assert_eq!(check["log_text"].as_array().unwrap().len(), qsos, "{check}");
}

/// ADIF の記録を、比べる項目だけにする(並びは問わない)
///
/// RST・グリッドは比べない(logconv の POTA の記録は RST を持たない。FLE の HAMLOG はグリッドを書かない)。
fn adif_qsos(body: &str) -> Vec<BTreeMap<String, String>> {
    const FIELDS: [&str; 11] = [
        "STATION_CALLSIGN",
        "OPERATOR",
        "CALL",
        "QSO_DATE",
        "TIME_ON",
        "BAND",
        "MODE",
        "MY_SIG",
        "MY_SIG_INFO",
        "SIG",
        "SIG_INFO",
    ];
    let mut qsos: Vec<BTreeMap<String, String>> = parse_adif(body)
        .into_iter()
        .map(|record| {
            FIELDS
                .iter()
                .filter_map(|&k| Some((k.to_string(), record.get(k)?.to_uppercase())))
                .collect()
        })
        .collect();
    qsos.sort();
    qsos
}

/// SOTA の CSV の行(`V2,自局,自局の山,日付,時刻,バンド,モード,相手,相手の山,場所の注記`)を、比べられる形にする
///
/// - CSV として読み、どの行も10列であること(場所の注記 `%QTH%35.6N, 139.2E%` のカンマで、列がずれない)。
/// - 日付の年は4桁にそろえる(FLE は `18/10/25`、logconv は旧版と同じ `18/10/2025`。SOTA はどちらも読む)。
/// - 場所の注記は、前後の空白を外す(logconv は旧版と同じく、末尾の空白が2つ)。
fn sota_rows(body: &str) -> Vec<String> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(body.as_bytes())
        .records()
        .map(|row| {
            let row = row.unwrap();
            assert_eq!(row.len(), 10, "SOTA の CSV は10列: {row:?}");
            let mut f: Vec<String> = row.iter().map(str::to_string).collect();
            f[9] = f[9].trim().to_string();
            if let [d, m, y] = f[3].split('/').collect::<Vec<_>>()[..] {
                if y.len() == 2 {
                    f[3] = format!("{d}/{m}/20{y}");
                }
            }
            f.join("|")
        })
        .collect()
}

/// 名前が条件に合うファイル(名前 → 中身)
fn pick(files: &BTreeMap<String, String>, keep: impl Fn(&str) -> bool) -> BTreeMap<&str, &str> {
    files
        .iter()
        .filter(|(name, _)| keep(name))
        .map(|(name, body)| (name.as_str(), body.as_str()))
        .collect()
}

fn is_sota_csv(name: &str) -> bool {
    name.starts_with("sota") && name.ends_with(".csv")
}

/// SOTA の CSV(`sotaYYYYMMDD.csv` と `sota-s2s-YYYYMMDD.csv`)が、FLE と同じ
fn assert_same_sota_csv(
    case: &str,
    fle: &BTreeMap<String, String>,
    got: &BTreeMap<String, String>,
) {
    let want = pick(fle, is_sota_csv);
    let got = pick(got, is_sota_csv);
    assert_eq!(
        got.keys().collect::<Vec<_>>(),
        want.keys().collect::<Vec<_>>(),
        "{case}: SOTA の CSV のファイル"
    );
    for (name, body) in &want {
        assert_eq!(sota_rows(got[name]), sota_rows(body), "{case}/{name}");
    }
}

/// S2S の交信(相手の山のある行)の、(相手, 時刻, 相手の山)。SOTA の CSV から
fn s2s_in_csv(fle: &BTreeMap<String, String>) -> Vec<(String, String, String)> {
    let mut s2s: Vec<_> = pick(fle, |n| n.starts_with("sota-s2s-"))
        .values()
        .flat_map(|body| body.lines())
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let hhmm = f[4].replace(':', "");
            (f[7].to_string(), hhmm, f[8].to_string())
        })
        .collect();
    s2s.sort();
    s2s
}

/// S2S の交信の、(相手, 時刻, 相手の山)。SOTA の ADIF(`SOTA_REF`)から
fn s2s_in_adif(body: &str) -> Vec<(String, String, String)> {
    let mut s2s: Vec<_> = parse_adif(body)
        .into_iter()
        .filter_map(|r| {
            Some((
                r.get("CALL")?.clone(),
                r.get("TIME_ON")?.clone(),
                r.get("SOTA_REF")?.clone(),
            ))
        })
        .collect();
    s2s.sort();
    s2s
}

/// POTA の画面の指定(運用先は Remarks1 か、入力した参照)
fn pota_options(park: Option<&str>) -> String {
    let my = match park {
        Some(park) => format!(r#""myQTH":"form","Park":"{park}""#),
        None => r#""myQTH":"rmks1""#.to_string(),
    };
    format!(
        r#"{{"format":"pota",{my},"QTH":"qth","POTAActivator":"JA1ZZZ/1","POTAOperator":"JA1ZZZ"}}"#
    )
}

/// SOTA Activator の画面の指定(運用した山は Remarks1 か、入力した山)
fn sota_options(summit: Option<&str>) -> String {
    let my = match summit {
        Some(summit) => format!(r#""myQTH":"form","Summit":"{summit}""#),
        None => r#""myQTH":"rmks1""#.to_string(),
    };
    format!(r#"{{"format":"sota_a",{my},"QTH":"qth","SOTAActivator":"JA1ZZZ/1"}}"#)
}

/// 公園・WWFF の ADIF(自局の参照ごとのファイル)が、FLE と同じ
fn assert_same_park_adif(
    case: &str,
    fle: &BTreeMap<String, String>,
    got: &BTreeMap<String, String>,
) {
    let is_park_adif = |n: &str| n.ends_with(".adi") && !n.contains("@JA-TK-");
    let want = pick(fle, is_park_adif);
    let got = pick(got, is_park_adif);
    assert_eq!(
        got.keys().collect::<Vec<_>>(),
        want.keys().collect::<Vec<_>>(),
        "{case}: 自局の参照ごとのファイル"
    );
    for (name, body) in &want {
        assert_eq!(adif_qsos(got[name]), adif_qsos(body), "{case}/{name}");
    }
}

// 自局 2-fer(+WWFF)× 相手 0〜3-fer: 自局の公園ごとの ADIF に、相手の公園ごとの記録。FLE の ADIF と同じ
#[tokio::test]
async fn fle_pota_nfer_converts_to_the_same_adif_as_fle() {
    let fle = fle_output("pota_nfer");
    for options in [
        pota_options(None),
        pota_options(Some("JA-0001 JA-0002 JAFF-0001")),
    ] {
        check_reads_every_qso(&fle.hamlog, &options, 14).await;
        assert_same_park_adif(&options, &fle.files, &convert(&fle.hamlog, &options).await);
    }
    let got = convert(&fle.hamlog, &pota_options(None)).await;
    // 相手 3-fer(JA1DDD)は、自局の公園ごとに3件。全バンド(135kHz〜10GHz)の交信が入る
    let ja0001 = &got["JA1ZZZ-1@JA-0001-20251018.adi"];
    assert_eq!(
        adif_qsos(ja0001)
            .iter()
            .filter(|q| q["CALL"] == "JA1DDD")
            .count(),
        3
    );
    let bands: std::collections::BTreeSet<_> = adif_qsos(ja0001)
        .into_iter()
        .map(|q| q["BAND"].clone())
        .collect();
    assert_eq!(bands.len(), 8, "{bands:?}");
}

// SOTA: S2S は、相手の山を QTH の列から読む(画面の「QTHで指定する」)。JST で書いて UTC の日付をまたぐログ
#[tokio::test]
async fn fle_sota_s2s_converts_to_the_same_csv_as_fle() {
    let fle = fle_output("sota_s2s");
    for options in [sota_options(None), sota_options(Some("JA/TK-001"))] {
        assert_same_sota_csv(&options, &fle.files, &convert(&fle.hamlog, &options).await);
    }
    let options = sota_options(Some("JA/TK-001"));
    let got = convert(&fle.hamlog, &options).await;
    // 備考の座標は、場所の注記になる(FLE の HAMLOG の CSV が、座標のカンマを残す)
    let row = sota_rows(&got["sota20251018.csv"])
        .into_iter()
        .find(|r| r.contains("|JA1DDD|"))
        .unwrap();
    assert!(row.ends_with("|%QTH%35.6N, 139.2E%"), "{row}");
    // ADIF にも、同じ S2S(SOTA_REF)。ファイルは1つ(最初の交信の UTC の日付)
    let adif = pick(&got, |n| n.ends_with(".adi"));
    assert_eq!(
        adif.keys().copied().collect::<Vec<_>>(),
        ["sota20251018.adi"]
    );
    assert_eq!(
        s2s_in_adif(adif["sota20251018.adi"]),
        s2s_in_csv(&fle.files)
    );
    assert_eq!(s2s_in_csv(&fle.files).len(), 4);

    // 相手を「指定しない」と、S2S はなくなる(交信は、すべて残る)
    let none = convert(
        &fle.hamlog,
        &options.replace(r#""QTH":"qth""#, r#""QTH":"none""#),
    )
    .await;
    assert!(pick(&none, |n| n.starts_with("sota-s2s-")).is_empty());
    assert!(s2s_in_adif(&none["sota20251018.adi"]).is_empty());
    let rows: usize = pick(&none, is_sota_csv)
        .values()
        .map(|b| b.lines().count())
        .sum();
    assert_eq!(rows, 8);
}

// SOTA と POTA の同時運用: POTA の画面に山と公園を入れると、公園の ADIF は FLE と同じ。山の ADIF に S2S
#[tokio::test]
async fn fle_sota_and_pota_together() {
    let fle = fle_output("sota_pota");
    for pota in [pota_options(None), pota_options(Some("JA/TK-001 JA-0001"))] {
        check_reads_every_qso(&fle.hamlog, &pota, 5).await;
        let got = convert(&fle.hamlog, &pota).await;
        assert_same_park_adif(&pota, &fle.files, &got);
        // 山の ADIF: 全交信(5件)と、S2S(JA1BBB・JA1DDD)。S2S+P2P の JA1DDD も1件(山の記録は、相手の公園で分けない)
        let summit = &got["JA1ZZZ-1@JA-TK-001-20251018.adi"];
        assert_eq!(parse_adif(summit).len(), 5, "{pota}");
        assert_eq!(s2s_in_adif(summit), s2s_in_csv(&fle.files), "{pota}");
    }
    for sota in [sota_options(None), sota_options(Some("JA/TK-001"))] {
        assert_same_sota_csv(&sota, &fle.files, &convert(&fle.hamlog, &sota).await);
    }
}

// SOTA チェイサー: 相手の山(QTH の列)のある交信が `sota…`、ほかが `other…`。山の行は、FLE の SOTA の CSV と同じ
#[tokio::test]
async fn fle_chaser_splits_summit_contacts() {
    let fle = fle_output("chaser");
    let got = convert(
        &fle.hamlog,
        r#"{"format":"sota_c","QTH":"qth","callsign":"JA1ZZZ"}"#,
    )
    .await;
    assert_eq!(
        got.keys().map(String::as_str).collect::<Vec<_>>(),
        ["other20251018.csv", "sota20251018.csv"]
    );
    // FLE は、自局の山・公園のないログを、全交信の SOTA の CSV にする。そのうち相手の山のある行
    let fle_rows = sota_rows(&fle.files["sota20251018.csv"]);
    let (summits, others): (Vec<_>, Vec<_>) = fle_rows
        .into_iter()
        .partition(|r| !r.split('|').nth(8).unwrap().is_empty());
    assert_eq!(sota_rows(&got["sota20251018.csv"]), summits);
    assert_eq!(sota_rows(&got["other20251018.csv"]), others);
    assert_eq!(summits.len(), 3);
}

// 途中で運用先(山・公園)を変えたログ: 「Remarks1で指定する」なら、QSO ごとの運用先で分かれ、FLE と同じ
#[tokio::test]
async fn fle_moving_between_summits_and_parks() {
    let fle = fle_output("pota_move");
    let pota = pota_options(None);
    check_reads_every_qso(&fle.hamlog, &pota, 5).await;
    let got = convert(&fle.hamlog, &pota).await;
    assert_same_park_adif("pota_move", &fle.files, &got);
    // 山の ADIF も、山ごと(JA/TK-001 に2件、JA/TK-002 に2件。山のない JA1EEE は入らない)
    for (summit, calls) in [
        ("JA-TK-001", ["JA1AAA", "JA1BBB"]),
        ("JA-TK-002", ["JA1CCC", "JA1DDD"]),
    ] {
        let body = &got[&format!("JA1ZZZ-1@{summit}-20251018.adi")];
        let got_calls: Vec<String> = parse_adif(body)
            .into_iter()
            .map(|r| r["CALL"].clone())
            .collect();
        assert_eq!(got_calls, calls, "{summit}");
    }

    assert_same_sota_csv(
        "pota_move",
        &fle.files,
        &convert(&fle.hamlog, &sota_options(None)).await,
    );
}
