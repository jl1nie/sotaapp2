//! HAMLOG CSV(日本語版の HAMLOG)と、HamLog iOS の CSV の、1行の読み取り

use super::bands::freq_to_band;
use super::modes::{mode_to_adif_mode, mode_to_sota_mode};
use super::record::{QsoRecord, RecordError};
use chrono::{Datelike, FixedOffset, TimeZone, Timelike, Utc};
use once_cell::sync::Lazy;
use regex::Regex;

/// HAMLOG CSV の列
///
/// `コールサイン,日付,時刻,RST送,RST受,周波数,モード,コード,GL,QSL,名前,QTH,備考1,備考2,…`
mod hamlog_col {
    pub const MIN_COLUMNS: usize = 15;
    pub const CALLSIGN: usize = 0;
    pub const DATE: usize = 1;
    pub const TIME: usize = 2;
    pub const RST_SENT: usize = 3;
    pub const RST_RCVD: usize = 4;
    pub const FREQ: usize = 5;
    pub const MODE: usize = 6;
    pub const QTH: usize = 11;
    pub const REMARKS1: usize = 12;
    pub const REMARKS2: usize = 13;
}

/// HamLog iOS の CSV の列
mod ios_col {
    pub const MIN_COLUMNS: usize = 19;
    pub const DATETIME: usize = 0;
    pub const FREQ: usize = 2;
    pub const CALLSIGN: usize = 3;
    pub const RST_RCVD: usize = 4;
    pub const RST_SENT: usize = 5;
    /// QTH(備考1としても使う。iOS 版には、備考1の列がない)
    pub const QTH: usize = 8;
    pub const MODE: usize = 11;
    pub const REMARKS2: usize = 13;
}

static DATE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)/(\d+)/(\d+)").unwrap());
static TIME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d{2}):(\d{2})(\w)?").unwrap());
static IOS_DATETIME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2}) ([+-]\d{4})").unwrap()
});
static IOS_LOOKS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap());

/// 交信の日時(UTC)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Moment {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
}

impl Moment {
    /// 読み取れなかった日時の代わり(日付の誤りは、別に知らせる)
    const UNKNOWN: Moment = Moment {
        year: 1900,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
    };

    /// 現地の日時と、UTC からの時差(時間)から、UTC の日時にする。存在しない日時なら None
    fn from_local(
        (year, month, day): (i32, u32, u32),
        (hour, minute): (u32, u32),
        utc_offset_secs: i32,
    ) -> Option<Self> {
        let offset = FixedOffset::east_opt(utc_offset_secs)?;
        let local = offset
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()?;
        let utc = local.with_timezone(&Utc);
        Some(Self {
            year: utc.year(),
            month: utc.month(),
            day: utc.day(),
            hour: utc.hour(),
            minute: utc.minute(),
        })
    }
}

/// 日付。`YY/MM/DD` か `YYYY/MM/DD`。2桁の年は、65 以上なら 19xx、未満なら 20xx
fn parse_date(text: &str) -> Result<(i32, u32, u32), RecordError> {
    let caps = DATE_RE
        .captures(text)
        .ok_or_else(|| RecordError::BadDate(text.to_string()))?;
    let year_text = &caps[1];
    let year = if year_text.len() > 2 {
        year_text.parse().unwrap_or(1900)
    } else {
        match year_text.parse::<i32>().unwrap_or(0) {
            y if y >= 65 => 1900 + y,
            y => 2000 + y,
        }
    };
    Ok((
        year,
        caps[2].parse().unwrap_or(1),
        caps[3].parse().unwrap_or(1),
    ))
}

/// 時刻と、UTC からの時差(秒)。`HH:MM` に、`U`(UTC)か `Z` が付けば UTC、それ以外(`J` や、付かないとき)は JST
fn parse_time(text: &str) -> Result<((u32, u32), i32), RecordError> {
    let caps = TIME_RE
        .captures(text)
        .ok_or_else(|| RecordError::BadTime(text.to_string()))?;
    let flag = caps
        .get(3)
        .map_or(String::new(), |m| m.as_str().to_uppercase());
    let hours = if flag == "U" || flag == "Z" { 0 } else { 9 };
    Ok((
        (caps[1].parse().unwrap_or(0), caps[2].parse().unwrap_or(0)),
        hours * 3600,
    ))
}

