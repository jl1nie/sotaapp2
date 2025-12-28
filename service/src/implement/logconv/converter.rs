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
        String::new(),
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

    for my in &my_sota {
        let mut fields = qso_fields.clone();
        fields.push(adif_field("mysotaref", my));
        for his in &his_sota {
            fields.push(adif_field("sotaref", his));
        }
        fields.push("<EOR>\n".to_string());
        log.insert(my.clone(), fields);
    }

    // POTA references
    for my in &my_ref.pota {
        let mut fields = base_fields.clone();
        fields.push(adif_field("mysig", "POTA"));
        fields.push(adif_field("mysiginfo", my));
        for his in &his_ref.pota {
            fields.push(adif_field("sig", "POTA"));
            fields.push(adif_field("siginfo", his));
        }
        fields.push("<EOR>\n".to_string());
        log.insert(my.clone(), fields);
    }

    // WWFF references
    for my in &my_ref.wwff {
        let mut fields = qso_fields.clone();
        fields.push(adif_field("mysig", "WWFF"));
        fields.push(adif_field("mysiginfo", my));
        for his in &his_ref.wwff {
            fields.push(adif_field("sig", "WWFF"));
            fields.push(adif_field("siginfo", his));
        }
        fields.push("<EOR>\n".to_string());
        log.insert(my.clone(), fields);
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
    let my_str = make_str(&my_ref);

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

/// Convert QSO records to SOTA activator format
pub fn convert_to_sota_activator(
    records: &[QsoRecord],
    callsign: &str,
    options: &ConversionOptions,
) -> HashMap<String, String> {
    let mut files: HashMap<String, String> = HashMap::new();
    let mut current_date = String::new();
    let mut current_csv = String::new();
    let mut s2s_csv = String::new();

    for qso in records {
        if let Some((date, is_s2s, row)) = to_sota_activator(qso, callsign, options) {
            if row.is_empty() {
                continue;
            }

            if current_date.is_empty() {
                current_date = date.clone();
            }

            if date != current_date {
                // Save current files
                files.insert(format!("sota{}.csv", current_date), current_csv.clone());
                if !s2s_csv.is_empty() {
                    files.insert(format!("sota-s2s-{}.csv", current_date), s2s_csv.clone());
                }
                current_csv.clear();
                s2s_csv.clear();
                current_date = date;
            }

            let line = row.join(",") + "\n";
            current_csv.push_str(&line);
            if is_s2s {
                s2s_csv.push_str(&line);
            }
        }
    }

    // Save final files
    if !current_csv.is_empty() {
        files.insert(format!("sota{}.csv", current_date), current_csv);
    }
    if !s2s_csv.is_empty() {
        files.insert(format!("sota-s2s-{}.csv", current_date), s2s_csv);
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
