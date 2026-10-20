//! 自局・相手の参照の種類と、出力の中身(SOTA の CSV の列、ADIF の項目)を、ケースごとに確かめる

use super::super::super::options::{ConversionOptions, HisQth, MyQth};
use super::super::*;
use super::support::*;
use std::collections::BTreeSet;

fn names(files: &std::collections::HashMap<String, String>) -> BTreeSet<&str> {
    files.keys().map(String::as_str).collect()
}

fn rmks1_qth() -> ConversionOptions {
    options(MyQth::Rmks1, HisQth::Qth)
}

// ---- A-2: 自局の参照の種類 ----

/// 自局の備考1に書いた参照の組み合わせごとの、ADIF のファイル(ファイル名の参照 → 記録の MY_SIG)
#[test]
fn pota_files_by_kind_of_my_references() {
    // (備考1, [(ファイルの参照, MY_SIG, MY_SOTA_REF)])
    type Want = &'static [(&'static str, Option<&'static str>)];
    let cases: [(&str, Want); 7] = [
        ("JA/TK-001", &[("JA-TK-001", None)]),
        ("JA-0001", &[("JA-0001", Some("POTA"))]),
        ("JAFF-0001", &[("JAFF-0001", Some("WWFF"))]),
        (
            "JA/TK-001 JA-0001",
            &[("JA-TK-001", None), ("JA-0001", Some("POTA"))],
        ),
        (
            "JA-0001 JA-0002 JA-0003",
            &[
                ("JA-0001", Some("POTA")),
                ("JA-0002", Some("POTA")),
                ("JA-0003", Some("POTA")),
            ],
        ),
        (
            "JA/TK-001 JA-0001 JAFF-0001",
            &[
                ("JA-TK-001", None),
                ("JA-0001", Some("POTA")),
                ("JAFF-0001", Some("WWFF")),
            ],
        ),
        // 参照のない備考。ファイルは作らない
        ("山頂から運用", &[]),
    ];
    for (rmks1, want) in cases {
        let rows = [row("JA1AAA", "01:00U", "", rmks1, "")];
        let result = convert_to_adif(&records(&rows), &rmks1_qth());
        let case = format!("備考1 = {rmks1}");
        let got: BTreeSet<String> = result.files.keys().cloned().collect();
        let want_names: BTreeSet<String> = want
            .iter()
            .map(|(r, _)| format!("JA1ZZZ-1@{r}-20251018.adi"))
            .collect();
        assert_eq!(got, want_names, "{case}");
        for (reference, sig) in want {
            let r = &adif_records(&result.files[&format!("JA1ZZZ-1@{reference}-20251018.adi")])[0];
            assert_eq!(r.get("MY_SIG").map(String::as_str), *sig, "{case}");
            // 山の記録は MY_SOTA_REF、公園・WWFF は MY_SIG_INFO(MY_POTA_REF は使わない: 仕様の確認が要る)
            if sig.is_none() {
                assert_eq!(r["MY_SOTA_REF"], reference.replacen('-', "/", 1), "{case}");
            } else {
                assert_eq!(&r["MY_SIG_INFO"], reference, "{case}");
                assert!(!r.contains_key("MY_SOTA_REF") && !r.contains_key("MY_POTA_REF"));
            }
        }
        // 公園だけが、確認画面のファイルの一覧に入る(山と WWFF は入らない)
        let parks: Vec<&str> = want
            .iter()
            .filter(|(r, s)| *s == Some("POTA") && r.starts_with("JA-"))
            .map(|(r, _)| *r)
            .collect();
        assert_eq!(result.file_list.len(), parks.len(), "{case}");
    }
}

