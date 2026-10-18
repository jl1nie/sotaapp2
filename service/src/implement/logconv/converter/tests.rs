use super::super::options::{ConversionOptions, HisQth, MyQth};
use super::super::record::QsoRecord;
use super::*;
use std::collections::HashMap;

fn make_qso() -> QsoRecord {
    QsoRecord {
        callsign: "JL1NIE".to_string(),
        year: 2024,
        month: 12,
        day: 28,
        hour: 12,
        minute: 34,
        rst_sent: "599".to_string(),
        rst_rcvd: "559".to_string(),
        band_sota: "7MHz".to_string(),
        band_wlen: "40m".to_string(),
        mode: "CW".to_string(),
        mode_sota: "CW".to_string(),
        remarks1: "JA/TK-001".to_string(),
        qth: "JA/KN-001".to_string(),
        ..Default::default()
    }
}

#[test]
fn test_to_sota_activator() {
    let options = ConversionOptions {
        my_qth: MyQth::Rmks1,
        his_qth: HisQth::Qth,
        ..Default::default()
    };

    let files = convert_to_sota_activator(&[make_qso()], "JL1NIE", &options);
    // 相手の QTH(JA/KN-001)にも山があるので、S2S の CSV にも入る
    let line = &files["sota20241228.csv"];
    assert_eq!(line, &files["sota-s2s-20241228.csv"]);
    let cols: Vec<&str> = line.trim_end().split(',').collect();
    assert_eq!(cols[0], "V2");
    assert_eq!(cols[2], "JA/TK-001");
    assert_eq!(cols[8], "JA/KN-001");
}

/// 日付が行き来するログや、誤りの行が間にあるログでも、交信が消えない
#[test]
fn test_convert_to_sota_activator_keeps_every_contact() {
    use super::super::hamlog::decode_hamlog;
    // コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,
    let rows = [
        "JA2AAA,25/10/19,00:30J,59,59,7.030,CW,,,N,,JA/KN-001,JA/TK-001,,", // 前日(UTC)・S2S
        "JA2BBB,25/10/19,09:15J,59,59,14.285,SSB,,,N,,,JA/TK-001,,",        // 翌日(UTC)
        "JA2CCC,25/10/19,99:99J,59,59,14.285,SSB,,,N,,,JA/TK-001,,",        // 時刻の誤り
        "JA2DDD,25/10/19,10:00J,59,59,9.999,SSB,,,N,,,JA/TK-001,,",         // 周波数の誤り
        "JA2EEE,25/10/19,10:10J,59,59,7.030,CW,,,N,,,JA/TK-001,,",          // 翌日(UTC)
        "JA2FFF,25/10/19,00:40J,59,59,7.030,CW,,,N,,,JA/TK-001,,",          // 前日に戻る
    ];
    let records: Vec<QsoRecord> = rows
        .iter()
        .map(|r| {
            let cols: Vec<String> = r.split(',').map(String::from).collect();
            decode_hamlog(&cols).unwrap()
        })
        .collect();
    let options = ConversionOptions {
        my_qth: MyQth::Rmks1,
        his_qth: HisQth::Qth,
        ..Default::default()
    };

    let files = convert_to_sota_activator(&records, "JA1ZZZ/1", &options);

    let day18 = &files["sota20251018.csv"];
    assert!(day18.contains(",JA2AAA,") && day18.contains(",JA2FFF,"));
    let day19 = &files["sota20251019.csv"];
    assert!(day19.contains(",JA2BBB,") && day19.contains(",JA2EEE,"));
    // 誤りの行は、消えずに、日付のファイルとは別の sota000000.csv に集まる
    let errors = &files["sota000000.csv"];
    assert!(errors.contains("周波数不正") && errors.contains("日時不正"));
    assert!(!day19.contains("HamLog format error"));
    assert_eq!(files["sota-s2s-20251018.csv"].lines().count(), 1);
}

