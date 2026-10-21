//! 自局の参照ごとの ADIF(POTA・WWFF・SOTA)と、確認画面に出す表

use super::super::adif::{adif_field, AdifField};
use super::super::callsign::{normalize_callsign, split_callsign};
use super::super::options::{ConversionOptions, ConversionResult};
use super::super::record::QsoRecord;
use super::super::reference::RefInfo;
use super::files::{adif_header, FileSet};
use super::refs::{date_key, error_log, his_ref, my_ref_for_adif};

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
fn station(options: &ConversionOptions) -> (String, String) {
    let activator = normalize_callsign(&options.pota_activator);
    let operator = normalize_callsign(&options.pota_operator);
    let operator = if operator.is_empty() {
        split_callsign(&activator).0
    } else {
        operator
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

/// 運用局のコールサインと運用者
struct Station {
    activator: String,
    operator: String,
}

/// 項目のうち、自局・相手の参照に依らない部分
struct QsoFields {
    /// 公園(POTA)の記録に使う(RST は持たない)
    base: Vec<String>,
    /// SOTA と WWFF の記録に使う(RST を持つ)
    with_rst: Vec<String>,
}

impl QsoFields {
    fn new(qso: &QsoRecord, station: &Station, date: &str, time: &str) -> Self {
        let base = vec![
            adif_field(AdifField::StationCallsign, &station.activator),
            adif_field(AdifField::Operator, &station.operator),
            adif_field(AdifField::Call, &qso.callsign),
            adif_field(AdifField::QsoDate, date),
            adif_field(AdifField::TimeOn, time),
            adif_field(AdifField::Band, &qso.band_wlen),
            adif_field(AdifField::Mode, &qso.mode),
        ];
        let mut with_rst = base.clone();
        with_rst.push(adif_field(AdifField::RstSent, &qso.rst_sent));
        with_rst.push(adif_field(AdifField::RstRcvd, &qso.rst_rcvd));
        Self { base, with_rst }
    }
}

/// SOTA の記録(自局の山が1つ)。山がなければ None
fn sota_records(fields: &QsoFields, my: &RefInfo, his: &RefInfo) -> Option<(String, String)> {
    if my.sota.is_empty() {
        return None;
    }
    let theirs = if his.sota.is_empty() {
        vec![]
    } else {
        vec![vec![adif_field(AdifField::SotaRef, &his.sota)]]
    };
    let mine = vec![adif_field(AdifField::MySotaRef, &my.sota)];
    Some((my.sota.clone(), records_for(&fields.with_rst, mine, theirs)))
}

/// 公園(POTA)の記録(自局の公園ごと)と、所在地(MY_STATE。複数あれば最後のもの)
fn park_records(
    fields: &QsoFields,
    my: &RefInfo,
    his: &RefInfo,
    options: &ConversionOptions,
) -> (Vec<(String, String)>, Option<String>) {
    let theirs: Vec<Vec<String>> = his
        .pota
        .iter()
        .map(|h| {
            vec![
                adif_field(AdifField::Sig, "POTA"),
                adif_field(AdifField::SigInfo, h),
            ]
        })
        .collect();

    let mut state = None;
    let mut records = Vec::new();
    for park in &my.pota {
        let mut mine = vec![
            adif_field(AdifField::MySig, "POTA"),
            adif_field(AdifField::MySigInfo, park),
        ];
        // 所在地が分かる公園は、MY_STATE を入れる
        if let Some(s) = options.park_states.get(park) {
            mine.push(adif_field(AdifField::MyState, s));
            state = Some(s.clone());
        }
        records.push((
            park.clone(),
            records_for(&fields.base, mine, theirs.clone()),
        ));
    }
    (records, state)
}

/// WWFF の記録(自局の WWFF ごと)
fn wwff_records(fields: &QsoFields, my: &RefInfo, his: &RefInfo) -> Vec<(String, String)> {
    let theirs: Vec<Vec<String>> = his
        .wwff
        .iter()
        .map(|h| {
            vec![
                adif_field(AdifField::Sig, "WWFF"),
                adif_field(AdifField::SigInfo, h),
            ]
        })
        .collect();
    my.wwff
        .iter()
        .map(|wwff| {
            let mine = vec![
                adif_field(AdifField::MySig, "WWFF"),
                adif_field(AdifField::MySigInfo, wwff),
            ];
            (
                wwff.clone(),
                records_for(&fields.with_rst, mine, theirs.clone()),
            )
        })
        .collect()
}

/// 確認画面の表の1行
fn display_row(
    qso: &QsoRecord,
    station: &Station,
    date: &str,
    time: &str,
    refs: (&RefInfo, &RefInfo, Option<&str>),
) -> Vec<String> {
    let (my, his, my_state) = refs;
    let mut my_text = refs_text(my);
    if let Some(state) = my_state {
        my_text.push_str(&format!("({state})"));
    }
    let mode = if qso.sub_mode.is_empty() {
        qso.mode.clone()
    } else {
        format!("{}/{}", qso.mode, qso.sub_mode)
    };
    vec![
        qso.callsign.clone(),
        date.to_string(),
        time.to_string(),
        qso.band_wlen.clone(),
        mode,
        qso.rst_sent.clone(),
        qso.rst_rcvd.clone(),
        refs_text(his),
        my_text,
        station.activator.clone(),
        station.operator.clone(),
    ]
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
    let station = Station {
        activator,
        operator,
    };
    let fields = QsoFields::new(qso, &station, &date, &time);

    let (park, my_state) = park_records(&fields, &my, &his, options);
    // ファイルの並び: 山、公園、WWFF
    let records = sota_records(&fields, &my, &his)
        .into_iter()
        .chain(park)
        .chain(wwff_records(&fields, &my, &his))
        .collect();

    Ok(AdifQso {
        display: display_row(
            qso,
            &station,
            &date,
            &time,
            (&my, &his, my_state.as_deref()),
        ),
        date,
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
        safe_name_part(activator),
        safe_name_part(reference),
        date
    )
}

/// ファイル名に入れる部分。区切り(`/`、`\\`)、Windows で使えない字、制御文字は `-` にする
///
/// ZIP を展開したときに、保存先の外を指す名前(`..\\..\\x`)や、ディレクトリになる名前にならないようにする。
fn safe_name_part(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect()
}

fn is_park(reference: &str) -> bool {
    reference.contains("JA-") || reference.contains("JP-")
}

/// Convert QSO records to ADIF format and return file contents
///
/// 読み取りに失敗した交信は、`status` を `NG` にして、`error_log` に行番号つきで残す(ADIF には入れない)。
pub fn convert_to_adif(records: &[QsoRecord], options: &ConversionOptions) -> ConversionResult {
    adif_result(records, options, true)
}

/// 確認(画面の表と誤りの一覧)だけ。ADIF のファイルは作らない(`files` は空。`file_list` は入る)
pub fn check_adif(records: &[QsoRecord], options: &ConversionOptions) -> ConversionResult {
    adif_result(records, options, false)
}

fn adif_result(
    records: &[QsoRecord],
    options: &ConversionOptions,
    with_files: bool,
) -> ConversionResult {
    let mut result = ConversionResult::default();
    let mut files = FileSet::default();
    let header = adif_header("3.1.4");
    let (activator, _) = station(options);

    // 最初の日付は、読み取りに成功した最初の交信のもの
    let first_date = records
        .iter()
        .find(|r| !r.has_error())
        .map(date_key)
        .unwrap_or_default();

    result.error_log = error_log(records);
    for qso in records {
        let q = match adif_qso(qso, options) {
            Ok(q) => q,
            Err(reason) => {
                result.status = "NG".to_string();
                result.log_text.push(vec![reason]);
                continue;
            }
        };
        result.log_text.push(q.display);

        for (reference, record) in &q.records {
            let name = file_name(&activator, reference, &q.date, &first_date);
            if with_files {
                files.append_with_header(name.clone(), &header, record);
            }
            if is_park(reference) && !result.file_list.contains(&name) {
                result.file_list.push(name);
            }
        }
    }

    result.files = files.into_files();
    result
}