/// 自局の参照がある行とない行が混ざるログ: ない行は、ファイルに入らず、表には空の参照で残る
#[test]
fn rows_without_my_reference_are_left_out_of_the_files_but_kept_in_the_table() {
    let rows = [
        row("JA1AAA", "01:00U", "", "JA/TK-001 JA-0001", ""),
        row("JA1BBB", "01:01U", "", "", ""),
        row("JA1CCC", "01:02U", "", "移動中", ""),
        row("JA1DDD", "01:03U", "", "JA/TK-001 JA-0001", ""),
    ];
    let recs = records(&rows);

    let result = convert_to_adif(&recs, &rmks1_qth());
    assert_eq!(result.status, "OK");
    for name in [
        "JA1ZZZ-1@JA-0001-20251018.adi",
        "JA1ZZZ-1@JA-TK-001-20251018.adi",
    ] {
        let calls: Vec<String> = adif_records(&result.files[name])
            .into_iter()
            .map(|r| r["CALL"].clone())
            .collect();
        assert_eq!(calls, ["JA1AAA", "JA1DDD"], "{name}");
    }
    let table: Vec<(&str, &str)> = result
        .log_text
        .iter()
        .map(|r| (r[0].as_str(), r[8].as_str()))
        .collect();
    assert_eq!(
        table,
        [
            ("JA1AAA", "JA/TK-001/JA-0001"),
            ("JA1BBB", ""),
            ("JA1CCC", ""),
            ("JA1DDD", "JA/TK-001/JA-0001")
        ]
    );

    let files = convert_to_sota_activator(&recs, "JA1ZZZ/1", &rmks1_qth());
    let calls: Vec<&str> = files["sota20251018.csv"]
        .lines()
        .map(|l| cols(l)[7])
        .collect();
    assert_eq!(calls, ["JA1AAA", "JA1DDD"]);
}

/// 山の移動(運用の途中で山が変わる): SOTA の CSV は交信ごとの山、ADIF は山ごとのファイル(日付はその交信の日付)
#[test]
fn moving_between_summits_in_one_operation() {
    let rows = [
        "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,JA/TK-001,,",
        "JA1BBB,25/10/18,02:00U,59,59,7.030,CW,,,N,,,JA/TK-001,,",
        "JA1CCC,25/10/18,23:50U,59,59,7.030,CW,,,N,,,JA/TK-002,,",
        "JA1DDD,25/10/19,00:10U,59,59,7.030,CW,,,N,,,JA/TK-002,,",
    ];
    let recs = super::fixture(&rows.join("\n"));

    let files = convert_to_sota_activator(&recs, "JA1ZZZ/1", &rmks1_qth());
    let summits = |name: &str| -> Vec<(&str, &str)> {
        files[name]
            .lines()
            .map(|l| {
                let c = cols(l);
                (c[2], c[7])
            })
            .collect::<Vec<_>>()
    };
    // 山ごとではなく日付ごと。山は交信ごとの列
    assert_eq!(
        summits("sota20251018.csv"),
        [
            ("JA/TK-001", "JA1AAA"),
            ("JA/TK-001", "JA1BBB"),
            ("JA/TK-002", "JA1CCC")
        ]
    );
    assert_eq!(summits("sota20251019.csv"), [("JA/TK-002", "JA1DDD")]);

    let result = convert_to_adif(&recs, &rmks1_qth());
    assert_eq!(
        names(&result.files),
        BTreeSet::from([
            "JA1ZZZ-1@JA-TK-001-20251018.adi",
            "JA1ZZZ-1@JA-TK-002-20251018.adi",
            "JA1ZZZ-1@JA-TK-002-20251019.adi",
        ])
    );
}

/// 公園と WWFF は、日をまたいでも、最初の交信の日付の1つのファイルにまとめる(POTA は1回の運用を1ファイルで出すため)
#[test]
fn park_file_spans_dates_and_is_named_after_the_first_contact() {
    let rows = [
        "JA1AAA,25/10/18,23:50U,59,59,7.030,CW,,,N,,,JA-0001 JAFF-0001,,",
        "JA1BBB,25/10/19,00:10U,59,59,7.030,CW,,,N,,,JA-0001 JAFF-0001,,",
    ];
    let result = convert_to_adif(&super::fixture(&rows.join("\n")), &rmks1_qth());
    assert_eq!(
        names(&result.files),
        BTreeSet::from([
            "JA1ZZZ-1@JA-0001-20251018.adi",
            "JA1ZZZ-1@JAFF-0001-20251018.adi"
        ])
    );
    let r = adif_records(&result.files["JA1ZZZ-1@JA-0001-20251018.adi"]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[1]["QSO_DATE"], "20251019");
}

// ---- A-3: 相手の参照 ----