#[test]
fn test_convert_to_sota_chaser_splits_sota_and_other_contacts() {
    let options = ConversionOptions {
        his_qth: HisQth::Qth,
        ..Default::default()
    };
    let mut with_summit = make_qso();
    with_summit.qth = "JA/KN-001".to_string();
    let mut without = make_qso();
    without.qth = "Tokyo".to_string();

    let files = convert_to_sota_chaser(&[with_summit, without], "JA1ZZZ", &options);
    assert_eq!(files.len(), 2);
    let sota = &files["sota20241228.csv"];
    assert_eq!(sota.lines().count(), 1);
    assert!(sota.starts_with("V2,JA1ZZZ,,"));
    assert!(sota.contains(",JL1NIE,JA/KN-001,"));
    // 山のない交信は、other のファイルに入る
    assert_eq!(files["other20241228.csv"].lines().count(), 1);
}

#[test]
fn test_convert_to_sota_chaser_without_sota_contacts_has_only_other() {
    let options = ConversionOptions {
        his_qth: HisQth::Qth,
        ..Default::default()
    };
    let mut qso = make_qso();
    qso.qth = String::new();
    let files = convert_to_sota_chaser(&[qso], "JA1ZZZ", &options);
    assert_eq!(files.keys().collect::<Vec<_>>(), vec!["other20241228.csv"]);
}

#[test]
fn test_convert_to_sota_chaser_from_hamlog_rows() {
    use super::super::hamlog::decode_hamlog;
    // コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,
    let rows = [
        "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,,,",
        "JA1BBB,25/10/18,01:05U,59,59,7.030,CW,,,N,,JA/KN-001,,,",
        "JA1CCC,25/10/18,01:10U,59,59,14.285,SSB,,,N,,JA-0123,,,",
        "JA1FFF,25/10/18,01:25U,59,59,21.300,SSB,,,N,,JA/KN-001 JA-0200,,,",
        "JA1GGG,25/10/19,00:30U,59,59,21.300,SSB,,,N,,JA/TK-005,,,",
    ];
    let records: Vec<QsoRecord> = rows
        .iter()
        .map(|r| {
            let cols: Vec<String> = r.split(',').map(String::from).collect();
            decode_hamlog(&cols).unwrap()
        })
        .collect();
    let options = ConversionOptions {
        his_qth: HisQth::Qth,
        ..Default::default()
    };

    let files = convert_to_sota_chaser(&records, "JA1ZZZ/1", &options);
    let mut names: Vec<&String> = files.keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec!["other20251018.csv", "sota20251018.csv", "sota20251019.csv"]
    );
    assert_eq!(
        files["sota20251018.csv"],
        "V2,JA1ZZZ/1,,18/10/2025,01:05,7MHz,CW,JA1BBB,JA/KN-001, \n\
         V2,JA1ZZZ/1,,18/10/2025,01:25,21MHz,SSB,JA1FFF,JA/KN-001, \n"
    );
    assert_eq!(
        files["sota20251019.csv"],
        "V2,JA1ZZZ/1,,19/10/2025,00:30,21MHz,SSB,JA1GGG,JA/TK-005, \n"
    );
    // 山のない交信(JA1AAA と、公園だけの JA1CCC)は other に
    assert_eq!(files["other20251018.csv"].lines().count(), 2);
}

/// フィクスチャ(test_data/logconv)の HAMLOG CSV を読む
fn fixture(csv: &str) -> Vec<QsoRecord> {
    use super::super::hamlog::decode_hamlog;
    csv.lines()
        .map(|line| {
            let cols: Vec<String> = line.split(',').map(String::from).collect();
            decode_hamlog(&cols).unwrap()
        })
        .collect()
}

const NFER: &str = include_str!("../../../../../test_data/logconv/hamlog_nfer.csv");
const NFER_RMKS2: &str = include_str!("../../../../../test_data/logconv/hamlog_nfer_rmks2.csv");
const JST_ERRORS: &str = include_str!("../../../../../test_data/logconv/hamlog_jst_errors.csv");

fn rmks1_qth() -> ConversionOptions {
    ConversionOptions {
        my_qth: MyQth::Rmks1,
        his_qth: HisQth::Qth,
        pota_activator: "JA1ZZZ/1".to_string(),
        pota_operator: "JA1ZZZ".to_string(),
        ..Default::default()
    }
}

