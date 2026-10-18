//! Log format converters - converts QSO records to various output formats
//!
//! Supports:
//! - SOTA CSV format (activator/chaser)
//! - POTA ADIF format
//! - WWFF ADIF format
//! - AirHam CSV format

use super::types::{adif_field, get_ref, ConversionOptions, ConversionResult, QsoRecord, RefInfo};
use std::collections::HashMap;
use std::io::Write;
use zip::write::SimpleFileOptions;

const ADIF_HEADER: &str = "ADIF Export from HAMLOG by JL1NIE\n";
const ADIF_VERSION: &str = "3.1.4";
const PROGRAM_ID: &str = "FCTH";

/// Generate SOTA CSV row for activator log
pub fn to_sota_activator(
    qso: &QsoRecord,
    callsign: &str,
    options: &ConversionOptions,
) -> Option<(String, bool, Vec<String>)> {
    if qso.has_error {
        let row = vec![
            format!("HamLog format error at Line: {}", qso.error_message),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        let date = format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day);
        return Some((date, false, row));
    }

    let my_ref = match options.my_qth.as_str() {
        "rmks1" => get_ref(&qso.remarks1),
        "rmks2" => get_ref(&qso.remarks2),
        _ => RefInfo {
            sota: options.summit.clone(),
            ..Default::default()
        },
    };

    let his_ref = match options.his_qth.as_str() {
        "rmks1" => get_ref(&qso.remarks1),
        "rmks2" => get_ref(&qso.remarks2),
        "qth" => get_ref(&qso.qth),
        _ => RefInfo::default(),
    };

    let comment = match options.his_qth.as_str() {
        "rmks1" => get_ref(&qso.remarks2),
        "rmks2" => get_ref(&qso.remarks1),
        _ => his_ref.clone(),
    };

    let date = format!("{:02}/{:02}/{:02}", qso.day, qso.month, qso.year);
    let date2 = format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day);

    // Skip if no SOTA reference
    if my_ref.sota.is_empty() {
        return Some((date2, true, vec![]));
    }

    let is_s2s = !his_ref.sota.is_empty();

    let row = vec![
        "V2".to_string(),
        callsign.to_string(),
        my_ref.sota.clone(),
        date,
        format!("{:02}:{:02}", qso.hour, qso.minute),
        qso.band_sota.clone(),
        qso.mode_sota.clone(),
        qso.callsign.clone(),
        his_ref.sota.clone(),
        format!("{} ", comment.loc),
    ];

    Some((date2, is_s2s, row))
}

/// Generate SOTA CSV row for chaser log
pub fn to_sota_chaser(
    qso: &QsoRecord,
    callsign: &str,
    options: &ConversionOptions,
) -> Option<(String, bool, Vec<String>)> {
    if qso.has_error {
        let row = vec![
            format!("HamLog format error at Line: {}", qso.error_message),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        let date = format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day);
        return Some((date, false, row));
    }

    let his_ref = match options.his_qth.as_str() {
        "rmks1" => get_ref(&qso.remarks1),
        "rmks2" => get_ref(&qso.remarks2),
        "qth" => get_ref(&qso.qth),
        _ => RefInfo::default(),
    };

    let date = format!("{:02}/{:02}/{:02}", qso.day, qso.month, qso.year);
    let date2 = format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day);

    let is_sota = !his_ref.sota.is_empty();

    let row = vec![
        "V2".to_string(),
        callsign.to_string(),
        String::new(), // my summit (empty for chaser)
        date,
        format!("{:02}:{:02}", qso.hour, qso.minute),
        qso.band_sota.clone(),
        qso.mode_sota.clone(),
        qso.callsign.clone(),
        his_ref.sota.clone(),
        format!("{} ", his_ref.loc),
    ];

    Some((date2, is_sota, row))
}

