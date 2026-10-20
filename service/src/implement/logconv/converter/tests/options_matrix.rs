//! 形式 3(sota_a・sota_c・pota)× 自局の列 3(form・rmks1・rmks2)× 相手の列 4(none・qth・rmks1・rmks2)の全組み合わせと、
//! 画面で指定する `Summit`・`Park`・コールサインの扱い
//!
//! 使わない列には、読まれてはいけない参照(`DECOY`)を置く。列を取り違えると、結果に混ざって分かる。

use super::super::super::options::{ConversionOptions, ConversionResult, HisQth, MyQth};
use super::super::*;
use super::support::*;

const MY_PLACES: [MyQth; 3] = [MyQth::Form, MyQth::Rmks1, MyQth::Rmks2];
const HIS_PLACES: [HisQth; 4] = [HisQth::None, HisQth::Qth, HisQth::Rmks1, HisQth::Rmks2];

/// 自局の参照・相手の参照を指定の列に置いた3交信のログ
///
/// - JA1AAA: 相手は山と公園(S2S・P2P)
/// - JA1BBB: 相手の参照なし
/// - JA1CCC: 相手は公園だけ
fn log(my: MyQth, his: HisQth) -> Vec<String> {
    let theirs = [
        ("JA1AAA", "01:00U", "JA/KN-001 JA-0123"),
        ("JA1BBB", "01:01U", ""),
        ("JA1CCC", "01:02U", "JA-0124"),
    ];
    theirs
        .iter()
        .map(|(call, time, his_text)| {
            // [QTH, 備考1, 備考2]
            let mut cells = [DECOY; 3];
            match my {
                MyQth::Rmks1 => cells[1] = "JA/TK-001 JA-0001",
                MyQth::Rmks2 => cells[2] = "JA/TK-001 JA-0001",
                MyQth::Form => {}
            }
            match his {
                HisQth::Qth => cells[0] = his_text,
                HisQth::Rmks1 => cells[1] = his_text,
                HisQth::Rmks2 => cells[2] = his_text,
                HisQth::None => {}
            }
            row(call, time, cells[0], cells[1], cells[2])
        })
        .collect()
}

fn opts(my: MyQth, his: HisQth) -> ConversionOptions {
    ConversionOptions {
        park: vec!["JA-0001".to_string()],
        ..options(my, his)
    }
}

/// 相手の参照が列から読めるか(`none` なら読まない)
fn reads_his(his: HisQth) -> bool {
    his != HisQth::None
}

/// 衝突する組み合わせ(自局と相手が同じ列)。画面では選べるが、別に確かめる
fn collides(my: MyQth, his: HisQth) -> bool {
    matches!(
        (my, his),
        (MyQth::Rmks1, HisQth::Rmks1) | (MyQth::Rmks2, HisQth::Rmks2)
    )
}