fn his_adif(his: &str) -> Vec<std::collections::BTreeMap<String, String>> {
    let rows = [row(
        "JA1AAA",
        "01:00U",
        his,
        "JA/TK-001 JA-0001 JAFF-0001",
        "",
    )];
    let result = convert_to_adif(&records(&rows), &rmks1_qth());
    let mut all = Vec::new();
    for name in [
        "JA1ZZZ-1@JA-0001-20251018.adi",
        "JA1ZZZ-1@JA-TK-001-20251018.adi",
        "JA1ZZZ-1@JAFF-0001-20251018.adi",
    ] {
        all.extend(adif_records(&result.files[name]));
    }
    all
}

/// 相手の参照が自由な文と混ざった欄: 参照だけを取る(日本語の語、括弧、感嘆符つき)
#[test]
fn his_references_among_free_text() {
    let recs = his_adif("JA-0123 TNX 山頂 FB! (JA/KN-001)");
    // P2P(JA-0123)。括弧で囲んだ山(JA/KN-001)も読む(語の中から参照の形を探すため)
    let pota: Vec<_> = recs
        .iter()
        .filter(|r| r.get("MY_SIG").map(String::as_str) == Some("POTA"))
        .collect();
    assert_eq!(pota[0]["SIG_INFO"], "JA-0123");
    let sota = recs.iter().find(|r| r.contains_key("MY_SOTA_REF")).unwrap();
    assert_eq!(sota["SOTA_REF"], "JA/KN-001");
}

/// 相手の参照: W2W は WWFF の記録だけ、P2P は POTA の記録だけ、S2S は山の記録だけに付く
#[test]
fn his_references_go_to_the_record_of_the_same_kind() {
    let recs = his_adif("JA/KN-001 JA-0123 JAFF-0456");
    let by_sig = |sig: &str| {
        recs.iter()
            .find(|r| r.get("MY_SIG").map(String::as_str) == Some(sig))
            .unwrap()
    };
    assert_eq!(
        (
            by_sig("POTA")["SIG"].as_str(),
            by_sig("POTA")["SIG_INFO"].as_str()
        ),
        ("POTA", "JA-0123")
    );
    assert_eq!(
        (
            by_sig("WWFF")["SIG"].as_str(),
            by_sig("WWFF")["SIG_INFO"].as_str()
        ),
        ("WWFF", "JAFF-0456")
    );
    let sota = recs.iter().find(|r| r.contains_key("MY_SOTA_REF")).unwrap();
    assert_eq!(sota["SOTA_REF"], "JA/KN-001");
    assert!(!sota.contains_key("SIG"));
}

/// 全角の空白(U+3000)で区切った参照は読める(空白として扱う)
#[test]
fn his_references_separated_by_ideographic_space() {
    let recs = his_adif("JA/KN-001\u{3000}JA-0123\u{3000}JAFF-0456");
    assert!(recs
        .iter()
        .any(|r| r.get("SIG_INFO").map(String::as_str) == Some("JA-0123")));
    assert!(recs
        .iter()
        .any(|r| r.get("SIG_INFO").map(String::as_str) == Some("JAFF-0456")));
    assert!(recs
        .iter()
        .any(|r| r.get("SOTA_REF").map(String::as_str) == Some("JA/KN-001")));
}

/// 全角の英数字・全角のハイフン・全角のスラッシュで書いた参照(日本語の入力で起きる)は、
/// いまは参照として読まれない(相手の参照なしとして変換する。エラーにもならない)。
/// 仕様の確認が要る: 半角にそろえて(NFKC)読むべきか、少なくとも警告を出すべきか
#[test]
fn fullwidth_references_are_not_recognized() {
    for his in [
        "ＪＡ／ＫＮ－００１",
        "ＪＡ－０１２３",
        "JA－0123",
        "JA/KN－001",
        "JA／KN-001",
    ] {
        let recs = his_adif(his);
        assert!(
            recs.iter()
                .all(|r| !r.contains_key("SIG_INFO") && !r.contains_key("SOTA_REF")),
            "{his}"
        );
    }
}

/// 全角の自局の参照(備考1)も読まれない: 参照のない行として、ファイルに入らない。仕様の確認が要る(上と同じ)
#[test]
fn fullwidth_my_reference_is_not_recognized() {
    let rows = [row("JA1AAA", "01:00U", "", "ＪＡ／ＴＫ－００１", "")];
    let result = convert_to_adif(&records(&rows), &rmks1_qth());
    assert!(result.files.is_empty());
    assert_eq!(result.status, "OK");
}

