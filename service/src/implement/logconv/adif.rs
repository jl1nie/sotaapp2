//! ADIF (Amateur Data Interchange Format)
//!
//! 入力: ADIF 3.x のファイルを読んで、交信(`QsoRecord`)にする。
//! 出力: ADIF の項目(`<CALL:6>JA1ABC`)の書式。

use super::bands::freq_to_band;
use super::modes::mode_to_adif_mode;
use super::record::{QsoRecord, RecordError};
use std::collections::HashMap;

/// ADIF の項目
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdifField {
    /// 運用局のコールサイン
    StationCallsign,
    Operator,
    /// 相手局のコールサイン
    Call,
    QsoDate,
    TimeOn,
    Band,
    Mode,
    RstSent,
    RstRcvd,
    MySig,
    MySigInfo,
    MyState,
    Sig,
    SigInfo,
    SotaRef,
    MySotaRef,
    ProgramId,
    AdifVer,
}

impl AdifField {
    /// ADIF のタグ名
    pub fn tag(self) -> &'static str {
        match self {
            Self::StationCallsign => "STATION_CALLSIGN",
            Self::Operator => "OPERATOR",
            Self::Call => "CALL",
            Self::QsoDate => "QSO_DATE",
            Self::TimeOn => "TIME_ON",
            Self::Band => "BAND",
            Self::Mode => "MODE",
            Self::RstSent => "RST_SENT",
            Self::RstRcvd => "RST_RCVD",
            Self::MySig => "MY_SIG",
            Self::MySigInfo => "MY_SIG_INFO",
            Self::MyState => "MY_STATE",
            Self::Sig => "SIG",
            Self::SigInfo => "SIG_INFO",
            Self::SotaRef => "SOTA_REF",
            Self::MySotaRef => "MY_SOTA_REF",
            Self::ProgramId => "PROGRAMID",
            Self::AdifVer => "ADIF_VER",
        }
    }
}

/// ADIF の1項目(`<CALL:6>JA1ABC`)。値が空でも項目は出す(旧版と同じ。RST のない交信は `<RST_SENT:0>`)
pub fn adif_field(field: AdifField, value: &str) -> String {
    format!("<{}:{}>{}", field.tag(), value.len(), value)
}

/// ADIF の記録の項目(タグ名は大文字)
pub type AdifFields = HashMap<String, String>;

/// 値のバイト列を文字列にする関数(ファイルの文字コードで読む)
pub type ValueDecoder<'a> = &'a dyn Fn(&[u8]) -> String;

fn utf8_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// 大文字・小文字を区別せず、バイト列の中から `needle`(大文字の ASCII)を探す
fn find_ascii_ci(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
}

/// ADIF のバイト列を、記録(交信1つ分の項目)に分ける
///
/// - `<FIELD:n>` の n は、元のファイルの文字コードでのバイト数。バイト列のまま読み、値ごとに `decode` で文字列にする
///   (文字列にしてから読むと、Shift_JIS のファイルで長さが合わない)。
/// - ヘッダー(`<EOH>` まで)は読み飛ばす。`<EOH>` はバイト列の中で大文字小文字を区別せず探す。
/// - 最後の記録に `<EOR>` がなくても、項目があれば記録として返す。`CALL` がない記録も返す(呼び出し側が誤りにする)。
pub fn parse_adif_bytes(content: &[u8], decode: ValueDecoder) -> Vec<AdifFields> {
    let start = find_ascii_ci(content, b"<EOH>").map_or(0, |p| p + 5);
    scan_records(&content[start..], decode)
}

fn scan_records(bytes: &[u8], decode: ValueDecoder) -> Vec<AdifFields> {
    let mut records = Vec::new();
    let mut fields = AdifFields::new();
    let mut pos = 0;
    let end = bytes.len();
    // `from` 以降で最初の `target` の位置(なければ末尾)
    let find = |from: usize, targets: &[u8]| {
        bytes[from.min(end)..]
            .iter()
            .position(|b| targets.contains(b))
            .map_or(end, |p| from + p)
    };

    while pos < end {
        pos = find(pos, b"<");
        if pos >= end {
            break;
        }
        pos += 1; // '<'

        let name_end = find(pos, b">:");
        if name_end >= end {
            break;
        }
        let tag = String::from_utf8_lossy(&bytes[pos..name_end]).to_uppercase();
        pos = name_end;

        if tag == "EOR" || tag == "EOH" {
            pos = find(pos, b">") + 1;
            if tag == "EOR" && !fields.is_empty() {
                records.push(std::mem::take(&mut fields));
            }
            continue;
        }

        let mut length = 0usize;
        if bytes[pos] == b':' {
            pos += 1;
            let len_end = find(pos, b">:");
            length = std::str::from_utf8(&bytes[pos..len_end])
                .ok()
                .and_then(|t| t.trim().parse::<usize>().ok())
                .unwrap_or(0);
            pos = len_end;
        }
        // データ型の指定(`<NAME:3:S>`)は読み飛ばす
        if pos < end && bytes[pos] == b':' {
            pos = find(pos, b">");
        }
        if pos < end && bytes[pos] == b'>' {
            pos += 1;
        }

        if length > 0 && length <= end - pos.min(end) {
            fields.insert(tag, decode(&bytes[pos..pos + length]));
            pos += length;
        }
    }
    if !fields.is_empty() {
        records.push(fields);
    }
    records
}

