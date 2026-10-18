//! SOTA アクティベーター(CSV・S2S の CSV・ADIF)と、SOTA チェイサー(CSV)

use super::super::adif::{adif_field, AdifField};
use super::super::callsign::split_callsign;
use super::super::options::{ConversionOptions, HisQth};
use super::super::record::QsoRecord;
use super::super::reference::{get_ref, RefInfo};
use super::files::{adif_header, csv_line, FileSet};
use super::refs::{date_key, his_ref, line_of, my_ref_for_sota};
use std::collections::HashMap;

/// 読み取りに失敗した行を集めるファイル(日付のファイルは、そのまま SOTA にアップロードするので、混ぜない)
const ERROR_FILE: &str = "sota000000.csv";

/// SOTA の CSV の1行(V2 形式)
struct SotaRow {
    /// 交信の日付(`YYYYMMDD`)。出力ファイルの名前になる
    date: String,
    /// 相手にも SOTA の参照がある(アクティベーターでは S2S、チェイサーでは山との交信)
    his_summit: bool,
    fields: Vec<String>,
}

impl SotaRow {
    /// `V2,自局,自局の山,日付,時刻,バンド,モード,相手,相手の山,場所の注記`
    fn new(
        qso: &QsoRecord,
        callsign: &str,
        my_summit: &str,
        his_summit: &str,
        note_loc: &str,
    ) -> Self {
        Self {
            date: date_key(qso),
            his_summit: !his_summit.is_empty(),
            fields: vec![
                "V2".to_string(),
                callsign.to_string(),
                my_summit.to_string(),
                format!("{:02}/{:02}/{:02}", qso.day, qso.month, qso.year),
                format!("{:02}:{:02}", qso.hour, qso.minute),
                qso.band_sota.clone(),
                qso.mode_sota.clone(),
                qso.callsign.clone(),
                his_summit.to_string(),
                format!("{note_loc} "),
            ],
        }
    }

    fn line(&self) -> String {
        csv_line(&self.fields) + "\n"
    }
}

/// 読み取りに失敗した行を、`sota000000.csv` に書く1行にする
fn error_line(qso: &QsoRecord, index: usize) -> String {
    let message = format!(
        "HamLog format error at Line {}. : {}",
        line_of(qso, index),
        qso.error_message()
    );
    let mut fields = vec![message];
    fields.extend(std::iter::repeat_n(String::new(), 9));
    csv_line(&fields) + "\n"
}

/// アクティベーターの行。自局の山がない交信は None
fn activator_row(qso: &QsoRecord, callsign: &str, options: &ConversionOptions) -> Option<SotaRow> {
    let my = my_ref_for_sota(qso, options);
    if my.sota.is_empty() {
        return None;
    }
    let his = his_ref(qso, options.his_qth);
    // 最後の列は、相手の参照に付いた場所(座標)の注記。相手を指定しないときは、空白1つ(旧版と同じ)
    let note_loc = match options.his_qth {
        HisQth::None => " ",
        _ => &his.loc,
    };
    Some(SotaRow::new(qso, callsign, &my.sota, &his.sota, note_loc))
}

/// チェイサーの行
fn chaser_row(qso: &QsoRecord, callsign: &str, options: &ConversionOptions) -> SotaRow {
    let his = his_ref(qso, options.his_qth);
    // 最後の列は、場所(座標)の注記。旧版は、相手の参照を置いていない側の備考から取る
    // (相手を QTH に置いたときは備考1。指定しないときは、空白1つ)
    let note_loc = match options.his_qth {
        HisQth::Rmks1 => get_ref(&qso.remarks2).loc,
        HisQth::Rmks2 | HisQth::Qth => get_ref(&qso.remarks1).loc,
        HisQth::None => " ".to_string(),
    };
    SotaRow::new(qso, callsign, "", &his.sota, &note_loc)
}

