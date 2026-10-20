//! 参照の組み合わせを網羅するテスト: POTA の自局 n-fer × 相手 m-fer、SOTA の S2S と相手の参照の列
//!
//! 旧版との一致(`legacy_parity_*`)は決まった例だけなので、ここでは組み合わせを作って、結果の性質を確かめる。

use super::super::super::adif::parse_adif;
use super::super::super::hamlog::decode_auto;
use super::super::super::options::{ConversionOptions, HisQth, MyQth};
use super::super::super::record::QsoRecord;
use super::super::*;
use super::fixture;
use std::collections::{BTreeMap, BTreeSet};

/// HAMLOG CSV の1行(`コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,`)
fn row(call: &str, time: &str, qth: &str, rmks1: &str, rmks2: &str) -> String {
    format!("{call},25/10/18,{time},59,59,7.030,CW,,,N,,{qth},{rmks1},{rmks2},")
}

fn records(rows: &[String]) -> Vec<QsoRecord> {
    fixture(&rows.join("\n"))
}

/// 引用符(`"JA/TK-001,JA-0001"`)を含む行も読む(画面と同じ CSV の読み方)
fn quoted_records(rows: &[String]) -> Vec<QsoRecord> {
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

/// 自局の参照・相手の参照を、指定の列に置いた1行
fn row_at(call: &str, time: &str, my: (MyQth, &str), his: (HisQth, &str)) -> String {
    let mut cols = [""; 3]; // QTH, 備考1, 備考2
    match my.0 {
        MyQth::Rmks1 => cols[1] = my.1,
        MyQth::Rmks2 => cols[2] = my.1,
        MyQth::Form => {}
    }
    match his.0 {
        HisQth::Qth => cols[0] = his.1,
        HisQth::Rmks1 => cols[1] = his.1,
        HisQth::Rmks2 => cols[2] = his.1,
        HisQth::None => {}
    }
    row(call, time, cols[0], cols[1], cols[2])
}

fn options(my_qth: MyQth, his_qth: HisQth) -> ConversionOptions {
    ConversionOptions {
        my_qth,
        his_qth,
        pota_activator: "JA1ZZZ/1".to_string(),
        pota_operator: "JA1ZZZ".to_string(),
        ..Default::default()
    }
}

/// 自局・相手の列の置き方(画面で選べる組み合わせ)
const PLACES: [(MyQth, HisQth); 4] = [
    (MyQth::Rmks1, HisQth::Qth),
    (MyQth::Rmks1, HisQth::Rmks2),
    (MyQth::Rmks2, HisQth::Rmks1),
    (MyQth::Rmks2, HisQth::Qth),
];

/// ADIF のファイルの記録ごとの (CALL, MY_SIG_INFO, SIG, SIG_INFO, SOTA_REF)
fn adif_rows(body: &str) -> Vec<BTreeMap<&'static str, String>> {
    parse_adif(body)
        .into_iter()
        .map(|r| {
            [
                "CALL",
                "MY_SIG",
                "MY_SIG_INFO",
                "SIG",
                "SIG_INFO",
                "MY_SOTA_REF",
                "SOTA_REF",
            ]
            .into_iter()
            .filter_map(|k| Some((k, r.get(k)?.clone())))
            .collect()
        })
        .collect()
}

fn parks(prefix: &str, start: usize, n: usize) -> Vec<String> {
    (start..start + n)
        .map(|i| format!("{prefix}-{i:04}"))
        .collect()
}

fn file_of(park: &str) -> String {
    format!("JA1ZZZ-1@{}-20251018.adi", park.replace('/', "-"))
}

