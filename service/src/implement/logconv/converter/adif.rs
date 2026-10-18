//! 自局の参照ごとの ADIF(POTA・WWFF・SOTA)と、確認画面に出す表

use super::super::adif::{adif_field, AdifField};
use super::super::callsign::split_callsign;
use super::super::options::{ConversionOptions, ConversionResult};
use super::super::record::QsoRecord;
use super::super::reference::RefInfo;
use super::files::{adif_header, FileSet};
use super::refs::{date_key, his_ref, line_of, my_ref_for_adif};

/// 1交信の ADIF
struct AdifQso {
    /// 交信の日付(`YYYYMMDD`)
    date: String,
    /// 確認画面の表の1行
    display: Vec<String>,
    /// 自局の参照ごとの記録(ファイル名の素になる参照 → ADIF の記録)
    records: Vec<(String, String)>,
}

/// 運用局のコールサインと、運用者(指定がなければ、運用局のコールサインから取る)
fn station(options: &ConversionOptions) -> (&str, String) {
    let activator = options.pota_activator.as_str();
    let operator = if options.pota_operator.is_empty() {
        split_callsign(activator).0
    } else {
        options.pota_operator.clone()
    };
    (activator, operator)
}

/// 自局の参照1つについて、記録を作る。相手の参照の数だけ記録を作る(相手がなければ1件)
///
/// 相手が複数の公園(n-fer)のときは、組み合わせごとの別の記録になる(旧版と同じ)。
fn records_for(base: &[String], mine: Vec<String>, theirs: Vec<Vec<String>>) -> String {
    let record = |his: &[String]| {
        let fields: String = base
            .iter()
            .chain(&mine)
            .chain(his)
            .map(String::as_str)
            .collect();
        fields + "<EOR>\n"
    };
    if theirs.is_empty() {
        record(&[])
    } else {
        theirs.iter().map(|his| record(his)).collect()
    }
}

/// 確認画面に出す、参照の表記(`JA/TK-001/JAFF-0001/JA-0001`)
fn refs_text(r: &RefInfo) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if !r.sota.is_empty() {
        parts.push(&r.sota);
    }
    parts.extend(r.wwff.iter().map(String::as_str));
    parts.extend(r.pota.iter().map(String::as_str));
    parts.join("/")
}

