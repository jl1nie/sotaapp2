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

/// Parse a single ADIF record into a HashMap of fields
pub fn parse_adif_record(record: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    let mut pos = 0;
    let bytes = record.as_bytes();

    while pos < bytes.len() {
        // Find next tag start
        while pos < bytes.len() && bytes[pos] != b'<' {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        pos += 1; // skip '<'

        // Find tag end
        let tag_start = pos;
        while pos < bytes.len() && bytes[pos] != b'>' && bytes[pos] != b':' {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }

        let tag_name = &record[tag_start..pos];
        let tag_upper = tag_name.to_uppercase();

        // Check for end markers
        if tag_upper == "EOR" || tag_upper == "EOH" {
            // Skip to end of tag
            while pos < bytes.len() && bytes[pos] != b'>' {
                pos += 1;
            }
            pos += 1;
            continue;
        }

        // Parse length if present
        let length = if pos < bytes.len() && bytes[pos] == b':' {
            pos += 1; // skip ':'
            let len_start = pos;
            while pos < bytes.len() && bytes[pos] != b'>' && bytes[pos] != b':' {
                pos += 1;
            }
            let len_str = &record[len_start..pos];
            len_str.parse::<usize>().unwrap_or(0)
        } else {
            0
        };

        // Skip optional data type indicator
        if pos < bytes.len() && bytes[pos] == b':' {
            while pos < bytes.len() && bytes[pos] != b'>' {
                pos += 1;
            }
        }

        // Skip '>'
        if pos < bytes.len() && bytes[pos] == b'>' {
            pos += 1;
        }

        // Extract value
        if length > 0 && pos + length <= record.len() {
            let value = &record[pos..pos + length];
            fields.insert(tag_upper, value.to_string());
            pos += length;
        }
    }

    fields
}

/// Parse ADIF content into QSO records
pub fn parse_adif(content: &str) -> Vec<HashMap<String, String>> {
    let mut records = Vec::new();
    let content_upper = content.to_uppercase();

    // Find end of header
    let body_start = if let Some(pos) = content_upper.find("<EOH>") {
        pos + 5
    } else {
        0
    };

    let body = &content[body_start..];

    // Split by <EOR>
    for record in body.split_inclusive("<EOR>") {
        // Case-insensitive split
        let record_lower = record.to_lowercase();
        let actual_record = if record_lower.contains("<eor>") {
            record
        } else {
            continue;
        };

        if actual_record.trim().is_empty() {
            continue;
        }

        let fields = parse_adif_record(actual_record);
        if !fields.is_empty() && fields.contains_key("CALL") {
            records.push(fields);
        }
    }

    records
}

/// ADIF の記録(1交信)を、交信にする
///
/// 自局の参照(`MY_SIG_INFO` か `MY_SOTA_REF`)は備考1に、相手の参照(`SIG_INFO` か `SOTA_REF`)は
/// QTH に、`COMMENT` は備考2に入れる。
pub fn decode_adif(record: &str) -> Result<QsoRecord, RecordError> {
    let bad = |reason: String| RecordError::BadAdif(reason);
    let fields = parse_adif_record(record);
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
    // 日付(YYYYMMDD)と時刻(HHMM か HHMMSS)
    if qso_date.len() < 8 {
        return Err(bad(format!("Invalid QSO_DATE: {qso_date}")));
    }
    if time_on.len() < 4 {
        return Err(bad(format!("Invalid TIME_ON: {time_on}")));
    }

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
        year: qso_date[0..4].parse().unwrap_or(1900),
        month: qso_date[4..6].parse().unwrap_or(1),
        day: qso_date[6..8].parse().unwrap_or(1),
        hour: time_on[0..2].parse().unwrap_or(0),
        minute: time_on[2..4].parse().unwrap_or(0),
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
    fn test_decode_adif_missing_fields() {
        let record = "<CALL:6>JL1NIE<EOR>";
        let result = decode_adif(record);
        assert!(result.is_err());
    }
}