// 自局 n-fer(1〜3)× 相手 m-fer(0〜3)× 列の置き方: 自局の公園ごとに1つのファイル。各ファイルに、相手の公園ごとの記録
// (相手がなければ1件)。自局を画面で入力しても(`form`)、同じ
#[test]
fn pota_n_fer_by_m_fer() {
    for n in 1..=3 {
        for m in 0..=3 {
            let mine = parks("JA", 1, n);
            let theirs = parks("JA", 101, m);
            let (my_text, his_text) = (mine.join(" "), theirs.join(" "));
            let mut cases: Vec<(String, ConversionOptions, String)> = PLACES
                .iter()
                .map(|&(my, his)| {
                    (
                        format!("{my:?}/{his:?}"),
                        options(my, his),
                        row_at("JA1AAA", "01:00U", (my, &my_text), (his, &his_text)),
                    )
                })
                .collect();
            cases.push((
                "Form/Qth".to_string(),
                ConversionOptions {
                    park: vec![my_text.clone()],
                    ..options(MyQth::Form, HisQth::Qth)
                },
                row_at(
                    "JA1AAA",
                    "01:00U",
                    (MyQth::Form, ""),
                    (HisQth::Qth, &his_text),
                ),
            ));

            for (place, opts, line) in cases {
                let case = format!("自局 {n}-fer × 相手 {m}-fer({place})");
                let result = convert_to_adif(&records(&[line]), &opts);
                assert_eq!(result.status, "OK", "{case}");
                let names: BTreeSet<&str> = result.files.keys().map(String::as_str).collect();
                let want: BTreeSet<String> = mine.iter().map(|p| file_of(p)).collect();
                assert_eq!(
                    names,
                    want.iter().map(String::as_str).collect(),
                    "{case}: ファイル"
                );
                assert_eq!(
                    result.file_list.len(),
                    n,
                    "{case}: 確認画面のファイルの一覧"
                );

                for park in &mine {
                    let rows = adif_rows(&result.files[&file_of(park)]);
                    assert_eq!(rows.len(), m.max(1), "{case}/{park}: 記録の数");
                    for r in &rows {
                        assert_eq!(r["MY_SIG"], "POTA", "{case}");
                        assert_eq!(&r["MY_SIG_INFO"], park, "{case}");
                    }
                    let got: Vec<Option<&String>> =
                        rows.iter().map(|r| r.get("SIG_INFO")).collect();
                    let want: Vec<Option<&String>> = if m == 0 {
                        vec![None]
                    } else {
                        theirs.iter().map(Some).collect()
                    };
                    assert_eq!(got, want, "{case}/{park}: 相手の公園");
                    assert!(
                        rows.iter().all(|r| r.contains_key("SIG") == (m > 0)),
                        "{case}: 相手がいなければ SIG もない"
                    );
                }
                // 確認画面の表: 相手・自局の参照
                let display = &result.log_text[0];
                assert_eq!(display[7], theirs.join("/"), "{case}");
                assert_eq!(display[8], mine.join("/"), "{case}");
            }
        }
    }
}

// いくつかの交信: 自局の公園のファイルの記録の数は、その公園の交信ごとの max(相手の公園の数, 1) の和
#[test]
fn pota_records_add_up_over_the_contacts() {
    let log = [
        ("JA1AAA", "JA-0001 JA-0002", ""),
        ("JA1BBB", "JA-0001 JA-0002", "JA-0101 JA-0102 JA-0103"),
        ("JA1CCC", "JA-0001", "JA-0101"),
        ("JA1DDD", "JA-0002 JA-0003", "JA-0101 JA-0102"),
    ];
    let rows: Vec<String> = log
        .iter()
        .enumerate()
        .map(|(i, (call, my, his))| row(call, &format!("01:0{i}U"), his, my, ""))
        .collect();
    let result = convert_to_adif(&records(&rows), &options(MyQth::Rmks1, HisQth::Qth));
    let count = |park: &str| adif_rows(&result.files[&file_of(park)]).len();
    assert_eq!(count("JA-0001"), 1 + 3 + 1);
    assert_eq!(count("JA-0002"), 1 + 3 + 2);
    assert_eq!(count("JA-0003"), 2);
    // 記録の (相手, 相手の公園) に重なりはない
    for body in result.files.values() {
        let pairs: Vec<_> = adif_rows(body)
            .into_iter()
            .map(|r| (r["CALL"].clone(), r.get("SIG_INFO").cloned()))
            .collect();
        let unique: BTreeSet<_> = pairs.iter().collect();
        assert_eq!(unique.len(), pairs.len());
    }
}