/// 日付と時刻の文字列から、UTC の日時を作る。読み取れない所は `errors` に足す
fn parse_moment(date: &str, time: &str, errors: &mut Vec<RecordError>) -> Moment {
    match (parse_date(date), parse_time(time)) {
        (Ok(ymd), Ok((hm, offset))) => Moment::from_local(ymd, hm, offset).unwrap_or_else(|| {
            errors.push(RecordError::NonexistentDateTime {
                date: date.to_string(),
                time: time.to_string(),
            });
            Moment::UNKNOWN
        }),
        (date_result, time_result) => {
            errors.extend(date_result.err());
            errors.extend(time_result.err());
            Moment::UNKNOWN
        }
    }
}

/// 周波数から、バンドの名前(SOTA での名前・波長での名前)を引く。どのバンドにもなければ `errors` に足す
fn parse_band(freq: &str, errors: &mut Vec<RecordError>) -> (String, String) {
    match freq_to_band(freq) {
        Ok((_, sota, wavelength)) => (sota.to_string(), wavelength.to_string()),
        Err(reason) => {
            errors.push(RecordError::BadFrequency(reason));
            (String::new(), String::new())
        }
    }
}

/// 交信を作る(日時・バンド以外の項目は、読み取り元の列から)
struct Columns<'a> {
    callsign: &'a str,
    rst_sent: &'a str,
    rst_rcvd: &'a str,
    mode: &'a str,
    qth: &'a str,
    remarks1: &'a str,
    remarks2: &'a str,
}

fn build(
    columns: Columns,
    moment: Moment,
    band: (String, String),
    errors: Vec<RecordError>,
) -> QsoRecord {
    let (mode, sub_mode) = mode_to_adif_mode(columns.mode);
    QsoRecord {
        source_line: 0,
        errors,
        callsign: columns.callsign.to_string(),
        year: moment.year,
        month: moment.month,
        day: moment.day,
        hour: moment.hour,
        minute: moment.minute,
        rst_sent: columns.rst_sent.to_string(),
        rst_rcvd: columns.rst_rcvd.to_string(),
        band_sota: band.0,
        band_wlen: band.1,
        mode,
        sub_mode,
        mode_sota: mode_to_sota_mode(columns.mode).to_string(),
        qth: columns.qth.to_string(),
        remarks1: columns.remarks1.to_string(),
        remarks2: columns.remarks2.to_string(),
    }
}

/// HAMLOG CSV の1行を読む
pub fn decode_hamlog(cols: &[String]) -> Result<QsoRecord, RecordError> {
    use hamlog_col as c;
    if cols.len() < c::MIN_COLUMNS {
        return Err(RecordError::NotHamlogCsv);
    }

    let mut errors = Vec::new();
    let moment = parse_moment(&cols[c::DATE], &cols[c::TIME], &mut errors);
    let band = parse_band(&cols[c::FREQ], &mut errors);
    Ok(build(
        Columns {
            callsign: &cols[c::CALLSIGN],
            rst_sent: &cols[c::RST_SENT],
            rst_rcvd: &cols[c::RST_RCVD],
            mode: &cols[c::MODE],
            qth: &cols[c::QTH],
            remarks1: &cols[c::REMARKS1],
            remarks2: &cols[c::REMARKS2],
        },
        moment,
        band,
        errors,
    ))
}

/// HamLog iOS の CSV の1行を読む
pub fn decode_hamlog_ios(cols: &[String]) -> Result<QsoRecord, RecordError> {
    use ios_col as c;
    if cols.len() < c::MIN_COLUMNS {
        return Err(RecordError::TooFewColumns(cols.len()));
    }

    let mut errors = Vec::new();
    let moment = parse_ios_datetime(&cols[c::DATETIME], &mut errors);
    let band = parse_band(&cols[c::FREQ], &mut errors);
    Ok(build(
        Columns {
            callsign: &cols[c::CALLSIGN],
            rst_sent: &cols[c::RST_SENT],
            rst_rcvd: &cols[c::RST_RCVD],
            mode: &cols[c::MODE],
            qth: &cols[c::QTH],
            remarks1: &cols[c::QTH],
            remarks2: &cols[c::REMARKS2],
        },
        moment,
        band,
        errors,
    ))
}