#[test]
fn sota_activator_every_column_combination() {
    for my in MY_PLACES {
        for his in HIS_PLACES {
            if collides(my, his) {
                continue;
            }
            let case = format!("sota_a {my:?}/{his:?}");
            let files =
                convert_to_sota_activator(&records(&log(my, his)), "JA1ZZZ/1", &opts(my, his));

            // 自局の山は、どの組み合わせでも JA/TK-001(form なら画面の Summit。列の DECOY は読まない)
            let csv = &files["sota20251018.csv"];
            let lines: Vec<Vec<&str>> = csv.lines().map(cols).collect();
            assert_eq!(lines.len(), 3, "{case}");
            for l in &lines {
                assert_eq!(l[..3], ["V2", "JA1ZZZ/1", "JA/TK-001"], "{case}");
                assert_eq!(l[3], "18/10/2025", "{case}");
                assert_eq!(&l[5..7], ["7MHz", "CW"], "{case}");
            }
            // 相手の山(9列目)。相手の列を指定しないときは空
            let his_summits: Vec<&str> = lines.iter().map(|l| l[8]).collect();
            if reads_his(his) {
                assert_eq!(his_summits, ["JA/KN-001", "", ""], "{case}");
                assert_eq!(files["sota-s2s-20251018.csv"].lines().count(), 1, "{case}");
                assert!(files["sota-s2s-20251018.csv"].contains(",JA1AAA,JA/KN-001,"));
            } else {
                assert_eq!(his_summits, ["", "", ""], "{case}");
                assert!(!files.contains_key("sota-s2s-20251018.csv"), "{case}");
            }

            // ADIF: 自局の山は MY_SOTA_REF、相手の山があれば SOTA_REF
            let adif = adif_records(&files["sota20251018.adi"]);
            assert_eq!(adif.len(), 3, "{case}");
            for r in &adif {
                assert_eq!(r["MY_SOTA_REF"], "JA/TK-001", "{case}");
                assert_eq!(r["STATION_CALLSIGN"], "JA1ZZZ/1", "{case}");
                assert_eq!(r["OPERATOR"], "JA1ZZZ", "{case}");
            }
            let sota_refs: Vec<Option<&str>> = adif
                .iter()
                .map(|r| r.get("SOTA_REF").map(String::as_str))
                .collect();
            let want = if reads_his(his) {
                vec![Some("JA/KN-001"), None, None]
            } else {
                vec![None; 3]
            };
            assert_eq!(sota_refs, want, "{case}");
            assert!(
                !csv.contains("ZZ-999") && !files["sota20251018.adi"].contains("ZZ-999"),
                "{case}: 使わない列の参照が混ざった"
            );
        }
    }
}

#[test]
fn sota_chaser_every_column_combination() {
    // チェイサーは自局の列を読まない(自局の山の列は空)。相手の山だけで、sota と other に分ける
    for my in MY_PLACES {
        for his in HIS_PLACES {
            if collides(my, his) {
                continue;
            }
            let case = format!("sota_c {my:?}/{his:?}");
            let files = convert_to_sota_chaser(&records(&log(my, his)), "JA1ZZZ", &opts(my, his));
            let calls = |name: &str| -> Vec<&str> {
                files
                    .get(name)
                    .map(|b| b.lines().map(|l| cols(l)[7]).collect())
                    .unwrap_or_default()
            };
            if reads_his(his) {
                assert_eq!(calls("sota20251018.csv"), ["JA1AAA"], "{case}");
                assert_eq!(calls("other20251018.csv"), ["JA1BBB", "JA1CCC"], "{case}");
                let line = files["sota20251018.csv"].lines().next().unwrap();
                assert_eq!(line[..].split(',').nth(8), Some("JA/KN-001"), "{case}");
            } else {
                assert!(!files.contains_key("sota20251018.csv"), "{case}");
                assert_eq!(
                    calls("other20251018.csv"),
                    ["JA1AAA", "JA1BBB", "JA1CCC"],
                    "{case}"
                );
            }
            for body in files.values() {
                assert!(
                    body.lines().all(|l| cols(l)[..3] == ["V2", "JA1ZZZ", ""]),
                    "{case}"
                );
                assert!(!body.contains("ZZ-999"), "{case}");
            }
        }
    }
}

