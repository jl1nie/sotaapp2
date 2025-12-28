//! ADIF (Amateur Data Interchange Format) parser
//!
//! Parses ADIF 3.x format files into QSO records

use super::types::{freq_to_band, mode_to_adif_mode, QsoRecord};
use std::collections::HashMap;

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

/// Decode ADIF record into QsoRecord
pub fn decode_adif(record: &str) -> Result<QsoRecord, String> {
    let fields = parse_adif_record(record);

    if fields.is_empty() {
        return Err("Empty ADIF record".to_string());
    }

    let call = fields
        .get("CALL")
        .ok_or_else(|| "Missing CALL field".to_string())?
        .clone();

    let qso_date = fields
        .get("QSO_DATE")
        .ok_or_else(|| "Missing QSO_DATE field".to_string())?;

    let time_on = fields
        .get("TIME_ON")
        .ok_or_else(|| "Missing TIME_ON field".to_string())?;

    // Parse date (YYYYMMDD)
    if qso_date.len() < 8 {
        return Err(format!("Invalid QSO_DATE: {}", qso_date));
    }
    let year: i32 = qso_date[0..4].parse().unwrap_or(1900);
    let month: u32 = qso_date[4..6].parse().unwrap_or(1);
    let day: u32 = qso_date[6..8].parse().unwrap_or(1);

    // Parse time (HHMM or HHMMSS)
    if time_on.len() < 4 {
        return Err(format!("Invalid TIME_ON: {}", time_on));
    }
    let hour: u32 = time_on[0..2].parse().unwrap_or(0);
    let minute: u32 = time_on[2..4].parse().unwrap_or(0);

    // Get band/wavelength
    let wlen = if let Some(freq) = fields.get("FREQ") {
        match freq_to_band(freq) {
            Ok((_, _, wl)) => wl.to_string(),
            Err(_) => freq.clone(),
        }
    } else if let Some(band) = fields.get("BAND") {
        band.clone()
    } else {
        String::new()
    };

    // Get mode
    let mode_raw = fields.get("MODE").cloned().unwrap_or_default();
    let (mode, sub_mode) = mode_to_adif_mode(&mode_raw);

    // Get references
    let my_sig = fields
        .get("MY_SIG_INFO")
        .or_else(|| fields.get("MY_SOTA_REF"))
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let his_sig = fields
        .get("SIG_INFO")
        .or_else(|| fields.get("SOTA_REF"))
        .cloned()
        .unwrap_or_default();

    Ok(QsoRecord {
        has_error: false,
        error_message: String::new(),
        date_error: String::new(),
        time_error: String::new(),
        band_error: String::new(),

        callsign: call,
        operator: String::new(),
        portable: String::new(),
        iso_time: String::new(),
        year,
        month,
        day,
        hour,
        minute,
        timezone: "+0000".to_string(),
        rst_sent: fields.get("RST_SENT").cloned().unwrap_or_default(),
        rst_rcvd: fields.get("RST_RCVD").cloned().unwrap_or_default(),
        freq: fields.get("FREQ").cloned().unwrap_or_default(),
        band: String::new(),
        band_sota: String::new(),
        band_wlen: wlen,
        mode,
        sub_mode,
        mode_airham: String::new(),
        mode_sota: String::new(),
        code: String::new(),
        gridsquare: fields.get("GRIDSQUARE").cloned().unwrap_or_default(),
        qsl: String::new(),
        qsl_sent: 0,
        qsl_rcvd: 0,
        name: fields.get("NAME").cloned().unwrap_or_default(),
        qth: his_sig,
        remarks1: my_sig,
        remarks2: fields.get("COMMENT").cloned().unwrap_or_default(),
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