/// iOS の日時(`2024-12-28 12:34:56 +0900`)。時差が付いているので、UTC にする
fn parse_ios_datetime(text: &str, errors: &mut Vec<RecordError>) -> Moment {
    let Some(caps) = IOS_DATETIME_RE.captures(text) else {
        errors.push(RecordError::BadTime(text.to_string()));
        return Moment::UNKNOWN;
    };
    let number = |i: usize| -> u32 { caps[i].parse().unwrap_or(0) };
    // `+0900` → 9時間0分(`-0530` は、マイナス5時間30分)
    let offset = &caps[7];
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let hours: i32 = offset[1..3].parse().unwrap_or(0);
    let minutes: i32 = offset[3..5].parse().unwrap_or(0);
    let offset_secs = sign * (hours * 3600 + minutes * 60);

    Moment::from_local(
        (caps[1].parse().unwrap_or(1900), number(2), number(3)),
        (number(4), number(5)),
        offset_secs,
    )
    .unwrap_or_else(|| {
        errors.push(RecordError::BadTime(text.to_string()));
        Moment::UNKNOWN
    })
}

/// 見出しの行かを調べ、形式(HAMLOG か iOS)を見分けて、1行を読む
///
/// 見出しの行(iOS の `TimeOn` を含む1行目)は `Ok(None)`。
pub fn decode_auto(cols: &[String], header_seen: bool) -> Result<Option<QsoRecord>, RecordError> {
    if !header_seen && cols.first().is_some_and(|s| s.contains("TimeOn")) {
        return Ok(None);
    }
    // 列が多く、1列目が iOS の日時の形なら、iOS
    if cols.len() >= ios_col::MIN_COLUMNS && IOS_LOOKS_RE.is_match(&cols[0]) {
        return decode_hamlog_ios(cols).map(Some);
    }
    decode_hamlog(cols).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols(data: &[&str]) -> Vec<String> {
        data.iter().map(|s| s.to_string()).collect()
    }

    /// 日付・時刻・周波数・モード以外は空の HAMLOG の行
    fn row(date: &str, time: &str, freq: &str, mode: &str) -> Vec<String> {
        let mut c = cols(&["JL1NIE", date, time, "599", "559", freq, mode]);
        c.resize(hamlog_col::MIN_COLUMNS, String::new());
        c
    }

    #[test]
    fn test_decode_hamlog_basic() {
        let mut c = row("24/12/28", "12:34U", "7.025", "CW");
        c[hamlog_col::QTH] = "Tokyo".to_string();
        c[hamlog_col::REMARKS1] = "JA/TK-001".to_string();

        let qso = decode_hamlog(&c).unwrap();
        assert!(!qso.has_error());
        assert_eq!(qso.callsign, "JL1NIE");
        assert_eq!(
            (qso.year, qso.month, qso.day, qso.hour, qso.minute),
            (2024, 12, 28, 12, 34)
        );
        assert_eq!(
            (qso.band_sota.as_str(), qso.band_wlen.as_str()),
            ("7MHz", "40m")
        );
        assert_eq!((qso.mode.as_str(), qso.mode_sota.as_str()), ("CW", "CW"));
        assert_eq!(
            (qso.rst_sent.as_str(), qso.rst_rcvd.as_str()),
            ("599", "559")
        );
        assert_eq!(
            (qso.qth.as_str(), qso.remarks1.as_str()),
            ("Tokyo", "JA/TK-001")
        );
    }

    #[test]
    fn jst_times_are_converted_to_utc() {
        // 21:34 JST = 12:34 UTC。フラグがなければ JST
        for time in ["21:34J", "21:34"] {
            let qso = decode_hamlog(&row("24/12/28", time, "7.025", "CW")).unwrap();
            assert_eq!((qso.hour, qso.minute), (12, 34), "{time}");
        }
        // UTC の日付をまたぐ(JST 00:30 は前日)
        let qso = decode_hamlog(&row("25/10/19", "00:30J", "7.025", "CW")).unwrap();
        assert_eq!((qso.month, qso.day, qso.hour, qso.minute), (10, 18, 15, 30));
    }

    #[test]
    fn two_digit_years_pivot_at_65() {
        let year = |d: &str| {
            decode_hamlog(&row(d, "12:00U", "7.025", "CW"))
                .unwrap()
                .year
        };
        assert_eq!(year("24/01/01"), 2024);
        assert_eq!(year("99/12/31"), 1999);
        assert_eq!(year("65/01/01"), 1965);
        assert_eq!(year("2025/10/18"), 2025);
    }

    #[test]
    fn a_nonexistent_time_is_an_error_with_a_message() {
        let qso = decode_hamlog(&row("25/10/19", "99:99J", "14.285", "SSB")).unwrap();
        assert!(qso.has_error());
        assert_eq!(qso.error_message(), "日時不正:25/10/19 99:99J");
    }

    #[test]
    fn malformed_fields_are_reported_each() {
        let qso = decode_hamlog(&row("25-10-18", "ab:cd", "9.999", "CW")).unwrap();
        assert_eq!(
            qso.errors,
            vec![
                RecordError::BadDate("25-10-18".to_string()),
                RecordError::BadTime("ab:cd".to_string()),
                RecordError::BadFrequency("Unknown frequency: 9.999".to_string()),
            ]
        );
        assert_eq!(
            qso.error_message(),
            "日付フォーマット不正:25-10-18, 時刻フォーマット不正:ab:cd, 周波数不正:Unknown frequency: 9.999"
        );
    }

    #[test]
    fn test_decode_hamlog_too_short() {
        let result = decode_hamlog(&cols(&["JL1NIE", "24/12/28"]));
        assert_eq!(result.unwrap_err(), RecordError::NotHamlogCsv);
    }

    fn ios_row(datetime: &str) -> Vec<String> {
        let mut c = vec![String::new(); ios_col::MIN_COLUMNS];
        c[ios_col::DATETIME] = datetime.to_string();
        c[ios_col::FREQ] = "7.025".to_string();
        c[ios_col::CALLSIGN] = "JL1NIE".to_string();
        c[ios_col::RST_RCVD] = "559".to_string();
        c[ios_col::RST_SENT] = "599".to_string();
        c[ios_col::QTH] = "JA/TK-001".to_string();
        c[ios_col::MODE] = "CW".to_string();
        c[ios_col::REMARKS2] = "Comment".to_string();
        c
    }

    #[test]
    fn test_decode_hamlog_ios() {
        let qso = decode_hamlog_ios(&ios_row("2024-12-28 12:34:56 +0000")).unwrap();
        assert_eq!(qso.callsign, "JL1NIE");
        assert_eq!(
            (qso.year, qso.month, qso.day, qso.hour, qso.minute),
            (2024, 12, 28, 12, 34)
        );
        assert_eq!(
            (qso.rst_sent.as_str(), qso.rst_rcvd.as_str()),
            ("599", "559")
        );
        // iOS 版には備考1の列がない。QTH を、備考1としても使う
        assert_eq!(
            (qso.qth.as_str(), qso.remarks1.as_str()),
            ("JA/TK-001", "JA/TK-001")
        );
        assert_eq!(qso.remarks2, "Comment");
    }

    #[test]
    fn ios_time_zones_are_converted_to_utc() {
        let qso = decode_hamlog_ios(&ios_row("2025-10-19 08:30:00 +0900")).unwrap();
        assert_eq!((qso.month, qso.day, qso.hour, qso.minute), (10, 18, 23, 30));
        let qso = decode_hamlog_ios(&ios_row("2025-10-18 20:00:00 -0500")).unwrap();
        assert_eq!((qso.month, qso.day, qso.hour, qso.minute), (10, 19, 1, 0));
        let qso = decode_hamlog_ios(&ios_row("2025-10-18 12:00:00 +0530")).unwrap();
        assert_eq!((qso.hour, qso.minute), (6, 30));
    }

    #[test]
    fn a_malformed_ios_datetime_is_an_error() {
        let qso = decode_hamlog_ios(&ios_row("yesterday")).unwrap();
        assert_eq!(
            qso.errors,
            vec![RecordError::BadTime("yesterday".to_string())]
        );
        assert_eq!(
            decode_hamlog_ios(&cols(&["x"])).unwrap_err(),
            RecordError::TooFewColumns(1)
        );
    }

    #[test]
    fn decode_auto_tells_the_formats_apart() {
        let hamlog = decode_auto(&row("24/12/28", "12:34U", "7.025", "CW"), true);
        assert_eq!(hamlog.unwrap().unwrap().callsign, "JL1NIE");

        let ios = decode_auto(&ios_row("2024-12-28 12:34:56 +0000"), true);
        assert_eq!(ios.unwrap().unwrap().callsign, "JL1NIE");

        // 列が足りない行は、誤り
        assert_eq!(
            decode_auto(&cols(&["JA1SHT", "25/10/18"]), true).unwrap_err(),
            RecordError::NotHamlogCsv
        );
    }

    #[test]
    fn the_ios_header_row_is_skipped_only_at_the_start() {
        let header = cols(&["TimeOn", "x", "Freq"]);
        assert!(decode_auto(&header, false).unwrap().is_none());
        assert!(decode_auto(&header, true).is_err());
    }
}