/// 1つの ADIF の記録(UTF-8)を、項目にする
pub fn parse_adif_record(record: &str) -> AdifFields {
    scan_records(record.as_bytes(), &utf8_lossy)
        .into_iter()
        .flatten()
        .collect()
}

/// UTF-8 の ADIF を、記録に分ける(`parse_adif_bytes` の文字列版)
pub fn parse_adif(content: &str) -> Vec<AdifFields> {
    parse_adif_bytes(content.as_bytes(), &utf8_lossy)
}

/// ADIF の記録(1交信)を、交信にする
///
/// 自局の参照(`MY_SIG_INFO` か `MY_SOTA_REF`)は備考1に、相手の参照(`SIG_INFO` か `SOTA_REF`)は
/// QTH に、`COMMENT` は備考2に入れる。
pub fn decode_adif(record: &str) -> Result<QsoRecord, RecordError> {
    decode_adif_fields(&parse_adif_record(record))
}

/// 日付(`YYYYMMDD`)と時刻(`HHMM` か `HHMMSS`)を、存在する日時として読む(UTC)
fn parse_adif_moment(
    qso_date: &str,
    time_on: &str,
) -> Result<(i32, u32, u32, u32, u32), RecordError> {
    // 先頭の n 桁が ASCII の数字なら、その数
    fn digits(text: &str, from: usize, n: usize) -> Option<u32> {
        let part = text.as_bytes().get(from..from + n)?;
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0, |acc, b| acc * 10 + u32::from(b - b'0')))
    }
    let (Some(year), Some(month), Some(day)) = (
        digits(qso_date, 0, 4),
        digits(qso_date, 4, 2),
        digits(qso_date, 6, 2),
    ) else {
        return Err(RecordError::BadAdif(format!(
            "Invalid QSO_DATE: {qso_date}"
        )));
    };
    let (Some(hour), Some(minute)) = (digits(time_on, 0, 2), digits(time_on, 2, 2)) else {
        return Err(RecordError::BadAdif(format!("Invalid TIME_ON: {time_on}")));
    };
    let year = year as i32;
    if chrono::NaiveDate::from_ymd_opt(year, month, day).is_none() || hour > 23 || minute > 59 {
        return Err(RecordError::NonexistentDateTime {
            date: qso_date.to_string(),
            time: time_on.to_string(),
        });
    }
    Ok((year, month, day, hour, minute))
}