fn records_in(adif: &str) -> usize {
    adif.matches("<EOR>").count()
}

#[test]
fn fixture_nfer_sota_activator_splits_s2s() {
    let files = convert_to_sota_activator(&fixture(NFER), "JA1ZZZ/1", &rmks1_qth());
    assert_eq!(files["sota20251018.csv"].lines().count(), 10);
    let s2s = &files["sota-s2s-20251018.csv"];
    assert_eq!(s2s.lines().count(), 2);
    assert!(s2s.contains(",JA1BBB,JA/KN-001,") && s2s.contains(",JA1FFF,JA/KN-001,"));
}

#[test]
fn fixture_nfer_sota_activator_reads_his_ref_from_remarks2() {
    let options = ConversionOptions {
        his_qth: HisQth::Rmks2,
        ..rmks1_qth()
    };
    let files = convert_to_sota_activator(&fixture(NFER_RMKS2), "JA1ZZZ/1", &options);
    assert_eq!(files["sota20251018.csv"].lines().count(), 10);
    assert_eq!(files["sota-s2s-20251018.csv"].lines().count(), 2);
}

#[test]
fn fixture_nfer_sota_chaser_splits_summit_contacts_from_the_others() {
    let files = convert_to_sota_chaser(&fixture(NFER), "JA1ZZZ", &rmks1_qth());
    assert_eq!(files.len(), 2);
    let csv = &files["sota20251018.csv"];
    assert_eq!(csv.lines().count(), 2);
    assert!(csv.contains(",JA1BBB,JA/KN-001,") && csv.contains(",JA1FFF,JA/KN-001,"));
    assert_eq!(files["other20251018.csv"].lines().count(), 8);
}

#[test]
fn fixture_nfer_adif_has_one_file_per_own_reference() {
    let result = convert_to_adif(&fixture(NFER), &rmks1_qth());
    let mut names: Vec<&String> = result.files.keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "JA1ZZZ-1@JA-0001-20251018.adi",
            "JA1ZZZ-1@JA-0002-20251018.adi",
            "JA1ZZZ-1@JA-0003-20251018.adi",
            "JA1ZZZ-1@JA-TK-001-20251018.adi",
            "JA1ZZZ-1@JAFF-0001-20251018.adi",
        ]
    );
    // 相手が複数の公園のときは、公園ごとに別の記録(JA-0001 は、10交信に、相手 2-fer ×2 と 3-fer の追加分)
    let count = |name: &str| records_in(&result.files[name]);
    assert_eq!(count("JA1ZZZ-1@JA-0001-20251018.adi"), 14);
    assert_eq!(count("JA1ZZZ-1@JA-TK-001-20251018.adi"), 10);
    assert_eq!(count("JA1ZZZ-1@JA-0002-20251018.adi"), 7);
    assert_eq!(count("JA1ZZZ-1@JA-0003-20251018.adi"), 3);
    assert_eq!(count("JA1ZZZ-1@JAFF-0001-20251018.adi"), 6);
}

#[test]
fn fixture_nfer_adif_has_the_his_references() {
    let result = convert_to_adif(&fixture(NFER), &rmks1_qth());
    let park = &result.files["JA1ZZZ-1@JA-0001-20251018.adi"];
    // P2P(相手 JA-0123)・相手2-fer・W2W(WWFF のファイルに入る)
    assert!(park.contains("<SIG_INFO:7>JA-0123"));
    assert!(park.contains("<SIG_INFO:7>JA-0124"));
    let wwff = &result.files["JA1ZZZ-1@JAFF-0001-20251018.adi"];
    assert!(wwff.contains("<SIG:4>WWFF<SIG_INFO:9>JAFF-0456"));
    // S2S は SOTA のファイルに SOTA_REF で入る
    let sota = &result.files["JA1ZZZ-1@JA-TK-001-20251018.adi"];
    assert!(sota.contains("<MY_SOTA_REF:9>JA/TK-001<SOTA_REF:9>JA/KN-001"));
}