#[test]
fn pota_every_column_combination() {
    for my in MY_PLACES {
        for his in HIS_PLACES {
            if collides(my, his) {
                continue;
            }
            let case = format!("pota {my:?}/{his:?}");
            let result = convert_to_adif(&records(&log(my, his)), &opts(my, his));
            assert_eq!(result.status, "OK", "{case}");

            // 公園の記録は、form でも列でも、自局 JA-0001 の1ファイル。
            // 山の記録(JA/TK-001 のファイル)は、列から読んだときだけ。form の Summit は SOTA アクティベーター用で、ADIF には使わない
            let mut names: Vec<&str> = result.files.keys().map(String::as_str).collect();
            names.sort();
            if my == MyQth::Form {
                assert_eq!(names, ["JA1ZZZ-1@JA-0001-20251018.adi"], "{case}");
            } else {
                assert_eq!(
                    names,
                    [
                        "JA1ZZZ-1@JA-0001-20251018.adi",
                        "JA1ZZZ-1@JA-TK-001-20251018.adi"
                    ],
                    "{case}"
                );
            }

            let park = adif_records(&result.files["JA1ZZZ-1@JA-0001-20251018.adi"]);
            let theirs: Vec<Option<&str>> = park
                .iter()
                .map(|r| r.get("SIG_INFO").map(String::as_str))
                .collect();
            let want = if reads_his(his) {
                vec![Some("JA-0123"), None, Some("JA-0124")]
            } else {
                vec![None; 3]
            };
            assert_eq!(theirs, want, "{case}");
            for r in &park {
                assert_eq!(r["MY_SIG"], "POTA", "{case}");
                assert_eq!(r["MY_SIG_INFO"], "JA-0001", "{case}");
                assert_eq!(r["STATION_CALLSIGN"], "JA1ZZZ/1", "{case}");
                assert_eq!(r["OPERATOR"], "JA1ZZZ", "{case}");
                // 公園の記録に、山の項目や RST は入らない
                assert!(!r.contains_key("MY_SOTA_REF") && !r.contains_key("RST_SENT"));
            }

            if my != MyQth::Form {
                let sota = adif_records(&result.files["JA1ZZZ-1@JA-TK-001-20251018.adi"]);
                assert_eq!(sota.len(), 3, "{case}");
                let refs: Vec<Option<&str>> = sota
                    .iter()
                    .map(|r| r.get("SOTA_REF").map(String::as_str))
                    .collect();
                let want = if reads_his(his) {
                    vec![Some("JA/KN-001"), None, None]
                } else {
                    vec![None; 3]
                };
                assert_eq!(refs, want, "{case}");
                assert!(sota.iter().all(|r| r["MY_SOTA_REF"] == "JA/TK-001"));
            }
            for body in result.files.values() {
                assert!(
                    !body.contains("ZZ-999"),
                    "{case}: 使わない列の参照が混ざった"
                );
            }
            // 確認画面の表: 3交信、自局の参照の欄
            assert_eq!(result.log_text.len(), 3, "{case}");
            let my_text = if my == MyQth::Form {
                "JA-0001"
            } else {
                "JA/TK-001/JA-0001"
            };
            assert!(result.log_text.iter().all(|r| r[8] == my_text), "{case}");
        }
    }
}

// 自局と相手が同じ列を指す(rmks1×rmks1、rmks2×rmks2)。1つの欄から自局も相手も読むので、自局の参照が相手の参照にもなる。
// 仕様の確認が要る: 画面でこの組み合わせを選べてよいか(警告を出すか、選べなくするか)
#[test]
fn same_column_for_mine_and_his_reads_the_same_text_twice() {
    for place in [(MyQth::Rmks1, HisQth::Rmks1), (MyQth::Rmks2, HisQth::Rmks2)] {
        let (my, his) = place;
        let (r1, r2) = if my == MyQth::Rmks1 {
            ("JA/TK-001 JA-0001", "")
        } else {
            ("", "JA/TK-001 JA-0001")
        };
        let rows = [row("JA1AAA", "01:00U", "", r1, r2)];
        let case = format!("{place:?}");

        // SOTA アクティベーター: 自分の山と相手の山が同じ → 自己 S2S になる
        let files = convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &opts(my, his));
        let l = cols(files["sota20251018.csv"].lines().next().unwrap()).to_vec();
        assert_eq!((l[2], l[8]), ("JA/TK-001", "JA/TK-001"), "{case}");
        assert_eq!(files["sota-s2s-20251018.csv"], files["sota20251018.csv"]);

        // SOTA チェイサー: 自分の欄の山が相手の山として読まれる
        let files = convert_to_sota_chaser(&records(&rows), "JA1ZZZ", &opts(my, his));
        assert!(files.contains_key("sota20251018.csv"), "{case}");

        // ADIF: 自局の公園が相手の公園(P2P)にもなる
        let result = convert_to_adif(&records(&rows), &opts(my, his));
        let park = adif_records(&result.files["JA1ZZZ-1@JA-0001-20251018.adi"]);
        assert_eq!(park.len(), 1, "{case}");
        assert_eq!(park[0]["MY_SIG_INFO"], "JA-0001");
        assert_eq!(park[0]["SIG_INFO"], "JA-0001", "{case}");
        let sota = adif_records(&result.files["JA1ZZZ-1@JA-TK-001-20251018.adi"]);
        assert_eq!(sota[0]["SOTA_REF"], "JA/TK-001", "{case}");
    }
}