/// Generate ADIF records for POTA/SOTA/WWFF
pub fn to_adif2(
    qso: &QsoRecord,
    options: &ConversionOptions,
) -> (
    String,
    Vec<String>,
    HashMap<String, Vec<String>>,
    Option<String>,
) {
    if qso.has_error || !qso.band_error.is_empty() {
        return (
            String::new(),
            vec![qso.error_message.clone()],
            HashMap::new(),
            Some(qso.error_message.clone()),
        );
    }

    let my_ref = match options.my_qth.as_str() {
        "rmks1" => get_ref(&qso.remarks1),
        "rmks2" => get_ref(&qso.remarks2),
        _ => RefInfo {
            pota: options.park.clone(),
            ..Default::default()
        },
    };

    let his_ref = match options.his_qth.as_str() {
        "rmks1" => get_ref(&qso.remarks1),
        "rmks2" => get_ref(&qso.remarks2),
        "qth" => get_ref(&qso.qth),
        _ => RefInfo::default(),
    };

    let date = if qso.date_error.is_empty() {
        format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day)
    } else {
        qso.date_error.clone()
    };

    let time = if qso.time_error.is_empty() {
        format!("{:02}{:02}", qso.hour, qso.minute)
    } else {
        qso.time_error.clone()
    };

    let activator = &options.pota_activator;
    let operator = if options.pota_operator.is_empty() {
        let (op, _) = super::types::split_callsign(activator);
        op
    } else {
        options.pota_operator.clone()
    };

    // Build base QSO fields
    let mut base_fields = vec![
        adif_field("activator", activator),
        adif_field("operator", &operator),
        adif_field("callsign", &qso.callsign),
        adif_field("date", &date),
        adif_field("time", &time),
        adif_field("band-wlen", &qso.band_wlen),
    ];

    let disp_mode = if qso.sub_mode.is_empty() {
        qso.mode.clone()
    } else {
        format!("{}/{}", qso.mode, qso.sub_mode)
    };

    base_fields.push(adif_field("mode", &qso.mode));

    let mut qso_fields = base_fields.clone();
    qso_fields.push(adif_field("rst_sent", &qso.rst_sent));
    qso_fields.push(adif_field("rst_rcvd", &qso.rst_rcvd));

    let mut log: HashMap<String, Vec<String>> = HashMap::new();

    // SOTA references
    let my_sota: Vec<String> = if my_ref.sota.is_empty() {
        vec![]
    } else {
        vec![my_ref.sota.clone()]
    };

    let his_sota: Vec<String> = if his_ref.sota.is_empty() {
        vec![]
    } else {
        vec![his_ref.sota.clone()]
    };

    // 自局の参照ごとに、相手の参照の数だけ記録を作る(相手がなければ1件)。
    // 相手が複数の公園(n-fer)のときは、組み合わせごとの別の記録になる(旧版と同じ)
    let records = |base: &Vec<String>, my_fields: Vec<String>, his: Vec<Vec<String>>| {
        let mut out: Vec<String> = Vec::new();
        let mut push = |extra: Vec<String>| {
            out.extend(base.iter().cloned());
            out.extend(my_fields.iter().cloned());
            out.extend(extra);
            out.push("<EOR>\n".to_string());
        };
        if his.is_empty() {
            push(Vec::new());
        } else {
            for h in his {
                push(h);
            }
        }
        out
    };

    for my in &my_sota {
        let his = his_sota
            .iter()
            .map(|h| vec![adif_field("sotaref", h)])
            .collect();
        log.insert(
            my.clone(),
            records(&qso_fields, vec![adif_field("mysotaref", my)], his),
        );
    }

    // POTA references
    let mut my_state = String::new();
    for my in &my_ref.pota {
        let his = his_ref
            .pota
            .iter()
            .map(|h| vec![adif_field("sig", "POTA"), adif_field("siginfo", h)])
            .collect();
        let mut my_fields = vec![adif_field("mysig", "POTA"), adif_field("mysiginfo", my)];
        // 所在地が分かる公園は、MY_STATE を入れる
        if let Some(state) = options.park_states.get(my) {
            my_fields.push(adif_field("mystate", state));
            my_state = state.clone();
        }
        log.insert(my.clone(), records(&base_fields, my_fields, his));
    }

    // WWFF references
    for my in &my_ref.wwff {
        let his = his_ref
            .wwff
            .iter()
            .map(|h| vec![adif_field("sig", "WWFF"), adif_field("siginfo", h)])
            .collect();
        log.insert(
            my.clone(),
            records(
                &qso_fields,
                vec![adif_field("mysig", "WWFF"), adif_field("mysiginfo", my)],
                his,
            ),
        );
    }

    // Build display row
    let make_str = |r: &RefInfo| {
        let mut parts = vec![];
        if !r.sota.is_empty() {
            parts.push(r.sota.clone());
        }
        parts.extend(r.wwff.clone());
        parts.extend(r.pota.clone());
        parts.join("/")
    };

    let his_str = make_str(&his_ref);
    let mut my_str = make_str(&my_ref);
    if !my_state.is_empty() {
        my_str.push_str(&format!("({})", my_state));
    }

    let disp = vec![
        qso.callsign.clone(),
        date.clone(),
        time.clone(),
        qso.band_wlen.clone(),
        disp_mode,
        qso.rst_sent.clone(),
        qso.rst_rcvd.clone(),
        his_str,
        my_str,
        activator.clone(),
        operator,
    ];

    (date, disp, log, None)
}