#[test]
fn fixture_jst_errors_keep_every_contact_and_report_errors() {
    let records = fixture(JST_ERRORS);
    let files = convert_to_sota_activator(&records, "JA1ZZZ/1", &rmks1_qth());
    // JST 00:30 は前日(UTC)、09:15 は当日。日付が行き来しても、消えない
    assert!(files["sota20251018.csv"].contains(",JA2AAA,"));
    assert!(files["sota20251019.csv"].contains(",JA2BBB,"));
    // 参照のない行は出ない
    assert!(!files.values().any(|v| v.contains("JA2EEE")));
    // 誤りの行は、理由つきで残る
    let all: String = files.values().cloned().collect();
    assert!(all.contains("日時不正:25/10/19 99:99J"));
    assert!(all.contains("周波数不正"));

    // ADIF は、誤りがあるとエラーを返す(何行目かが分かる)
    let result = convert_to_adif(&records, &rmks1_qth());
    assert_eq!(result.status, "NG");
    assert_eq!(result.error_log.len(), 2);
    assert!(result.error_log[0].starts_with("3行:"));
    assert!(result.error_log[1].starts_with("4行:"));
}

// ---- 旧版(HamLogConv)の出力との一致(test_data/logconv/legacy) ----

macro_rules! legacy {
    ($path:literal) => {
        include_str!(concat!("../../../../../test_data/logconv/legacy/", $path))
    };
}

#[test]
fn legacy_parity_sota_activator() {
    let options = ConversionOptions {
        my_qth: MyQth::Rmks1,
        his_qth: HisQth::Rmks2,
        ..rmks1_qth()
    };
    let files = convert_to_sota_activator(&fixture(NFER_RMKS2), "JA1ZZZ/1", &options);
    let mut names: Vec<&String> = files.keys().collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "sota-s2s-20251018.csv",
            "sota20251018.adi",
            "sota20251018.csv"
        ]
    );
    assert_eq!(
        files["sota20251018.csv"],
        legacy!("sota_a/sota20251018.csv")
    );
    assert_eq!(
        files["sota-s2s-20251018.csv"],
        legacy!("sota_a/sota-s2s-20251018.csv")
    );
    assert_eq!(
        files["sota20251018.adi"],
        legacy!("sota_a/sota20251018.adi")
    );
}

#[test]
fn adif_has_my_state_for_parks_with_a_known_location() {
    let mut options = rmks1_qth();
    options
        .park_states
        .insert("JA-0001".to_string(), "TK".to_string());
    let result = convert_to_adif(&fixture(NFER), &options);
    let park = &result.files["JA1ZZZ-1@JA-0001-20251018.adi"];
    // MY_SIG_INFO の直後。相手の SIG の前
    assert!(park.contains("<MY_SIG_INFO:7>JA-0001<MY_STATE:2>TK<SIG:4>POTA<SIG_INFO:7>JA-0123"));
    assert_eq!(records_in(park), park.matches("<MY_STATE:2>TK").count());
    // 所在地が分からない公園(JA-0002)には、入れない
    assert!(!result.files["JA1ZZZ-1@JA-0002-20251018.adi"].contains("MY_STATE"));
    // 表示の行にも所在地
    assert!(result.log_text[0][8].contains("JA-0001(TK)"));
}

/// HamLog iOS 形式(1行目が見出し)。旧版と同じ出力になる(誤りの行のファイル名を除く)
#[test]
fn sota_adif_has_no_lines_for_rows_with_errors() {
    let records = fixture(JST_ERRORS);
    let files = convert_to_sota_activator(&records, "JA1ZZZ/1", &rmks1_qth());
    let adi = files
        .iter()
        .find(|(name, _)| name.ends_with(".adi"))
        .map(|(_, body)| body)
        .unwrap();
    assert!(!adi.contains("日時不正") && !adi.contains("周波数不正"));
    // 5行のうち、エラーでなく自局の山がある行(JA2AAA と JA2BBB)だけ
    assert_eq!(adi.matches("<EOR>").count(), 2);
}

