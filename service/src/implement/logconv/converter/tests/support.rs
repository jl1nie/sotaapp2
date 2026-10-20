//! 担当範囲のテストが共有する小さな道具(HAMLOG の行の組み立てと、ADIF の読み取り)

use super::super::super::adif::parse_adif;
use super::super::super::options::{ConversionOptions, HisQth, MyQth};
use super::super::super::record::QsoRecord;
use std::collections::BTreeMap;

/// 使わない列に置く、読まれてはいけない参照(列の取り違えを見つける)
pub const DECOY: &str = "JA/ZZ-999 JA-9999";

/// HAMLOG CSV の1行(2025-10-18 の UTC、7.030MHz、CW)
pub fn row(call: &str, time: &str, qth: &str, rmks1: &str, rmks2: &str) -> String {
    format!("{call},25/10/18,{time},59,59,7.030,CW,,,N,,{qth},{rmks1},{rmks2},")
}

pub fn records(rows: &[String]) -> Vec<QsoRecord> {
    super::fixture(&rows.join("\n"))
}

pub fn options(my_qth: MyQth, his_qth: HisQth) -> ConversionOptions {
    ConversionOptions {
        my_qth,
        his_qth,
        summit: "JA/TK-001".to_string(),
        pota_activator: "JA1ZZZ/1".to_string(),
        pota_operator: "JA1ZZZ".to_string(),
        ..Default::default()
    }
}

/// ADIF のファイルの記録ごとの項目(タグ → 値)
pub fn adif_records(body: &str) -> Vec<BTreeMap<String, String>> {
    parse_adif(body)
        .into_iter()
        .map(|r| r.into_iter().collect())
        .collect()
}

/// CSV の1行を列に分ける(引用符のない行用)
pub fn cols(line: &str) -> Vec<&str> {
    line.split(',').collect()
}

/// 引用符(`"JA/KN-001 35.6N, 139.2E"`)を含む行も読む(画面と同じ CSV の読み方)
pub fn fixture_quoted(rows: &[String]) -> Vec<QsoRecord> {
    use super::super::super::hamlog::decode_auto;
    let csv = rows.join("\n");
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(csv.as_bytes())
        .records()
        .map(|row| {
            let cols: Vec<String> = row.unwrap().iter().map(String::from).collect();
            decode_auto(&cols, true).unwrap().unwrap()
        })
        .collect()
}

/// HAMLOG CSV の1行(カンマ区切りで、引用符なし)を読む
pub fn fixture_line(line: &str) -> Vec<QsoRecord> {
    super::fixture(line)
}