// ---- B-1: Summit(画面で入力する自局の山。sota_a で myQTH=form のとき) ----

/// 交信が1つで、相手の参照はなく、自局の列は使わないログを `Summit` を指定して変換する
fn sota_with_summit(summit: &str) -> std::collections::HashMap<String, String> {
    let rows = [row("JA1AAA", "01:00U", "", "", "")];
    let opts = ConversionOptions {
        summit: summit.to_string(),
        ..options(MyQth::Form, HisQth::Qth)
    };
    convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &opts)
}

#[test]
fn summit_in_the_form_goes_to_the_csv_and_adif() {
    let files = sota_with_summit("JA/TK-001");
    assert_eq!(
        files["sota20251018.csv"],
        "V2,JA1ZZZ/1,JA/TK-001,18/10/2025,01:00,7MHz,CW,JA1AAA,, \n"
    );
    let adif = adif_records(&files["sota20251018.adi"]);
    assert_eq!(adif[0]["MY_SOTA_REF"], "JA/TK-001");
}

/// 他の協会の山(`W7W/LC-001` など)もそのまま使える
#[test]
fn summit_of_another_association_is_accepted() {
    let files = sota_with_summit("W7W/LC-001");
    assert!(files["sota20251018.csv"].starts_with("V2,JA1ZZZ/1,W7W/LC-001,"));
}

/// 画面で小文字や前後の空白つきで入れた Summit を、そのまま CSV に書く。
/// SOTA の CSV の山の参照は大文字で空白なしなので、列から読んだ参照と同じに整える(大文字・trim)べき
#[test]
fn summit_in_the_form_is_trimmed_and_uppercased() {
    for input in ["ja/tk-001", " JA/TK-001 ", "\tja/tk-001\n"] {
        let files = sota_with_summit(input);
        assert!(
            files["sota20251018.csv"].starts_with("V2,JA1ZZZ/1,JA/TK-001,18/10/2025"),
            "{input:?}: {:?}",
            files["sota20251018.csv"]
        );
        let adif = adif_records(&files["sota20251018.adi"]);
        assert_eq!(adif[0]["MY_SOTA_REF"], "JA/TK-001", "{input:?}");
    }
}

/// 誤った形の Summit(`ABC`、`JA-TK-001`、全角)。service は形を検べず、そのまま CSV に書く。
/// 仕様の確認が要る: 誤った形は 400 で断るべきか(いまは api にも検査がない)
#[test]
fn malformed_summit_is_written_as_is() {
    for input in ["ABC", "JA-TK-001", "JA/TK-001-"] {
        let files = sota_with_summit(input);
        let first = files["sota20251018.csv"]
            .lines()
            .next()
            .unwrap()
            .to_string();
        assert_eq!(cols(&first)[2], input, "{input}");
    }
}

/// Summit が空で myQTH=form のとき、自局の山がないので、交信は1つも出ない(CSV も ADIF も作らない)
#[test]
fn empty_summit_in_the_form_produces_nothing() {
    for input in ["", "   "] {
        let files = sota_with_summit(input);
        // 空白だけの Summit も、空とみなす(前後の空白を除くため)
        assert!(files.is_empty(), "{input:?}: {files:?}");
    }
}

/// myQTH=form のとき、列の山は無視する。myQTH=rmks1 のとき、Summit は無視する
#[test]
fn summit_and_the_column_do_not_mix() {
    let rows = [row("JA1AAA", "01:00U", "", "JA/KN-777", "")];
    let form = ConversionOptions {
        summit: "JA/TK-001".to_string(),
        ..options(MyQth::Form, HisQth::None)
    };
    let files = convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &form);
    assert!(files["sota20251018.csv"].contains(",JA/TK-001,"));
    assert!(!files["sota20251018.csv"].contains("JA/KN-777"));

    let column = ConversionOptions {
        summit: "JA/TK-001".to_string(),
        ..options(MyQth::Rmks1, HisQth::None)
    };
    let files = convert_to_sota_activator(&records(&rows), "JA1ZZZ/1", &column);
    assert!(files["sota20251018.csv"].contains(",JA/KN-777,"));
    assert!(!files["sota20251018.csv"].contains("JA/TK-001"));
}

