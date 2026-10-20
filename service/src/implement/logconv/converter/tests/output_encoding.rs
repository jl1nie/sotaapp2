//! 出力の文字コード: ADIF の `<TAG:n>` の長さ(バイト数)、日本語が出る所と出ない所、ZIP の中のファイル名
//!
//! 変換の出力は `String`(UTF-8)で、ZIP の中も UTF-8 のバイト列。日本語の名前・QTH・備考は、どの出力にも出さない。
//! 日本語(や全角)が入りうるのは、利用者が入力した値をそのまま使う所: 相手のコールサイン・RST・
//! 画面で入力した `Summit`・コールサイン(運用局・運用者)と、それが入るファイル名。

use super::super::super::options::{ConversionOptions, HisQth, MyQth};
use super::super::*;
use super::support::*;
use std::collections::HashMap;
use std::io::Read;

/// ADIF の本文の `<TAG:n>` が、すべてバイト数として正しいか(値の直後が `<`・空白・改行・終わりのどれか)
fn assert_adif_lengths_are_bytes(body: &str) {
    let bytes = body.as_bytes();
    let mut pos = 0;
    let mut tags = 0;
    while let Some(start) = body[pos..].find('<').map(|i| pos + i) {
        let close = body[start..].find('>').map(|i| start + i).unwrap();
        let tag = &body[start + 1..close];
        pos = close + 1;
        let Some((_, len)) = tag.split_once(':') else {
            continue; // <EOH> <EOR>
        };
        let len: usize = len.parse().unwrap();
        let end = close + 1 + len;
        assert!(end <= bytes.len(), "<{tag}> が本文より長い");
        assert!(
            body.is_char_boundary(end),
            "<{tag}> の長さが文字の途中で終わる"
        );
        assert!(
            end == bytes.len() || matches!(bytes[end], b'<' | b' ' | b'\n'),
            "<{tag}> の長さが値と合わない: 次は {:?}",
            &body[end..(end + 8).min(body.len())]
        );
        pos = end;
        tags += 1;
    }
    assert!(tags > 0);
}

fn zip_entries(zip: &[u8]) -> HashMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut f = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            (f.name().to_string(), data)
        })
        .collect()
}

// ---- ADIF の項目の長さはバイト数 ----

#[test]
fn adif_field_length_counts_bytes_not_characters() {
    use super::super::super::adif::{adif_field, AdifField};
    assert_eq!(adif_field(AdifField::Call, "JA1AAA"), "<CALL:6>JA1AAA");
    // 全角3文字は9バイト
    assert_eq!(adif_field(AdifField::Call, "あいう"), "<CALL:9>あいう");
    assert_eq!(adif_field(AdifField::RstSent, "５９"), "<RST_SENT:6>５９");
    // 値が空でも項目は出す
    assert_eq!(adif_field(AdifField::RstSent, ""), "<RST_SENT:0>");
}

/// 日本語・全角が入りうる項目(相手のコールサイン・RST・運用局・運用者)を入れても、全項目の長さがバイト数として合う
/// (ADIF を読むソフトが、項目の境目を取り違えない)。仕様の確認が要る: ADIF の仕様が ADI の非 ASCII を認めるか、
/// 長さを文字数とするソフトがないか(POTA・WWFF・SOTA の取り込み)。ASCII だけにする(検査で断る)方が安全
#[test]
fn adif_lengths_stay_correct_with_japanese_values() {
    let line = "あいう1,25/10/18,01:00U,５９,５９,7.030,CW,,,N,,,JA/TK-001 JA-0001 JAFF-0001,,";
    let recs = fixture_line(line);
    let opts = ConversionOptions {
        pota_activator: "ＪＡ１ＺＺＺ/１".to_string(),
        pota_operator: "山田太郎".to_string(),
        ..options(MyQth::Rmks1, HisQth::Qth)
    };

    let result = convert_to_adif(&recs, &opts);
    assert_eq!(result.files.len(), 3);
    for (name, body) in &result.files {
        assert_adif_lengths_are_bytes(body);
        // 読み戻すと、元の値になる
        let r = &adif_records(body)[0];
        assert_eq!(r["CALL"], "あいう1", "{name}");
        assert_eq!(r["OPERATOR"], "山田太郎", "{name}");
        assert_eq!(r["STATION_CALLSIGN"], "ＪＡ１ＺＺＺ/１", "{name}");
    }
    assert!(result.files["ＪＡ１ＺＺＺ-１@JA-0001-20251018.adi"].contains("<OPERATOR:12>山田太郎"));

    // SOTA アクティベーターの ADIF も同じ
    let files = convert_to_sota_activator(&recs, "ＪＡ１ＺＺＺ/１", &opts);
    assert_adif_lengths_are_bytes(&files["sota20251018.adi"]);
    assert!(files["sota20251018.adi"].contains("<RST_SENT:6>５９"));
}