/// 半角の英字に全角の数字が続く参照(`JA-０１２３`)は、そのまま参照として読んでしまう(`\d` が全角の数字に合う)。
/// ADIF の SIG_INFO に全角の数字が出る(POTA に取り込めない)。読まないか、半角にするべき
#[test]
#[ignore = "不具合: 全角の数字を含む参照(JA-０１２３)を参照として読み、ADIF に全角のまま出す"]
fn fullwidth_digits_after_ascii_prefix_do_not_leak_into_adif() {
    let recs = his_adif("JA-０１２３");
    for r in &recs {
        for v in r.values() {
            assert!(v.is_ascii(), "ASCII でない値: {v}");
        }
    }
}

/// 小文字の参照は大文字にして出す(相手・自局とも)
#[test]
fn lowercase_references_are_uppercased() {
    let rows = [row(
        "JA1AAA",
        "01:00U",
        "ja/kn-001 ja-0123 jaff-0456",
        "ja/tk-001 ja-0001",
        "",
    )];
    let result = convert_to_adif(&records(&rows), &rmks1_qth());
    let r = &adif_records(&result.files["JA1ZZZ-1@JA-0001-20251018.adi"])[0];
    assert_eq!(
        (r["MY_SIG_INFO"].as_str(), r["SIG_INFO"].as_str()),
        ("JA-0001", "JA-0123")
    );
    let s = &adif_records(&result.files["JA1ZZZ-1@JA-TK-001-20251018.adi"])[0];
    assert_eq!(s["SOTA_REF"], "JA/KN-001");
}

/// 相手の参照に座標がついた欄: 山の参照と座標は両方読む。CSV の最後の列は座標の注記
#[test]
fn his_summit_with_coordinates_in_the_qth_column() {
    let rows = [row(
        "JA1AAA",
        "01:00U",
        "\"JA/KN-001 35.6N, 139.2E\"",
        "JA/TK-001",
        "",
    )];
    let recs = fixture_quoted(&rows);
    let files = convert_to_sota_activator(&recs, "JA1ZZZ/1", &rmks1_qth());
    assert_eq!(
        files["sota-s2s-20251018.csv"],
        "V2,JA1ZZZ/1,JA/TK-001,18/10/2025,01:00,7MHz,CW,JA1AAA,JA/KN-001,\"%QTH%35.6N, 139.2E%  \"\n"
    );
}

// ---- A-4: 出力の中身(期待の値を、仕様から書く) ----

const SOTA_CSV_LINE: &str = "V2,JA1ZZZ/1,JA/TK-001,18/10/2025,01:00,7MHz,CW,JA1AAA,JA/KN-001, \n";

/// SOTA の CSV V2: `V2,自局,自局の山,DD/MM/YYYY,HH:MM,バンド,モード,相手,相手の山,注記`(末尾に空白1つ)
#[test]
fn sota_csv_v2_columns() {
    let rows = [row("JA1AAA", "01:00U", "JA/KN-001", "JA/TK-001", "")];
    let files = convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &rmks1_qth());
    assert_eq!(files["sota20251018.csv"], SOTA_CSV_LINE);
    assert_eq!(files["sota-s2s-20251018.csv"], SOTA_CSV_LINE);
    assert_eq!(cols(SOTA_CSV_LINE.trim_end_matches('\n')).len(), 10);
}

/// バンドとモードの SOTA での名前(CSV)と、ADIF での名前
#[test]
fn band_and_mode_names_in_each_output() {
    // (周波数, モード, SOTA のバンド, SOTA のモード, ADIF の BAND, ADIF の MODE)
    let cases = [
        ("1.9", "CW", "1.8MHz", "CW", "160m", "CW"),
        ("14.285", "SSB", "14MHz", "SSB", "20m", "SSB"),
        ("144.6", "FM", "144MHz", "FM", "2m", "FM"),
        ("430.1", "FM", "433MHz", "FM", "70cm", "FM"),
    ];
    for (freq, mode, sota_band, sota_mode, band, adif_mode) in cases {
        let line = format!("JA1AAA,25/10/18,01:00U,59,59,{freq},{mode},,,N,,,JA/TK-001,,");
        let recs = super::fixture(&line);
        let files = convert_to_sota_activator(&recs, "JA1ZZZ/1", &rmks1_qth());
        let csv = cols(files["sota20251018.csv"].lines().next().unwrap())
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            (csv[5].as_str(), csv[6].as_str()),
            (sota_band, sota_mode),
            "{freq}"
        );
        let a = &adif_records(&files["sota20251018.adi"])[0];
        assert_eq!(
            (a["BAND"].as_str(), a["MODE"].as_str()),
            (band, adif_mode),
            "{freq}"
        );
    }
}