// ---- B-2: Park(画面で入力する自局の公園。pota で myQTH=form のとき。API は文字列1つを渡す) ----

fn pota_with_park(park: &str) -> ConversionResult {
    let rows = [row("JA1AAA", "01:00U", "", "", "")];
    let opts = ConversionOptions {
        park: if park.is_empty() {
            vec![]
        } else {
            vec![park.to_string()]
        },
        ..options(MyQth::Form, HisQth::Qth)
    };
    convert_to_adif(&records(&rows), &opts)
}

fn file_names(result: &ConversionResult) -> Vec<&str> {
    let mut names: Vec<&str> = result.files.keys().map(String::as_str).collect();
    names.sort();
    names
}

#[test]
fn park_in_the_form_one_or_many() {
    // 空白・カンマ・両方の区切りのどれでも同じ。POTA(JA-)と JAFF の混在、4桁と5桁
    for input in [
        "JA-0001 JA-0002",
        "JA-0001,JA-0002",
        "JA-0001, JA-0002",
        "JA-0001\u{3000}JA-0002",
    ] {
        let result = pota_with_park(input);
        assert_eq!(
            file_names(&result),
            [
                "JA1ZZZ-1@JA-0001-20251018.adi",
                "JA1ZZZ-1@JA-0002-20251018.adi"
            ],
            "{input:?}"
        );
        assert_eq!(result.file_list.len(), 2, "{input:?}");
    }

    let result = pota_with_park("JAFF-0005 JA-0014 US-10995");
    assert_eq!(
        file_names(&result),
        [
            "JA1ZZZ-1@JA-0014-20251018.adi",
            "JA1ZZZ-1@JAFF-0005-20251018.adi",
            "JA1ZZZ-1@US-10995-20251018.adi",
        ]
    );
    let wwff = adif_records(&result.files["JA1ZZZ-1@JAFF-0005-20251018.adi"]);
    assert_eq!(
        (wwff[0]["MY_SIG"].as_str(), wwff[0]["MY_SIG_INFO"].as_str()),
        ("WWFF", "JAFF-0005")
    );
    // WWFF の記録は RST を持ち、POTA の記録は持たない(旧版と同じ)
    assert!(wwff[0].contains_key("RST_SENT"));
    let pota = adif_records(&result.files["JA1ZZZ-1@US-10995-20251018.adi"]);
    assert_eq!(pota[0]["MY_SIG_INFO"], "US-10995");
    assert!(!pota[0].contains_key("RST_SENT"));
}

#[test]
fn park_in_the_form_is_uppercased() {
    let result = pota_with_park("ja-0001 jaff-0002");
    assert_eq!(
        file_names(&result),
        [
            "JA1ZZZ-1@JA-0001-20251018.adi",
            "JA1ZZZ-1@JAFF-0002-20251018.adi"
        ]
    );
}

/// 誤った形・空の Park では、ファイルを作らない(交信は表に残るが、自局の参照が空)。status は OK のまま。
/// 仕様の確認が要る: 自局の参照が空なら、確認の画面で警告するか
#[test]
fn malformed_or_empty_park_produces_no_file() {
    for input in ["", "ABC", "JA-1", "JA-123456", "山頂"] {
        let result = pota_with_park(input);
        assert!(
            result.files.is_empty(),
            "{input:?}: {:?}",
            result.files.keys()
        );
        assert_eq!(result.status, "OK", "{input:?}");
        assert_eq!(result.log_text[0][8], "", "{input:?}");
    }
}

/// 同じ公園を2回書くと、同じ記録が2つ入る。仕様の確認が要る(重複は1つにするべきか)
#[test]
fn duplicated_park_in_the_form_is_written_twice() {
    let result = pota_with_park("JA-0001 JA-0001");
    let park = adif_records(&result.files["JA1ZZZ-1@JA-0001-20251018.adi"]);
    assert_eq!(park.len(), 2);
}