/// 画面で入力した Summit に全角が入っても、MY_SOTA_REF の長さはバイト数で合う(値の検査は別。`summit_in_the_form_*`)
#[test]
fn adif_length_of_a_fullwidth_summit_is_in_bytes() {
    let recs = fixture_line("JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,,,");
    let opts = ConversionOptions {
        summit: "ＪＡ/ＴＫ-００１".to_string(),
        ..options(MyQth::Form, HisQth::Qth)
    };
    let files = convert_to_sota_activator(&recs, "JA1ZZZ/1", &opts);
    assert_adif_lengths_are_bytes(&files["sota20251018.adi"]);
    assert!(files["sota20251018.adi"].contains("<MY_SOTA_REF:"));
}

// ---- 日本語の名前・QTH・備考は、どの出力にも出さない ----

#[test]
fn japanese_name_qth_and_remarks_never_reach_the_outputs() {
    // 名前・QTH(参照つき)・備考1(参照つき)・備考2が日本語
    let line =
        "JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,山田 太郎,東京都千代田区 JA/KN-001 JA-0123,\
                高尾山 JA/TK-001 JA-0001 JAFF-0001,良く聞こえました,";
    let recs = fixture_line(line);
    let opts = options(MyQth::Rmks1, HisQth::Qth);

    let mut outputs: Vec<(String, String)> = Vec::new();
    outputs.extend(convert_to_sota_activator(&recs, "JA1ZZZ/1", &opts));
    outputs.extend(convert_to_sota_chaser(&recs, "JA1ZZZ", &opts));
    outputs.extend(convert_to_adif(&recs, &opts).files);
    assert!(outputs.len() >= 6);
    for (name, body) in &outputs {
        assert!(name.is_ascii(), "{name}");
        assert!(body.is_ascii(), "{name}: ASCII でない文字が出た: {body}");
    }
}

// ---- SOTA の CSV で日本語が出る列: 誤りの行のファイル(sota000000.csv)の1列目 ----

/// 誤りの行の理由は日本語で、`sota000000.csv` の1列目に出る。UTF-8(BOM なし)。
/// 仕様の確認が要る: このファイルは SOTA にアップロードするものではなく、利用者が読むもの。
/// Excel は BOM なしの UTF-8 を文字化けして開くため、BOM をつけるべきか
#[test]
fn error_file_has_japanese_reason_in_utf8_without_bom() {
    let recs = fixture_line("JA2CCC,25/10/19,99:99J,59,59,14.285,SSB,,,N,,,JA/TK-001,,");
    let files = convert_to_sota_activator(&recs, "JA1ZZZ/1", &options(MyQth::Rmks1, HisQth::Qth));
    let body = &files["sota000000.csv"];
    assert!(body.starts_with("HamLog format error at Line 1. : 日時不正:25/10/19 99:99J,"));
    assert!(!body.starts_with('\u{feff}'));

    let zip = create_zip(&files).unwrap();
    let entries = zip_entries(&zip);
    let raw = &entries["sota000000.csv"];
    assert_eq!(std::str::from_utf8(raw).unwrap(), body);
    assert_ne!(&raw[..3], [0xEF, 0xBB, 0xBF]);
    // 10列(1列目が理由、あとは空)
    assert_eq!(body.trim_end().matches(',').count(), 9);
}