/// 1交信の ADIF を作る。読み取りに失敗した交信は、その理由
fn adif_qso(qso: &QsoRecord, options: &ConversionOptions) -> Result<AdifQso, String> {
    if qso.has_error() {
        return Err(qso.error_message());
    }

    let my = my_ref_for_adif(qso, options);
    let his = his_ref(qso, options.his_qth);
    let date = date_key(qso);
    let time = format!("{:02}{:02}", qso.hour, qso.minute);
    let (activator, operator) = station(options);

    // 公園(POTA)の記録は RST を持たない。SOTA と WWFF の記録は RST を持つ
    let base = vec![
        adif_field(AdifField::StationCallsign, activator),
        adif_field(AdifField::Operator, &operator),
        adif_field(AdifField::Call, &qso.callsign),
        adif_field(AdifField::QsoDate, &date),
        adif_field(AdifField::TimeOn, &time),
        adif_field(AdifField::Band, &qso.band_wlen),
        adif_field(AdifField::Mode, &qso.mode),
    ];
    let mut with_rst = base.clone();
    with_rst.push(adif_field(AdifField::RstSent, &qso.rst_sent));
    with_rst.push(adif_field(AdifField::RstRcvd, &qso.rst_rcvd));

    let mut records: Vec<(String, String)> = Vec::new();

    // SOTA
    if !my.sota.is_empty() {
        let theirs = if his.sota.is_empty() {
            vec![]
        } else {
            vec![vec![adif_field(AdifField::SotaRef, &his.sota)]]
        };
        let mine = vec![adif_field(AdifField::MySotaRef, &my.sota)];
        records.push((my.sota.clone(), records_for(&with_rst, mine, theirs)));
    }

    // POTA
    let mut my_state = String::new();
    for park in &my.pota {
        let theirs = his
            .pota
            .iter()
            .map(|h| {
                vec![
                    adif_field(AdifField::Sig, "POTA"),
                    adif_field(AdifField::SigInfo, h),
                ]
            })
            .collect();
        let mut mine = vec![
            adif_field(AdifField::MySig, "POTA"),
            adif_field(AdifField::MySigInfo, park),
        ];
        // 所在地が分かる公園は、MY_STATE を入れる
        if let Some(state) = options.park_states.get(park) {
            mine.push(adif_field(AdifField::MyState, state));
            my_state = state.clone();
        }
        records.push((park.clone(), records_for(&base, mine, theirs)));
    }

    // WWFF
    for wwff in &my.wwff {
        let theirs = his
            .wwff
            .iter()
            .map(|h| {
                vec![
                    adif_field(AdifField::Sig, "WWFF"),
                    adif_field(AdifField::SigInfo, h),
                ]
            })
            .collect();
        let mine = vec![
            adif_field(AdifField::MySig, "WWFF"),
            adif_field(AdifField::MySigInfo, wwff),
        ];
        records.push((wwff.clone(), records_for(&with_rst, mine, theirs)));
    }

    let mut my_text = refs_text(&my);
    if !my_state.is_empty() {
        my_text.push_str(&format!("({my_state})"));
    }
    let mode = if qso.sub_mode.is_empty() {
        qso.mode.clone()
    } else {
        format!("{}/{}", qso.mode, qso.sub_mode)
    };
    let display = vec![
        qso.callsign.clone(),
        date.clone(),
        time,
        qso.band_wlen.clone(),
        mode,
        qso.rst_sent.clone(),
        qso.rst_rcvd.clone(),
        refs_text(&his),
        my_text,
        activator.to_string(),
        operator,
    ];
    Ok(AdifQso {
        date,
        display,
        records,
    })
}

/// ADIF のファイル名: `運用局@参照-日付.adi`
///
/// 日付は、山(`/` を含む参照)ではその交信の日付、公園と WWFF では最初の日付(旧版と同じ)。
fn file_name(activator: &str, reference: &str, qso_date: &str, first_date: &str) -> String {
    let date = if is_park(reference) || !reference.contains('/') {
        first_date
    } else {
        qso_date
    };
    format!(
        "{}@{}-{}.adi",
        activator.replace('/', "-"),
        reference.replace('/', "-"),
        date
    )
}

fn is_park(reference: &str) -> bool {
    reference.contains("JA-") || reference.contains("JP-")
}

/// Convert QSO records to ADIF format and return file contents
///
/// 読み取りに失敗した交信は、`status` を `NG` にして、`error_log` に行番号つきで残す(ADIF には入れない)。
pub fn convert_to_adif(records: &[QsoRecord], options: &ConversionOptions) -> ConversionResult {
    let mut result = ConversionResult::default();
    let mut files = FileSet::default();
    let header = adif_header("3.1.4");

    // 最初の日付は、読み取りに成功した最初の交信のもの
    let first_date = records
        .iter()
        .find(|r| !r.has_error())
        .map(date_key)
        .unwrap_or_default();

    for (index, qso) in records.iter().enumerate() {
        let q = match adif_qso(qso, options) {
            Ok(q) => q,
            Err(reason) => {
                result.status = "NG".to_string();
                result
                    .error_log
                    .push(format!("{}行:{}", line_of(qso, index), reason));
                result.log_text.push(vec![reason]);
                continue;
            }
        };
        result.log_text.push(q.display);

        for (reference, record) in &q.records {
            let name = file_name(&options.pota_activator, reference, &q.date, &first_date);
            files.append_with_header(name.clone(), &header, record);
            if is_park(reference) && !result.file_list.contains(&name) {
                result.file_list.push(name);
            }
        }
    }

    result.files = files.into_files();
    result
}
