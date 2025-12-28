//! HAMLOG CSV format parser
//!
//! Parses HAMLOG CSV format (Japanese logging software) and HamLog iOS format

use super::types::{
    freq_to_band, mode_to_adif_mode, mode_to_airham_mode, mode_to_sota_mode, split_callsign,
    QsoRecord,
};
use chrono::{FixedOffset, Offset, TimeZone, Utc};
use regex::Regex;

/// Decode HAMLOG CSV row into QsoRecord
///
/// HAMLOG CSV format columns:
/// 0: callsign, 1: date, 2: time, 3: rst_sent, 4: rst_rcvd, 5: freq,
/// 6: mode, 7: code, 8: gl, 9: qsl, 10: name, 11: qth, 12: rmks1, 13: rmks2, ...
pub fn decode_hamlog(cols: &[String]) -> Result<QsoRecord, String> {
    if cols.len() < 15 {
        return Err("HAMLOG CSV形式ではありません".to_string());
    }

    let mut record = QsoRecord::default();
    let (operator, portable) = split_callsign(&cols[0]);
    record.callsign = cols[0].clone();
    record.operator = operator;
    record.portable = portable;

    // Parse date (YY/MM/DD or YYYY/MM/DD)
    let date_re = Regex::new(r"(\d+)/(\d+)/(\d+)").unwrap();
    let (year, month, day, date_error) = if let Some(caps) = date_re.captures(&cols[1]) {
        let y_str = &caps[1];
        let year = if y_str.len() > 2 {
            y_str.parse::<i32>().unwrap_or(1900)
        } else {
            let y = y_str.parse::<i32>().unwrap_or(0);
            if y >= 65 {
                1900 + y
            } else {
                2000 + y
            }
        };
        let month = caps[2].parse::<u32>().unwrap_or(1);
        let day = caps[3].parse::<u32>().unwrap_or(1);
        (year, month, day, String::new())
    } else {
        record.has_error = true;
        record.error_message = format!("日付フォーマット不正:{}", cols[1]);
        (
            1900,
            1,
            1,
            format!("<font color=\"red\"><b>{}</b></font>", cols[1]),
        )
    };
    record.date_error = date_error;

    // Parse time (HH:MMU or HH:MMJ)
    let time_re = Regex::new(r"(\d{2}):(\d{2})(\w)?").unwrap();
    let (hour, minute, tz_offset, time_error) = if let Some(caps) = time_re.captures(&cols[2]) {
        let hour = caps[1].parse::<u32>().unwrap_or(0);
        let minute = caps[2].parse::<u32>().unwrap_or(0);
        let flag = caps
            .get(3)
            .map(|m| m.as_str().to_uppercase())
            .unwrap_or_default();
        let offset = if flag == "U" || flag == "Z" { 0 } else { 9 };
        (hour, minute, offset, String::new())
    } else {
        record.has_error = true;
        if !record.error_message.is_empty() {
            record.error_message.push_str(", ");
        }
        record
            .error_message
            .push_str(&format!("時刻フォーマット不正:{}", cols[2]));
        (
            0,
            0,
            9,
            format!("<font color=\"red\"><b>{}</b></font>", cols[2]),
        )
    };
    record.time_error = time_error;
    record.timezone = if tz_offset == 0 {
        "+0000".to_string()
    } else {
        "+0900".to_string()
    };

    // Convert to UTC
    let offset = FixedOffset::east_opt(tz_offset * 3600).unwrap_or(Utc.fix());
    if let Some(dt) = offset
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
    {
        let utc_dt = dt.with_timezone(&Utc);
        record.year = utc_dt.year();
        record.month = utc_dt.month();
        record.day = utc_dt.day();
        record.hour = utc_dt.hour();
        record.minute = utc_dt.minute();
        record.iso_time = dt.to_rfc3339();
    } else {
        record.has_error = true;
        record.year = 1900;
        record.month = 1;
        record.day = 1;
        record.hour = 0;
        record.minute = 0;
        record.iso_time = String::new();
    }

    // Parse frequency/band
    match freq_to_band(&cols[5]) {
        Ok((band_air, band_sota, wlen)) => {
            record.band = band_air.to_string();
            record.band_sota = band_sota.to_string();
            record.band_wlen = wlen.to_string();
        }
        Err(e) => {
            record.has_error = true;
            record.band_error = format!("<font color=\"red\"><b>{}</b></font>", e);
            if !record.error_message.is_empty() {
                record.error_message.push_str(", ");
            }
            record.error_message.push_str(&format!("周波数不正:{}", e));
        }
    }

    // Parse QSL flag
    let qsl_flag = format!("{}   ", cols[9].to_uppercase());
    let qsl_chars: Vec<char> = qsl_flag.chars().take(3).collect();

    record.qsl = match qsl_chars.first() {
        Some('N') => "No Card".to_string(),
        Some('J') => "JARL (Bureau)".to_string(),
        _ => qsl_flag[..3].to_string(),
    };
    record.qsl_sent = if qsl_chars.get(1).map(|c| *c != ' ').unwrap_or(false) {
        1
    } else {
        0
    };
    record.qsl_rcvd = if qsl_chars.get(2).map(|c| *c != ' ').unwrap_or(false) {
        1
    } else {
        0
    };

    // Mode conversion
    let (mode, sub_mode) = mode_to_adif_mode(&cols[6]);
    record.mode = mode;
    record.sub_mode = sub_mode;
    record.mode_airham = mode_to_airham_mode(&cols[6], &cols[5]);
    record.mode_sota = mode_to_sota_mode(&cols[6]).to_string();

    // Other fields
    record.rst_sent = cols[3].clone();
    record.rst_rcvd = cols[4].clone();
    record.freq = cols[5].clone();
    record.code = cols[7].clone();
    record.gridsquare = cols[8].clone();
    record.name = cols[10].clone();
    record.qth = cols[11].clone();
    record.remarks1 = cols[12].clone();
    record.remarks2 = cols.get(13).cloned().unwrap_or_default();

    Ok(record)
}