/// Generate AirHam CSV row
pub fn to_airham(qso: &QsoRecord, options: &ConversionOptions, is_header: bool) -> Vec<String> {
    if is_header {
        return vec![
            "id".to_string(),
            "callsign".to_string(),
            "portable".to_string(),
            "qso_at".to_string(),
            "sent_rst".to_string(),
            "received_rst".to_string(),
            "sent_qth".to_string(),
            "received_qth".to_string(),
            "received_qra".to_string(),
            "frequency".to_string(),
            "mode".to_string(),
            "card".to_string(),
            "remarks".to_string(),
        ];
    }

    if qso.has_error {
        return vec![
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            format!("HamLog format error: {}", qso.error_message),
        ];
    }

    let (my_qth, comment) = match options.my_qth.as_str() {
        "rmks1" => (qso.remarks1.clone(), qso.remarks2.clone()),
        "rmks2" => (qso.remarks2.clone(), qso.remarks1.clone()),
        "user_defined" => (options.location.clone(), qso.remarks1.clone()),
        _ => (String::new(), String::new()),
    };

    vec![
        String::new(),
        qso.operator.clone(),
        qso.portable.clone(),
        qso.iso_time.clone(),
        qso.rst_sent.clone(),
        qso.rst_rcvd.clone(),
        my_qth,
        qso.qth.clone(),
        qso.name.clone(),
        qso.band.clone(),
        qso.mode_airham.clone(),
        qso.qsl.clone(),
        comment,
    ]
}

/// Convert QSO records to ADIF format and return file contents
pub fn convert_to_adif(records: &[QsoRecord], options: &ConversionOptions) -> ConversionResult {
    let mut result = ConversionResult::default();
    let mut files: HashMap<String, String> = HashMap::new();
    let mut pota_files: Vec<String> = Vec::new();

    let header = format!(
        "{}{}\n{}\n<EOH>\n",
        ADIF_HEADER,
        adif_field("programid", PROGRAM_ID),
        adif_field("adifver", ADIF_VERSION)
    );

    let act_call = &options.pota_activator;
    let first_date = if let Some(first) = records.first() {
        format!("{:04}{:02}{:02}", first.year, first.month, first.day)
    } else {
        String::new()
    };

    for (idx, qso) in records.iter().enumerate() {
        let (date, disp, log, error) = to_adif2(qso, options);

        if let Some(err) = error {
            result.status = "NG".to_string();
            result.error_log.push(format!("{}行:{}", idx + 1, err));
        }

        if !disp.is_empty() {
            result.log_text.push(disp);
        }

        for (ref_key, fields) in log {
            let is_pota = ref_key.starts_with("JA-") || ref_key.starts_with("JP-");
            let use_date = if is_pota || ref_key.contains('/') {
                &first_date
            } else {
                &date
            };

            let filename = format!(
                "{}@{}-{}.adi",
                act_call.replace('/', "-"),
                ref_key.replace('/', "-"),
                use_date
            );

            let entry = files
                .entry(filename.clone())
                .or_insert_with(|| header.clone());
            entry.push_str(&fields.join(""));

            if is_pota && !pota_files.contains(&filename) {
                pota_files.push(filename);
            }
        }
    }

    result.files = files;
    result.file_list = pota_files;
    result
}