/// myQTH=form のとき列の公園は無視、列を指定したとき Park は無視
#[test]
fn park_and_the_column_do_not_mix() {
    let rows = [row("JA1AAA", "01:00U", "", "JA-0777", "")];
    let form = ConversionOptions {
        park: vec!["JA-0001".to_string()],
        ..options(MyQth::Form, HisQth::Qth)
    };
    let r = convert_to_adif(&records(&rows), &form);
    assert_eq!(file_names(&r), ["JA1ZZZ-1@JA-0001-20251018.adi"]);

    let column = ConversionOptions {
        park: vec!["JA-0001".to_string()],
        ..options(MyQth::Rmks1, HisQth::Qth)
    };
    let r = convert_to_adif(&records(&rows), &column);
    assert_eq!(file_names(&r), ["JA1ZZZ-1@JA-0777-20251018.adi"]);
}

// ---- B-4: コールサイン(SOTAActivator・POTAActivator・POTAOperator) ----

fn sota_station(callsign: &str) -> (String, std::collections::BTreeMap<String, String>) {
    let rows = [row("JA1AAA", "01:00U", "", "JA/TK-001", "")];
    let files = convert_to_sota_activator(
        &records(&rows),
        callsign,
        &options(MyQth::Rmks1, HisQth::None),
    );
    let csv = files["sota20251018.csv"].clone();
    let adif = adif_records(&files["sota20251018.adi"]).remove(0);
    (cols(csv.lines().next().unwrap())[1].to_string(), adif)
}

#[test]
fn sota_activator_callsign_with_suffixes() {
    // (入力, CSV の自局, STATION_CALLSIGN, OPERATOR)
    let cases = [
        ("JA1ZZZ", "JA1ZZZ", "JA1ZZZ", "JA1ZZZ"),
        ("JA1ZZZ/1", "JA1ZZZ/1", "JA1ZZZ/1", "JA1ZZZ"),
        ("JA1ZZZ/P", "JA1ZZZ/P", "JA1ZZZ/P", "JA1ZZZ"),
        ("JA1ZZZ/QRP", "JA1ZZZ/QRP", "JA1ZZZ/QRP", "JA1ZZZ"),
        ("JA1ZZZ/7/P", "JA1ZZZ/7/P", "JA1ZZZ/7/P", "JA1ZZZ"),
        ("W7/JA1ZZZ", "W7/JA1ZZZ", "W7/JA1ZZZ", "JA1ZZZ"),
    ];
    for (input, csv, station, operator) in cases {
        let (got_csv, adif) = sota_station(input);
        assert_eq!(got_csv, csv, "{input}");
        assert_eq!(adif["STATION_CALLSIGN"], station, "{input}");
        assert_eq!(adif["OPERATOR"], operator, "{input}");
    }
}

/// 小文字・全角・前後の空白のコールサインは、半角の大文字にそろえる(CSV の自局・STATION_CALLSIGN・OPERATOR とも)
#[test]
fn sota_activator_callsign_is_normalized() {
    for input in [
        "ja1zzz/1",
        " ja1zzz/1\t",
        "ｊａ１ｚｚｚ／１",
        "ＪＡ１ＺＺＺ/１",
    ] {
        let (csv, adif) = sota_station(input);
        assert_eq!(csv, "JA1ZZZ/1", "{input:?}");
        assert_eq!(adif["STATION_CALLSIGN"], "JA1ZZZ/1", "{input:?}");
        assert_eq!(adif["OPERATOR"], "JA1ZZZ", "{input:?}");
    }
}