/// Decode HamLog iOS CSV row into QsoRecord
///
/// HamLog iOS format columns:
/// 0: datetime, 1: ?, 2: freq, 3: callsign, 4: rst_rcvd, 5: rst_sent,
/// 6: gl, 7: name, 8: qth, ...11: mode, ...13: rmks2, 14: qsl, 15: qsl_sent, 16: qsl_rcvd, ...
pub fn decode_hamlog_ios(cols: &[String]) -> Result<QsoRecord, String> {
    if cols.len() < 19 {
        return Err(format!("Too short columns: {} < 19", cols.len()));
    }

    let mut record = QsoRecord::default();
    let (operator, portable) = split_callsign(&cols[3]);
    record.callsign = cols[3].clone();
    record.operator = operator;
    record.portable = portable;

    // Parse datetime (YYYY-MM-DD HH:MM:SS +ZZZZ)
    let dt_re = Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2}) ([+-]\d{4})").unwrap();
    if let Some(caps) = dt_re.captures(&cols[0]) {
        let year: i32 = caps[1].parse().unwrap_or(1900);
        let month: u32 = caps[2].parse().unwrap_or(1);
        let day: u32 = caps[3].parse().unwrap_or(1);
        let hour: u32 = caps[4].parse().unwrap_or(0);
        let minute: u32 = caps[5].parse().unwrap_or(0);
        let _second: u32 = caps[6].parse().unwrap_or(0);
        let tz_str = &caps[7];

        let tz_hours: i32 = tz_str[0..3].parse().unwrap_or(0);
        let tz_mins: i32 = tz_str[3..5].parse().unwrap_or(0);
        let tz_secs = tz_hours * 3600 + tz_mins * 60;

        if let Some(offset) = FixedOffset::east_opt(tz_secs) {
            if let Some(dt) = offset
                .with_ymd_and_hms(year, month, day, hour, minute, 0)
                .single()
            {
                let utc_dt = dt.with_timezone(&Utc);
                record.year = utc_dt.year();
                record.month = utc_dt.month();
                record.day = utc_dt.day();
                record.hour = utc_dt.hour();
                record.minute = utc_dt.minute();
                record.iso_time = dt.to_rfc3339();
                record.timezone = "+0000".to_string();
            }
        }
    } else {
        record.has_error = true;
        record.error_message = format!("時刻フォーマット不正:{}", cols[0]);
        record.date_error = format!(
            "<font color=\"red\"><b>{}</b></font>",
            cols[0].split(' ').next().unwrap_or("")
        );
        record.time_error = format!(
            "<font color=\"red\"><b>{}</b></font>",
            cols[0].split(' ').nth(1).unwrap_or("")
        );
    }

    // Parse frequency/band
    match freq_to_band(&cols[2]) {
        Ok((band_air, band_sota, wlen)) => {
            record.band = band_air.to_string();
            record.band_sota = band_sota.to_string();
            record.band_wlen = wlen.to_string();
        }
        Err(e) => {
            record.has_error = true;
            record.band_error = format!("<font color=\"red\"><b>{}</b></font>", e);
            if !record.error_message.is_empty() {
                record.error_message.push_str(", ");
            }
            record
                .error_message
                .push_str(&format!("Frequency out of range:{}", e));
        }
    }

    // Mode conversion
    let (mode, sub_mode) = mode_to_adif_mode(&cols[11]);
    record.mode = mode;
    record.sub_mode = sub_mode;
    record.mode_airham = mode_to_airham_mode(&cols[11], &cols[2]);
    record.mode_sota = mode_to_sota_mode(&cols[11]).to_string();

    // Other fields
    record.rst_sent = cols[5].clone();
    record.rst_rcvd = cols[4].clone();
    record.freq = cols[2].clone();
    record.gridsquare = cols[6].clone();
    record.name = cols[7].clone();
    record.qth = cols[8].clone();
    record.remarks1 = cols[8].clone();
    record.remarks2 = cols[13].clone();
    record.qsl = cols[14].clone();
    record.qsl_sent = cols[15].parse().unwrap_or(0);
    record.qsl_rcvd = cols[16].parse().unwrap_or(0);

    Ok(record)
}