/// SOTA アクティベーターの ADIF の1行。読み取りに失敗した行と、自局の山がない交信は None
///
/// 旧版と同じく、項目は空白で区切り、最後に `<EOR>`。
fn adif_line(
    qso: &QsoRecord,
    callsign: &str,
    options: &ConversionOptions,
) -> Option<(String, String)> {
    if qso.has_error() {
        return None;
    }
    let my = my_ref_for_sota(qso, options);
    if my.sota.is_empty() {
        return None;
    }
    // 旧版の ADIF は、相手の参照を備考1・備考2からだけ読む
    let his = match options.his_qth {
        HisQth::Rmks1 | HisQth::Rmks2 => his_ref(qso, options.his_qth),
        HisQth::Qth | HisQth::None => RefInfo::default(),
    };

    let date = date_key(qso);
    let (operator, _) = split_callsign(callsign);
    let mut fields = vec![
        adif_field(AdifField::StationCallsign, callsign),
        adif_field(AdifField::Operator, &operator),
        adif_field(AdifField::Call, &qso.callsign),
        adif_field(AdifField::QsoDate, &date),
        adif_field(
            AdifField::TimeOn,
            &format!("{:02}{:02}", qso.hour, qso.minute),
        ),
        adif_field(AdifField::Band, &qso.band_wlen),
        adif_field(AdifField::Mode, &qso.mode),
        adif_field(AdifField::RstSent, &qso.rst_sent),
        adif_field(AdifField::RstRcvd, &qso.rst_rcvd),
        adif_field(AdifField::MySotaRef, &my.sota),
    ];
    if !his.sota.is_empty() {
        fields.push(adif_field(AdifField::SotaRef, &his.sota));
    }
    fields.push("<EOR>".to_string());
    Some((date, fields.join(" ")))
}

/// ADIF のファイルを作る。1つのファイルに全交信(ファイル名は、最初の交信の日付)
fn adif_file(
    records: &[QsoRecord],
    callsign: &str,
    options: &ConversionOptions,
) -> Option<(String, String)> {
    let mut name = None;
    let mut body = String::new();
    for (date, line) in records
        .iter()
        .filter_map(|qso| adif_line(qso, callsign, options))
    {
        name.get_or_insert_with(|| format!("sota{date}.adi"));
        body.push_str(&line);
        body.push('\n');
    }
    name.map(|name| (name, adif_header("3.0.6") + &body))
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
    let mut files = FileSet::default();

    for (index, qso) in records.iter().enumerate() {
        if qso.has_error() {
            files.append(ERROR_FILE, &error_line(qso, index));
            continue;
        }
        let Some(row) = activator_row(qso, callsign, options) else {
            continue;
        };
        let line = row.line();
        files.append(format!("sota{}.csv", row.date), &line);
        if row.his_summit {
            files.append(format!("sota-s2s-{}.csv", row.date), &line);
        }
    }

    if let Some((name, body)) = adif_file(records, callsign, options) {
        files.insert(name, body);
    }
    files.into_files()
}

/// Convert QSO records to SOTA chaser format
///
/// 相手局の SOTA 参照番号がある交信を日付ごとの CSV(`sotaYYYYMMDD.csv`)に、ない交信を
/// `otherYYYYMMDD.csv` にする(旧版と同じ)。読み取りに失敗した行は、`sota000000.csv` に集める。
pub fn convert_to_sota_chaser(
    records: &[QsoRecord],
    callsign: &str,
    options: &ConversionOptions,
) -> HashMap<String, String> {
    let mut files = FileSet::default();

    for (index, qso) in records.iter().enumerate() {
        if qso.has_error() {
            files.append(ERROR_FILE, &error_line(qso, index));
            continue;
        }
        let row = chaser_row(qso, callsign, options);
        let prefix = if row.his_summit { "sota" } else { "other" };
        files.append(format!("{prefix}{}.csv", row.date), &row.line());
    }
    files.into_files()
}