/// 項目にした ADIF の記録を、交信にする
pub fn decode_adif_fields(fields: &AdifFields) -> Result<QsoRecord, RecordError> {
    let bad = |reason: String| RecordError::BadAdif(reason);
    if fields.is_empty() {
        return Err(bad("Empty ADIF record".to_string()));
    }
    let field = |name: &str| fields.get(name).cloned().unwrap_or_default();
    let required = |name: &str| {
        fields
            .get(name)
            .ok_or_else(|| bad(format!("Missing {name} field")))
    };

    let callsign = required("CALL")?.clone();
    let qso_date = required("QSO_DATE")?;
    let time_on = required("TIME_ON")?;
    let (year, month, day, hour, minute) = parse_adif_moment(qso_date, time_on)?;

    // バンドは、周波数があればそこから、なければ BAND をそのまま
    let band_wlen = match (fields.get("FREQ"), fields.get("BAND")) {
        (Some(freq), _) => freq_to_band(freq).map_or_else(
            |_| freq.clone(),
            |(_, _, wavelength)| wavelength.to_string(),
        ),
        (None, Some(band)) => band.clone(),
        (None, None) => String::new(),
    };
    let (mode, sub_mode) = mode_to_adif_mode(&field("MODE"));

    let first_of = |names: [&str; 2]| {
        names
            .iter()
            .find_map(|n| fields.get(*n))
            .cloned()
            .unwrap_or_default()
    };
    Ok(QsoRecord {
        source_line: 0,
        errors: Vec::new(),
        callsign,
        year,
        month,
        day,
        hour,
        minute,
        rst_sent: field("RST_SENT"),
        rst_rcvd: field("RST_RCVD"),
        band_sota: String::new(),
        band_wlen,
        mode,
        sub_mode,
        mode_sota: String::new(),
        qth: first_of(["SIG_INFO", "SOTA_REF"]),
        remarks1: first_of(["MY_SIG_INFO", "MY_SOTA_REF"]),
        remarks2: field("COMMENT"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adif_record() {
        let record = "<CALL:6>JL1NIE<QSO_DATE:8>20241228<TIME_ON:4>1234<MODE:2>CW<BAND:3>40m<EOR>";
        let fields = parse_adif_record(record);

        assert_eq!(fields.get("CALL"), Some(&"JL1NIE".to_string()));
        assert_eq!(fields.get("QSO_DATE"), Some(&"20241228".to_string()));
        assert_eq!(fields.get("TIME_ON"), Some(&"1234".to_string()));
        assert_eq!(fields.get("MODE"), Some(&"CW".to_string()));
        assert_eq!(fields.get("BAND"), Some(&"40m".to_string()));
    }

    #[test]
    fn test_parse_adif_with_header() {
        let content = r#"ADIF Export
<PROGRAMID:4>TEST
<ADIF_VER:5>3.1.4
<EOH>
<CALL:6>JL1NIE<QSO_DATE:8>20241228<TIME_ON:4>1234<MODE:2>CW<EOR>
<CALL:6>JA1ABC<QSO_DATE:8>20241228<TIME_ON:4>1300<MODE:3>SSB<EOR>
"#;
        let records = parse_adif(content);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].get("CALL"), Some(&"JL1NIE".to_string()));
        assert_eq!(records[1].get("CALL"), Some(&"JA1ABC".to_string()));
    }

    #[test]
    fn test_decode_adif() {
        let record = "<CALL:6>JL1NIE<QSO_DATE:8>20241228<TIME_ON:4>1234<MODE:2>CW<RST_SENT:3>599<RST_RCVD:3>559<EOR>";
        let qso = decode_adif(record).unwrap();

        assert_eq!(qso.callsign, "JL1NIE");
        assert_eq!(qso.year, 2024);
        assert_eq!(qso.month, 12);
        assert_eq!(qso.day, 28);
        assert_eq!(qso.hour, 12);
        assert_eq!(qso.minute, 34);
        assert_eq!(qso.mode, "CW");
        assert_eq!(qso.rst_sent, "599");
        assert_eq!(qso.rst_rcvd, "559");
    }

    #[test]
    fn test_decode_adif_with_sota_ref() {
        let record = "<CALL:6>JL1NIE<QSO_DATE:8>20241228<TIME_ON:4>1234<MODE:2>CW<MY_SOTA_REF:9>JA/TK-001<SOTA_REF:9>JA/KN-001<EOR>";
        let qso = decode_adif(record).unwrap();

        assert_eq!(qso.remarks1, "JA/TK-001"); // my_sig
        assert_eq!(qso.qth, "JA/KN-001"); // his_sig
    }

    #[test]
    fn lengths_are_bytes_of_the_original_encoding() {
        // Shift_JIS の「山頂」(8E 52 92 A3)は 4 バイト。値ごとに、そのバイト数で切って読む
        let decode = |v: &[u8]| format!("{}bytes", v.len());
        let mut adif = b"<CALL:6>JA1AAA<COMMENT:4>".to_vec();
        adif.extend_from_slice(&[0x8E, 0x52, 0x92, 0xA3]);
        adif.extend_from_slice(b"<MODE:2>CW<EOR>");
        let records = parse_adif_bytes(&adif, &decode);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["COMMENT"], "4bytes");
        assert_eq!(records[0]["MODE"], "2bytes");
        // UTF-8 のファイルでは UTF-8 のバイト数
        let records = parse_adif("<CALL:6>JA1AAA<COMMENT:6>山頂<EOR>");
        assert_eq!(records[0]["COMMENT"], "山頂");
    }

    #[test]
    fn a_length_inside_a_character_does_not_panic() {
        let records = parse_adif("<CALL:6>JA1AAA<COMMENT:4>山頂<MODE:2>CW<EOR>");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["CALL"], "JA1AAA");
    }

    #[test]
    fn the_last_record_without_eor_and_records_without_call_are_returned() {
        let records = parse_adif(
            "ADIF\n<eoh>\n<CALL:6>JA1AAA<EOR>\n<QSO_DATE:8>20251018<EOR>\n<CALL:6>JA1BBB",
        );
        assert_eq!(records.len(), 3);
        assert_eq!(records[2]["CALL"], "JA1BBB");
        assert!(decode_adif_fields(&records[1]).is_err());
    }

    #[test]
    fn eoh_is_found_without_changing_the_position() {
        // 大文字にすると長さが変わる字(ŉ)が、ヘッダーにあっても
        let records = parse_adif("ADIF ŉŉŉŉ\n<EOH>\n<CALL:6>JA1AAA<EOR>");
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn non_ascii_or_nonexistent_datetime_is_an_error() {
        let rec = |date: &str, time: &str| {
            format!(
                "<CALL:6>JA1AAA<QSO_DATE:{}>{date}<TIME_ON:{}>{time}<EOR>",
                date.len(),
                time.len()
            )
        };
        for (date, time) in [
            ("２０２５１０１８", "0100"),
            ("20251018", "０１００"),
            ("20251345", "0100"),
            ("20250229", "0100"),
            ("20251018", "9999"),
            ("20251018", "2400"),
        ] {
            assert!(decode_adif(&rec(date, time)).is_err(), "{date} {time}");
        }
        assert!(decode_adif(&rec("20240229", "235959")).is_ok());
    }

    #[test]
    fn test_decode_adif_missing_fields() {
        let record = "<CALL:6>JL1NIE<EOR>";
        let result = decode_adif(record);
        assert!(result.is_err());
    }
}