/// Detect format and decode row
pub fn decode_auto(cols: &[String], header_seen: bool) -> Result<QsoRecord, String> {
    // Check for HamLog iOS header
    if !header_seen && cols.first().map(|s| s.contains("TimeOn")).unwrap_or(false) {
        return Err("HEADER".to_string());
    }

    // Try HamLog iOS format if enough columns and datetime pattern matches
    if cols.len() >= 19 {
        let dt_re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();
        if dt_re.is_match(&cols[0]) {
            return decode_hamlog_ios(cols);
        }
    }

    // Default to HAMLOG format
    decode_hamlog(cols)
}

use chrono::Datelike;
use chrono::Timelike;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cols(data: &[&str]) -> Vec<String> {
        data.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_decode_hamlog_basic() {
        let cols = make_cols(&[
            "JL1NIE",    // 0: callsign
            "24/12/28",  // 1: date
            "12:34U",    // 2: time
            "599",       // 3: rst_sent
            "559",       // 4: rst_rcvd
            "7.025",     // 5: freq
            "CW",        // 6: mode
            "",          // 7: code
            "PM95",      // 8: gl
            "J S",       // 9: qsl
            "Taro",      // 10: name
            "Tokyo",     // 11: qth
            "JA/TK-001", // 12: rmks1
            "",          // 13: rmks2
            "",          // 14
        ]);

        let qso = decode_hamlog(&cols).unwrap();
        assert_eq!(qso.callsign, "JL1NIE");
        assert_eq!(qso.operator, "JL1NIE");
        assert_eq!(qso.year, 2024);
        assert_eq!(qso.month, 12);
        assert_eq!(qso.day, 28);
        assert_eq!(qso.hour, 12);
        assert_eq!(qso.minute, 34);
        assert_eq!(qso.band, "7MHz");
        assert_eq!(qso.mode, "CW");
        assert_eq!(qso.rst_sent, "599");
        assert_eq!(qso.qsl, "JARL (Bureau)"); // J -> JARL Bureau
        assert_eq!(qso.qsl_sent, 0); // ' ' = not sent (2nd char)
        assert_eq!(qso.qsl_rcvd, 1); // 'S' = received (3rd char)
    }

    #[test]
    fn test_decode_hamlog_jst_time() {
        let cols = make_cols(&[
            "JL1NIE", "24/12/28", "21:34J", // JST
            "599", "559", "7.025", "CW", "", "", "", "", "", "", "", "",
        ]);

        let qso = decode_hamlog(&cols).unwrap();
        // 21:34 JST = 12:34 UTC
        assert_eq!(qso.hour, 12);
        assert_eq!(qso.minute, 34);
    }

    #[test]
    fn test_decode_hamlog_portable() {
        let cols = make_cols(&[
            "JA/JL1NIE/P",
            "24/12/28",
            "12:34U",
            "599",
            "559",
            "7.025",
            "CW",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ]);

        let qso = decode_hamlog(&cols).unwrap();
        assert_eq!(qso.callsign, "JA/JL1NIE/P");
        assert_eq!(qso.operator, "JL1NIE");
        assert_eq!(qso.portable, "JA");
    }

    #[test]
    fn test_decode_hamlog_too_short() {
        let cols = make_cols(&["JL1NIE", "24/12/28"]);
        let result = decode_hamlog(&cols);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_hamlog_ios() {
        let cols = make_cols(&[
            "2024-12-28 12:34:56 +0000", // 0
            "",                          // 1
            "7.025",                     // 2
            "JL1NIE",                    // 3
            "559",                       // 4 rst_rcvd
            "599",                       // 5 rst_sent
            "PM95",                      // 6 gl
            "Taro",                      // 7 name
            "Tokyo",                     // 8 qth
            "",                          // 9
            "",                          // 10
            "CW",                        // 11 mode
            "",                          // 12
            "Comment",                   // 13 rmks2
            "J",                         // 14 qsl
            "1",                         // 15 qsl_sent
            "0",                         // 16 qsl_rcvd
            "",                          // 17
            "",                          // 18
        ]);

        let qso = decode_hamlog_ios(&cols).unwrap();
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
    fn test_decode_auto_hamlog() {
        let cols = make_cols(&[
            "JL1NIE", "24/12/28", "12:34U", "599", "559", "7.025", "CW", "", "", "", "", "", "",
            "", "",
        ]);

        let qso = decode_auto(&cols, true).unwrap();
        assert_eq!(qso.callsign, "JL1NIE");
    }

    #[test]
    fn test_decode_auto_ios() {
        let cols = make_cols(&[
            "2024-12-28 12:34:56 +0000",
            "",
            "7.025",
            "JL1NIE",
            "559",
            "599",
            "PM95",
            "Taro",
            "Tokyo",
            "",
            "",
            "CW",
            "",
            "",
            "J",
            "1",
            "0",
            "",
            "",
        ]);

        let qso = decode_auto(&cols, true).unwrap();
        assert_eq!(qso.callsign, "JL1NIE");
    }
}