// 山・公園・WWFF の組み合わせ: 山のファイルは S2S だけ、公園は P2P だけ、WWFF は W2W だけを相手の参照にする
#[test]
fn pota_mixed_references() {
    // (自局, 相手, ファイルの参照 → 記録ごとの相手の参照)
    type Want = &'static [(&'static str, &'static [Option<&'static str>])];
    let cases: [(&str, &str, Want); 7] = [
        (
            "JA/TK-001 JA-0001 JA-0002 JAFF-0001",
            "JA/KN-001 JA-0123 JA-0124 JAFF-0456 JAFF-0457",
            &[
                ("JA/TK-001", &[Some("JA/KN-001")]),
                ("JA-0001", &[Some("JA-0123"), Some("JA-0124")]),
                ("JA-0002", &[Some("JA-0123"), Some("JA-0124")]),
                ("JAFF-0001", &[Some("JAFF-0456"), Some("JAFF-0457")]),
            ],
        ),
        // 相手が山だけ: 公園の記録は1件(相手の公園なし)
        (
            "JA/TK-001 JA-0001",
            "JA/KN-001",
            &[("JA/TK-001", &[Some("JA/KN-001")]), ("JA-0001", &[None])],
        ),
        // 自局に山がなければ、相手の山は記録に出ない(S2S ではない)
        (
            "JA-0001",
            "JA/KN-001 JA-0123",
            &[("JA-0001", &[Some("JA-0123")])],
        ),
        // 自局が山だけ: 相手の公園は、山の記録に出ない
        ("JA/TK-001", "JA-0123 JA-0124", &[("JA/TK-001", &[None])]),
        // WWFF だけ
        (
            "JAFF-0001",
            "JAFF-0456",
            &[("JAFF-0001", &[Some("JAFF-0456")])],
        ),
        // 小文字・カンマ区切りでも読む
        (
            "ja/tk-001,ja-0001",
            "ja/kn-001,ja-0123",
            &[
                ("JA/TK-001", &[Some("JA/KN-001")]),
                ("JA-0001", &[Some("JA-0123")]),
            ],
        ),
        // 参照でない語は無視する
        (
            "JA-0001 (山頂)",
            "JA-0123 (TNX)",
            &[("JA-0001", &[Some("JA-0123")])],
        ),
    ];
    for (mine, theirs, want) in cases {
        // カンマを含む参照は、CSV の1列にするため、引用符で囲む
        let quote = |s: &str| format!("\"{s}\"");
        let line = row("JA1AAA", "01:00U", &quote(theirs), &quote(mine), "");
        let result = convert_to_adif(
            &quoted_records(&[line]),
            &options(MyQth::Rmks1, HisQth::Qth),
        );
        let case = format!("自局 {mine} / 相手 {theirs}");
        let names: BTreeSet<String> = result.files.keys().cloned().collect();
        let want_names: BTreeSet<String> = want.iter().map(|(r, _)| file_of(r)).collect();
        assert_eq!(names, want_names, "{case}");
        for (reference, his) in want {
            let rows = adif_rows(&result.files[&file_of(reference)]);
            let key = if reference.contains('/') {
                "SOTA_REF"
            } else {
                "SIG_INFO"
            };
            let got: Vec<Option<&str>> = rows
                .iter()
                .map(|r| r.get(key).map(String::as_str))
                .collect();
            assert_eq!(got, *his, "{case}/{reference}");
        }
    }
}