/// SOTA アクティベーターの ADIF の1行(`sotaYYYYMMDD.adi`)。自局の山がない行は None
///
/// 旧版と同じく、項目は空白で区切り、最後に `<EOR>`。
fn to_sota_adif(
    qso: &QsoRecord,
    callsign: &str,
    options: &ConversionOptions,
) -> Option<(String, String)> {
    let date = format!("{:04}{:02}{:02}", qso.year, qso.month, qso.day);
    // 読み取りに失敗した行は、ADIF に入れない(sota000000.csv に集める)
    if qso.has_error {
        return None;
    }

    let ref_of = |source: &str, own: Option<RefInfo>| match source {
        "rmks1" => get_ref(&qso.remarks1),
        "rmks2" => get_ref(&qso.remarks2),
        _ => own.unwrap_or_default(),
    };
    let my_ref = ref_of(
        options.my_qth.as_str(),
        Some(RefInfo {
            sota: options.summit.clone(),
            ..Default::default()
        }),
    );
    if my_ref.sota.is_empty() {
        return None;
    }
    // 旧版の ADIF は、相手の参照を備考1・備考2からだけ読む
    let his_ref = match options.his_qth.as_str() {
        "rmks1" | "rmks2" => ref_of(options.his_qth.as_str(), None),
        _ => RefInfo::default(),
    };

    let (operator, _) = super::types::split_callsign(callsign);
    let mut fields = vec![
        adif_field("activator", callsign),
        adif_field("operator", &operator),
        adif_field("callsign", &qso.callsign),
        adif_field("date", &date),
        adif_field("time", &format!("{:02}{:02}", qso.hour, qso.minute)),
        adif_field("band-wlen", &qso.band_wlen),
        adif_field("mode", &qso.mode),
        adif_field("rst_sent", &qso.rst_sent),
        adif_field("rst_rcvd", &qso.rst_rcvd),
        adif_field("mysotaref", &my_ref.sota),
    ];
    if !his_ref.sota.is_empty() {
        fields.push(adif_field("sotaref", &his_ref.sota));
    }
    fields.push("<EOR>".to_string());
    Some((date, fields.join(" ")))
}

/// Convert QSO records to SOTA activator format
///
/// 日付ごとに `sotaYYYYMMDD.csv` を作り、相手にも SOTA 参照番号がある交信(S2S)は
/// `sota-s2s-YYYYMMDD.csv` にも入れる。日付の順に並んでいないログや、読み取りに失敗した行
/// (日付が不正)が間に入っていても、交信が消えないよう、ファイル名ごとに足していく。
pub fn convert_to_sota_activator(
    records: &[QsoRecord],
    callsign: &str,
    options: &ConversionOptions,
) -> HashMap<String, String> {
    let mut files: HashMap<String, String> = HashMap::new();

    for (index, qso) in records.iter().enumerate() {
        // 読み取りに失敗した行は、日付のファイルに混ぜず、別のファイル(sota000000.csv)に集める
        // (日付のファイルは、そのまま SOTA にアップロードするため)
        if qso.has_error {
            let message = format!(
                "HamLog format error at Line {}. : {}",
                index + 1,
                qso.error_message
            );
            let row = std::iter::once(message.as_str())
                .chain(std::iter::repeat_n("", 9))
                .collect::<Vec<_>>();
            files
                .entry("sota000000.csv".to_string())
                .or_default()
                .push_str(&(row.join(",") + "\n"));
            continue;
        }
        let Some((date, is_s2s, row)) = to_sota_activator(qso, callsign, options) else {
            continue;
        };
        if row.is_empty() {
            continue;
        }

        let line = row.join(",") + "\n";
        files
            .entry(format!("sota{}.csv", date))
            .or_default()
            .push_str(&line);
        if is_s2s {
            files
                .entry(format!("sota-s2s-{}.csv", date))
                .or_default()
                .push_str(&line);
        }
    }

    // ADIF は、1つのファイルに全交信(ファイル名は、最初の交信の日付)
    let mut adif_name: Option<String> = None;
    let mut adif_body = String::new();
    for qso in records {
        if let Some((date, line)) = to_sota_adif(qso, callsign, options) {
            adif_name.get_or_insert_with(|| format!("sota{}.adi", date));
            adif_body.push_str(&line);
            adif_body.push('\n');
        }
    }
    if let Some(name) = adif_name {
        let header = format!(
            "ADIF Export from HAMLOG by JL1NIE\n{}\n{}\n<EOH>\n",
            adif_field("programid", PROGRAM_ID),
            adif_field("adifver", "3.0.6")
        );
        files.insert(name, header + &adif_body);
    }

    files
}

/// Convert QSO records to SOTA chaser format
///
/// 相手局の SOTA 参照番号がある交信を日付ごとの CSV(`sotaYYYYMMDD.csv`)に、ない交信を
/// `otherYYYYMMDD.csv` にする(旧版と同じ)。読み取りに失敗した行は、`other` の側に入れる。
pub fn convert_to_sota_chaser(
    records: &[QsoRecord],
    callsign: &str,
    options: &ConversionOptions,
) -> HashMap<String, String> {
    let mut files: HashMap<String, String> = HashMap::new();

    for qso in records {
        let Some((date, is_sota, row)) = to_sota_chaser(qso, callsign, options) else {
            continue;
        };
        let prefix = if is_sota { "sota" } else { "other" };
        files
            .entry(format!("{}{}.csv", prefix, date))
            .or_default()
            .push_str(&(row.join(",") + "\n"));
    }

    files
}