/// 空のコールサインでも変換は進み、CSV の自局の列が空になる(service は検査しない)。
/// 仕様の確認が要る: 空・誤った形は 400 で断るべきか(いまは api にも検査がない)
#[test]
fn sota_activator_empty_or_malformed_callsign_is_written_as_is() {
    let (csv, adif) = sota_station("");
    assert_eq!(csv, "");
    // 値が空の項目(`<STATION_CALLSIGN:0>`)は、読み取りでは項目なしになる
    assert!(!adif.contains_key("STATION_CALLSIGN") && !adif.contains_key("OPERATOR"));

    // コンマを含むコールサインは、引用符で囲んで、列がずれない
    let rows = [row("JA1AAA", "01:00U", "", "JA/TK-001", "")];
    let files = convert_to_sota_activator(
        &records(&rows),
        "JA1ZZZ,1",
        &options(MyQth::Rmks1, HisQth::None),
    );
    assert!(files["sota20251018.csv"].starts_with("V2,\"JA1ZZZ,1\",JA/TK-001,"));
}

fn pota_station(activator: &str, operator: &str) -> ConversionResult {
    let rows = [row("JA1AAA", "01:00U", "", "JA-0001", "")];
    let opts = ConversionOptions {
        pota_activator: activator.to_string(),
        pota_operator: operator.to_string(),
        ..options(MyQth::Rmks1, HisQth::None)
    };
    convert_to_adif(&records(&rows), &opts)
}

#[test]
fn pota_operator_is_taken_from_the_activator_when_omitted() {
    // (運用局, 運用者の指定, ファイル名の運用局, OPERATOR)
    let cases = [
        ("JA1ZZZ/1", "", "JA1ZZZ-1", "JA1ZZZ"),
        ("JA1ZZZ/P", "", "JA1ZZZ-P", "JA1ZZZ"),
        ("JA1ZZZ", "", "JA1ZZZ", "JA1ZZZ"),
        ("JA1ZZZ/1", "JA1YYY", "JA1ZZZ-1", "JA1YYY"),
        ("JA1ZZZ/QRP", "", "JA1ZZZ-QRP", "JA1ZZZ"),
        ("W7/JA1ZZZ", "", "W7-JA1ZZZ", "JA1ZZZ"),
    ];
    for (activator, operator, in_name, want_operator) in cases {
        let result = pota_station(activator, operator);
        let name = format!("{in_name}@JA-0001-20251018.adi");
        let body = result
            .files
            .get(&name)
            .unwrap_or_else(|| panic!("{activator}: {:?}", result.files.keys()));
        let r = &adif_records(body)[0];
        assert_eq!(r["STATION_CALLSIGN"], activator);
        assert_eq!(r["OPERATOR"], want_operator, "{activator}/{operator}");
        // 確認画面の表の運用局・運用者
        assert_eq!(result.log_text[0][9], activator);
        assert_eq!(result.log_text[0][10], want_operator);
    }
}

/// 小文字・全角・前後の空白の運用局と運用者は、大文字にそろえる(STATION_CALLSIGN・OPERATOR・ファイル名・確認の表とも)
#[test]
fn pota_callsigns_are_normalized() {
    let result = pota_station(" ja1zzz/1 ", "");
    let body = &result.files["JA1ZZZ-1@JA-0001-20251018.adi"];
    let r = &adif_records(body)[0];
    assert_eq!(r["STATION_CALLSIGN"], "JA1ZZZ/1");
    assert_eq!(r["OPERATOR"], "JA1ZZZ");
    assert_eq!(result.log_text[0][9], "JA1ZZZ/1");

    let result = pota_station("ＪＡ１ＺＺＺ/１", " ja1yyy ");
    let r = &adif_records(&result.files["JA1ZZZ-1@JA-0001-20251018.adi"])[0];
    assert_eq!(r["STATION_CALLSIGN"], "JA1ZZZ/1");
    assert_eq!(r["OPERATOR"], "JA1YYY");
}

/// 運用局が空のとき、ファイル名は `@JA-0001-…` になり、STATION_CALLSIGN は空。
/// 仕様の確認が要る: 空は 400 で断るべきか(いまは api にも検査がない)
#[test]
fn pota_empty_activator_makes_a_nameless_file() {
    let result = pota_station("", "");
    let r = &adif_records(&result.files["@JA-0001-20251018.adi"])[0];
    assert!(!r.contains_key("STATION_CALLSIGN"));
    assert!(!r.contains_key("OPERATOR"));
}