// ---- ZIP の中のファイル名 ----

#[test]
fn zip_keeps_japanese_file_names_and_marks_them_utf8() {
    let mut files = HashMap::new();
    files.insert(
        "山田@JA-0001-20251018.adi".to_string(),
        "日本語の中身\n".to_string(),
    );
    files.insert("plain.adi".to_string(), "ascii\n".to_string());
    let zip = create_zip(&files).unwrap();

    let entries = zip_entries(&zip);
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries["山田@JA-0001-20251018.adi"],
        "日本語の中身\n".as_bytes()
    );
    assert_eq!(entries["plain.adi"], b"ascii\n");

    // ローカルファイルヘッダ(PK\x03\x04)の汎用フラグの bit 11 = 名前が UTF-8。日本語の名前の方だけ立つ
    let mut flags = HashMap::new();
    let mut i = 0;
    while let Some(p) = zip[i..].windows(4).position(|w| w == [0x50, 0x4b, 3, 4]) {
        let h = i + p;
        let flag = u16::from_le_bytes([zip[h + 6], zip[h + 7]]);
        let name_len = u16::from_le_bytes([zip[h + 26], zip[h + 27]]) as usize;
        let name = String::from_utf8(zip[h + 30..h + 30 + name_len].to_vec()).unwrap();
        flags.insert(name, flag & 0x0800 != 0);
        i = h + 4;
    }
    assert!(flags["山田@JA-0001-20251018.adi"]);
    assert!(!flags["plain.adi"]);
}

/// 運用局のコールサインがファイル名になる。`/` は `-` に置き換える(ZIP の中にディレクトリができない)
#[test]
fn zip_names_from_the_activator_have_no_directories() {
    let recs = fixture_line("JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,");
    let opts = ConversionOptions {
        pota_activator: "JA1ZZZ/1/P".to_string(),
        ..options(MyQth::Rmks1, HisQth::Qth)
    };
    let result = convert_to_adif(&recs, &opts);
    let entries = zip_entries(&create_zip(&result.files).unwrap());
    let names: Vec<&String> = entries.keys().collect();
    assert_eq!(names, ["JA1ZZZ-1-P@JA-0001-20251018.adi"]);
}

/// 日本語の運用局でも、ファイル名は ZIP に入り、読み戻せる
#[test]
fn zip_name_from_a_japanese_activator_round_trips() {
    let recs = fixture_line("JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,");
    let opts = ConversionOptions {
        pota_activator: "山田".to_string(),
        ..options(MyQth::Rmks1, HisQth::Qth)
    };
    let result = convert_to_adif(&recs, &opts);
    let entries = zip_entries(&create_zip(&result.files).unwrap());
    assert!(entries.contains_key("山田@JA-0001-20251018.adi"));
}

/// 運用局に `\` や `..` を入れても、ZIP の中の名前がディレクトリを指さない(展開したときに、保存先の外へ書かない)。
/// `/` は置き換えるが、`\` はそのまま名前に入り、Windows で展開すると区切りとして読まれる
#[test]
#[ignore = "不具合: 運用局の `\\` をファイル名に残す(ZIP の名前に `..\\` が入り、Windows で展開すると保存先の外を指す)"]
fn zip_names_never_contain_backslash_or_dot_dot_components() {
    let recs = fixture_line("JA1AAA,25/10/18,01:00U,59,59,7.030,CW,,,N,,,JA-0001,,");
    let opts = ConversionOptions {
        pota_activator: r"..\..\evil".to_string(),
        ..options(MyQth::Rmks1, HisQth::Qth)
    };
    let result = convert_to_adif(&recs, &opts);
    let entries = zip_entries(&create_zip(&result.files).unwrap());
    for name in entries.keys() {
        assert!(!name.contains('\\'), "{name}");
        assert!(!name.split(['/', '\\']).any(|p| p == ".."), "{name}");
    }
}