/// ADIF の全項目(SOTA アクティベーター): 項目の並びと、ヘッダ(ADIF 3.0.6)。項目は空白で区切る
#[test]
fn sota_adif_record_has_every_field() {
    let rows = [row("JA1AAA", "01:00U", "JA/KN-001", "JA/TK-001", "")];
    let files = convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &rmks1_qth());
    assert_eq!(
        files["sota20251018.adi"],
        "ADIF Export from HAMLOG by JL1NIE\n<PROGRAMID:4>FCTH\n<ADIF_VER:5>3.0.6\n<EOH>\n\
         <STATION_CALLSIGN:8>JA1ZZZ/1 <OPERATOR:6>JA1ZZZ <CALL:6>JA1AAA <QSO_DATE:8>20251018 \
         <TIME_ON:4>0100 <BAND:3>40m <MODE:2>CW <RST_SENT:2>59 <RST_RCVD:2>59 \
         <MY_SOTA_REF:9>JA/TK-001 <SOTA_REF:9>JA/KN-001 <EOR>\n"
    );
}

/// ADIF の全項目(POTA の公園・WWFF・山)。ヘッダは ADIF 3.1.4
#[test]
fn pota_adif_records_have_every_field() {
    let rows = [row(
        "JA1AAA",
        "01:00U",
        "JA/KN-001 JA-0123 JAFF-0456",
        "JA/TK-001 JA-0001 JAFF-0001",
        "",
    )];
    let result = convert_to_adif(&records(&rows), &rmks1_qth());
    let header = "ADIF Export from HAMLOG by JL1NIE\n<PROGRAMID:4>FCTH\n<ADIF_VER:5>3.1.4\n<EOH>\n";
    let base = "<STATION_CALLSIGN:8>JA1ZZZ/1<OPERATOR:6>JA1ZZZ<CALL:6>JA1AAA<QSO_DATE:8>20251018<TIME_ON:4>0100<BAND:3>40m<MODE:2>CW";
    let rst = "<RST_SENT:2>59<RST_RCVD:2>59";
    // 公園: RST なし
    assert_eq!(
        result.files["JA1ZZZ-1@JA-0001-20251018.adi"],
        format!("{header}{base}<MY_SIG:4>POTA<MY_SIG_INFO:7>JA-0001<SIG:4>POTA<SIG_INFO:7>JA-0123<EOR>\n")
    );
    // WWFF・山: RST あり
    assert_eq!(
        result.files["JA1ZZZ-1@JAFF-0001-20251018.adi"],
        format!("{header}{base}{rst}<MY_SIG:4>WWFF<MY_SIG_INFO:9>JAFF-0001<SIG:4>WWFF<SIG_INFO:9>JAFF-0456<EOR>\n")
    );
    assert_eq!(
        result.files["JA1ZZZ-1@JA-TK-001-20251018.adi"],
        format!("{header}{base}{rst}<MY_SOTA_REF:9>JA/TK-001<SOTA_REF:9>JA/KN-001<EOR>\n")
    );
}

/// 日付ごとのファイル(SOTA の CSV)と、S2S のファイルは、同じ交信の行が同じ
#[test]
fn s2s_file_lines_are_a_subset_of_the_date_file() {
    let rows = [
        row("JA1AAA", "01:00U", "JA/KN-001", "JA/TK-001", ""),
        row("JA1BBB", "01:01U", "", "JA/TK-001", ""),
        row("JA1CCC", "01:02U", "JA/KN-002", "JA/TK-001", ""),
    ];
    let files = convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &rmks1_qth());
    let all: Vec<&str> = files["sota20251018.csv"].lines().collect();
    let s2s: Vec<&str> = files["sota-s2s-20251018.csv"].lines().collect();
    assert_eq!(all.len(), 3);
    assert_eq!(s2s, [all[0], all[2]]);
}