/// Create a ZIP file from a hashmap of filename -> content
pub fn create_zip(files: &HashMap<String, String>) -> Result<Vec<u8>, std::io::Error> {
    let mut buffer = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (name, content) in files {
            zip.start_file(name, options)?;
            zip.write_all(content.as_bytes())?;
        }
        zip.finish()?;
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_qso() -> QsoRecord {
        QsoRecord {
            has_error: false,
            callsign: "JL1NIE".to_string(),
            operator: "JL1NIE".to_string(),
            portable: String::new(),
            iso_time: "2024-12-28T12:34:00+00:00".to_string(),
            year: 2024,
            month: 12,
            day: 28,
            hour: 12,
            minute: 34,
            timezone: "+0000".to_string(),
            rst_sent: "599".to_string(),
            rst_rcvd: "559".to_string(),
            band: "7MHz".to_string(),
            band_sota: "7MHz".to_string(),
            band_wlen: "40m".to_string(),
            mode: "CW".to_string(),
            mode_sota: "CW".to_string(),
            mode_airham: "CW".to_string(),
            remarks1: "JA/TK-001".to_string(),
            qth: "JA/KN-001".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_to_sota_activator() {
        let qso = make_qso();
        let options = ConversionOptions {
            my_qth: "rmks1".to_string(),
            his_qth: "qth".to_string(),
            ..Default::default()
        };

        let result = to_sota_activator(&qso, "JL1NIE", &options);
        assert!(result.is_some());

        let (date, is_s2s, row) = result.unwrap();
        assert_eq!(date, "20241228");
        assert!(is_s2s); // JA/KN-001 in qth
        assert_eq!(row[0], "V2");
        assert_eq!(row[2], "JA/TK-001");
        assert_eq!(row[8], "JA/KN-001");
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
            my_qth: "rmks1".to_string(),
            his_qth: "qth".to_string(),
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
            his_qth: "qth".to_string(),
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
            his_qth: "qth".to_string(),
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
            his_qth: "qth".to_string(),
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

    const NFER: &str = include_str!("../../../../test_data/logconv/hamlog_nfer.csv");
    const NFER_RMKS2: &str = include_str!("../../../../test_data/logconv/hamlog_nfer_rmks2.csv");
    const JST_ERRORS: &str = include_str!("../../../../test_data/logconv/hamlog_jst_errors.csv");

    fn rmks1_qth() -> ConversionOptions {
        ConversionOptions {
            my_qth: "rmks1".to_string(),
            his_qth: "qth".to_string(),
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
            his_qth: "rmks2".to_string(),
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
            include_str!(concat!("../../../../test_data/logconv/legacy/", $path))
        };
    }

    #[test]
    fn legacy_parity_sota_activator() {
        let options = ConversionOptions {
            my_qth: "rmks1".to_string(),
            his_qth: "rmks2".to_string(),
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
        let csv = include_str!("../../../../test_data/logconv/hamlog_ios.csv");
        let mut records = Vec::new();
        let mut header_seen = false;
        for line in csv.lines() {
            let cols: Vec<String> = line.split(',').map(String::from).collect();
            if let Ok(qso) = decode_auto(&cols, header_seen) {
                records.push(qso);
            }
            header_seen = true;
        }
        assert_eq!(records.len(), 5);

        let options = ConversionOptions {
            my_qth: "rmks1".to_string(),
            his_qth: "rmks2".to_string(),
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
    fn test_to_airham_header() {
        let qso = QsoRecord::default();
        let options = ConversionOptions::default();
        let row = to_airham(&qso, &options, true);
        assert_eq!(row[0], "id");
        assert_eq!(row[1], "callsign");
    }

    #[test]
    fn test_to_airham_data() {
        let qso = make_qso();
        let options = ConversionOptions {
            my_qth: "rmks1".to_string(),
            ..Default::default()
        };
        let row = to_airham(&qso, &options, false);
        assert_eq!(row[1], "JL1NIE");
        assert_eq!(row[6], "JA/TK-001");
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
}