/// S2S のテストのログ: 相手なし・S2S・P2P だけ・S2S+P2P・座標つきの S2S
const S2S_LOG: [(&str, &str); 5] = [
    ("JA1AAA", ""),
    ("JA1BBB", "JA/KN-001"),
    ("JA1CCC", "JA-0123"),
    ("JA1DDD", "ja/kn-002 JA-0123"),
    ("JA1EEE", "JA/KN-003 35.6N 139.2E"),
];

fn s2s_log(my: (MyQth, &str), his_qth: HisQth) -> Vec<QsoRecord> {
    let rows: Vec<String> = S2S_LOG
        .iter()
        .enumerate()
        .map(|(i, (call, his))| row_at(call, &format!("01:0{i}U"), my, (his_qth, his)))
        .collect();
    records(&rows)
}

fn sota_options(my_qth: MyQth, his_qth: HisQth) -> ConversionOptions {
    ConversionOptions {
        summit: "JA/TK-001".to_string(),
        ..options(my_qth, his_qth)
    }
}

// SOTA アクティベーター: 相手の山を、どの列に置いても(備考1・備考2・QTH)、S2S のファイルと ADIF の SOTA_REF になる。
// 「指定しない」なら S2S はない。交信は、どれも全部残る
#[test]
fn sota_activator_s2s_from_every_column() {
    let s2s_calls = ["JA1BBB", "JA1DDD", "JA1EEE"];
    let cases = [
        (MyQth::Rmks1, HisQth::Rmks2),
        (MyQth::Rmks2, HisQth::Rmks1),
        (MyQth::Rmks1, HisQth::Qth),
        (MyQth::Form, HisQth::Qth),
        (MyQth::Form, HisQth::Rmks1),
        (MyQth::Rmks1, HisQth::None),
    ];
    for (my, his) in cases {
        let case = format!("{my:?}/{his:?}");
        let records = s2s_log(
            (my, "JA/TK-001"),
            if his == HisQth::None {
                HisQth::Qth
            } else {
                his
            },
        );
        let files = convert_to_sota_activator(&records, "JA1ZZZ/1", &sota_options(my, his));

        let all = &files["sota20251018.csv"];
        assert_eq!(all.lines().count(), S2S_LOG.len(), "{case}");
        assert!(
            all.lines()
                .all(|l| l.starts_with("V2,JA1ZZZ/1,JA/TK-001,18/10/2025,")),
            "{case}"
        );

        let adif = adif_rows(&files["sota20251018.adi"]);
        assert_eq!(adif.len(), S2S_LOG.len(), "{case}");
        let adif_s2s: Vec<(&str, &str)> = adif
            .iter()
            .filter_map(|r| Some((r["CALL"].as_str(), r.get("SOTA_REF")?.as_str())))
            .collect();

        if his == HisQth::None {
            assert!(!files.contains_key("sota-s2s-20251018.csv"), "{case}");
            assert!(adif_s2s.is_empty(), "{case}");
            continue;
        }
        let s2s: Vec<&str> = files["sota-s2s-20251018.csv"]
            .lines()
            .map(|l| l.split(',').nth(7).unwrap())
            .collect();
        assert_eq!(s2s, s2s_calls, "{case}: S2S のファイル");
        assert_eq!(
            adif_s2s,
            [
                ("JA1BBB", "JA/KN-001"),
                ("JA1DDD", "JA/KN-002"),
                ("JA1EEE", "JA/KN-003")
            ],
            "{case}: ADIF の SOTA_REF(小文字は大文字に)"
        );
        // 座標は、最後の列(場所の注記)。HAMLOG の CSV にカンマなしで書くと、座標と読まない
        let eee = all.lines().find(|l| l.contains(",JA1EEE,")).unwrap();
        assert!(eee.ends_with(",JA/KN-003, "), "{case}: {eee}");
    }
}