#[test]
fn legacy_parity_hamlog_ios() {
    use super::super::hamlog::decode_auto;
    let csv = include_str!("../../../../../test_data/logconv/hamlog_ios.csv");
    let mut records = Vec::new();
    let mut header_seen = false;
    for line in csv.lines() {
        let cols: Vec<String> = line.split(',').map(String::from).collect();
        if let Ok(Some(qso)) = decode_auto(&cols, header_seen) {
            records.push(qso);
        }
        header_seen = true;
    }
    assert_eq!(records.len(), 5);

    let options = ConversionOptions {
        my_qth: MyQth::Rmks1,
        his_qth: HisQth::Rmks2,
        ..rmks1_qth()
    };
    let sota = convert_to_sota_activator(&records, "JA1ZZZ/1", &options);
    assert_eq!(
        sota["sota20251018.csv"],
        legacy!("ios_sota_a/sota20251018.csv")
    );
    assert_eq!(
        sota["sota-s2s-20251018.csv"],
        legacy!("ios_sota_a/sota-s2s-20251018.csv")
    );

    let adif = convert_to_adif(&records, &options);
    assert_eq!(adif.status, "NG"); // 5行目の周波数が範囲外
    assert_eq!(adif.files.len(), 2);
    // 旧版は、読み取りに失敗した行を、エラー文つきの記録として ADIF の中に残す。
    // 新版は、その記録を ADIF に入れない(エラーは、確認の画面と error_log で知らせる)。
    // それ以外の記録は、旧版と同じ
    for name in ["JA1ZZZ-1@JA-0001-20251018.adi"] {
        let legacy_adif = legacy!("ios_pota/JA1ZZZ-1@JA-0001-20251018.adi");
        assert!(legacy_adif.starts_with(adif.files[name].as_str()), "{name}");
    }
    assert_eq!(
        adif.files["JA1ZZZ-1@JA-TK-001-20251018.adi"],
        legacy!("ios_pota/JA1ZZZ-1@JA-TK-001-20251018.adi")
            .split("Err:")
            .next()
            .unwrap()
    );
    assert_eq!(adif.error_log.len(), 1);
    assert!(adif.error_log[0].starts_with("5行:"));
}

#[test]
fn legacy_parity_sota_chaser() {
    let files = convert_to_sota_chaser(&fixture(NFER), "JA1ZZZ", &rmks1_qth());
    let mut names: Vec<&String> = files.keys().collect();
    names.sort();
    assert_eq!(names, vec!["other20251018.csv", "sota20251018.csv"]);
    assert_eq!(
        files["sota20251018.csv"],
        legacy!("sota_c/sota20251018.csv")
    );
    assert_eq!(
        files["other20251018.csv"],
        legacy!("sota_c/other20251018.csv")
    );
}

#[test]
fn legacy_parity_adif_one_record_per_pair_of_references() {
    let result = convert_to_adif(&fixture(NFER), &rmks1_qth());
    let expected = [
        (
            "JA1ZZZ-1@JA-0001-20251018.adi",
            legacy!("pota/JA1ZZZ-1@JA-0001-20251018.adi"),
        ),
        (
            "JA1ZZZ-1@JA-0002-20251018.adi",
            legacy!("pota/JA1ZZZ-1@JA-0002-20251018.adi"),
        ),
        (
            "JA1ZZZ-1@JA-0003-20251018.adi",
            legacy!("pota/JA1ZZZ-1@JA-0003-20251018.adi"),
        ),
        (
            "JA1ZZZ-1@JA-TK-001-20251018.adi",
            legacy!("pota/JA1ZZZ-1@JA-TK-001-20251018.adi"),
        ),
        (
            "JA1ZZZ-1@JAFF-0001-20251018.adi",
            legacy!("pota/JA1ZZZ-1@JAFF-0001-20251018.adi"),
        ),
    ];
    assert_eq!(result.files.len(), expected.len());
    for (name, body) in expected {
        assert_eq!(result.files[name], body, "{name}");
    }
}

#[test]
fn test_create_zip() {
    let mut files = HashMap::new();
    files.insert("test.txt".to_string(), "Hello, World!".to_string());
    files.insert("test2.txt".to_string(), "Goodbye!".to_string());

    let zip_data = create_zip(&files).unwrap();
    assert!(!zip_data.is_empty());

    // Verify ZIP header
    assert_eq!(&zip_data[0..2], b"PK");
}