// 座標は「緯度, 経度」と、カンマで区切ったときだけ場所の注記になる(カンマを含むので、CSV では引用符で囲む。旧版と同じ)
#[test]
fn sota_activator_location_note_needs_a_comma() {
    let line = row(
        "JA1EEE",
        "01:00U",
        "\"JA/KN-003 35.6N, 139.2E\"",
        "JA/TK-001",
        "",
    );
    let records = quoted_records(&[line]);
    let files = convert_to_sota_activator(
        &records,
        "JA1ZZZ/1",
        &sota_options(MyQth::Rmks1, HisQth::Qth),
    );
    assert!(
        files["sota-s2s-20251018.csv"].ends_with(",JA1EEE,JA/KN-003,\"%QTH%35.6N, 139.2E%  \"\n"),
        "{}",
        files["sota-s2s-20251018.csv"]
    );
}

// 自局の山がない交信(備考に山がない行)は、SOTA の CSV にも ADIF にも出ない
#[test]
fn sota_activator_skips_contacts_without_my_summit() {
    let rows = [
        row("JA1AAA", "01:00U", "JA/KN-001", "JA/TK-001", ""),
        row("JA1BBB", "01:01U", "JA/KN-001", "JA-0001", ""),
        row("JA1CCC", "01:02U", "", "", ""),
    ];
    let files = convert_to_sota_activator(
        &records(&rows),
        "JA1ZZZ/1",
        &sota_options(MyQth::Rmks1, HisQth::Qth),
    );
    assert_eq!(files["sota20251018.csv"].lines().count(), 1);
    assert_eq!(files["sota-s2s-20251018.csv"].lines().count(), 1);
    assert_eq!(adif_rows(&files["sota20251018.adi"]).len(), 1);
}

// JST で書いたログ: S2S のファイルも UTC の日付ごと。ADIF は1つ(最初の交信の UTC の日付)
#[test]
fn sota_activator_s2s_split_by_utc_date() {
    let rows = [
        "JA1AAA,25/10/19,08:50J,59,59,7.030,CW,,,N,,JA/KN-001,JA/TK-001,,",
        "JA1BBB,25/10/19,08:55J,59,59,7.030,CW,,,N,,,JA/TK-001,,",
        "JA1CCC,25/10/19,09:05J,59,59,7.030,CW,,,N,,JA/KN-002,JA/TK-001,,",
    ];
    let files = convert_to_sota_activator(
        &fixture(&rows.join("\n")),
        "JA1ZZZ/1",
        &sota_options(MyQth::Rmks1, HisQth::Qth),
    );
    let names: BTreeSet<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        names,
        BTreeSet::from([
            "sota20251018.csv",
            "sota20251019.csv",
            "sota-s2s-20251018.csv",
            "sota-s2s-20251019.csv",
            "sota20251018.adi",
        ])
    );
    assert!(files["sota-s2s-20251018.csv"].contains(",18/10/2025,23:50,"));
    assert!(files["sota-s2s-20251019.csv"].contains(",19/10/2025,00:05,"));
    assert_eq!(adif_rows(&files["sota20251018.adi"]).len(), 3);
}

// SOTA チェイサー: 相手の山をどの列に置いても、山との交信は `sota…`、ほかは `other…`
#[test]
fn sota_chaser_from_every_column() {
    for his in [HisQth::Rmks1, HisQth::Rmks2, HisQth::Qth] {
        let records = s2s_log((MyQth::Form, ""), his);
        let files = convert_to_sota_chaser(&records, "JA1ZZZ", &options(MyQth::Form, his));
        let calls = |name: &str| -> Vec<String> {
            files[name]
                .lines()
                .map(|l| l.split(',').nth(7).unwrap().to_string())
                .collect()
        };
        assert_eq!(
            calls("sota20251018.csv"),
            ["JA1BBB", "JA1DDD", "JA1EEE"],
            "{his:?}"
        );
        assert_eq!(calls("other20251018.csv"), ["JA1AAA", "JA1CCC"], "{his:?}");
        assert!(
            files["sota20251018.csv"]
                .lines()
                .all(|l| l.starts_with("V2,JA1ZZZ,,")),
            "{his:?}"
        );
    }
}
