//! モードの名前の変換(SOTA のモード、ADIF のモードとサブモード、AirHam のモード。旧 FLE も使う)

use once_cell::sync::Lazy;

/// SOTA mode conversion patterns
pub static SOTA_MODE_TABLE: Lazy<Vec<(&'static str, Vec<&'static str>)>> = Lazy::new(|| {
    vec![
        ("CW", vec!["CW"]),
        ("SSB", vec!["SSB"]),
        ("FM", vec!["FM"]),
        ("AM", vec!["AM"]),
        (
            "DATA",
            vec![
                "RTTY", "RTY", "PSK", "PSK31", "PSK-31", "DIG", "DATA", "JT9", "JT65", "FT8",
                "FT4", "FSQ",
            ],
        ),
        ("DV", vec!["DV", "FUSION", "DSTAR", "D-STAR", "DMR", "C4FM"]),
    ]
});

/// ADIF mode normalization patterns
pub static ADIF_NORMALIZE: Lazy<Vec<(&'static str, Vec<&'static str>)>> = Lazy::new(|| {
    vec![
        ("DIGITALVOICE", vec!["DV"]),
        ("DSTAR", vec!["D-STAR", "FUSION"]),
    ]
});

/// ADIF mode/submode mapping
pub static ADIF_MODE_TABLE: Lazy<Vec<(&'static str, Vec<&'static str>)>> = Lazy::new(|| {
    vec![
        (
            "MFSK",
            vec![
                "FSQCALL", "FST4", "FST4W", "FT4", "JS8", "JTMS", "MFSK4", "MFSK8", "MFSK11",
                "MFSK16", "MFSK22", "MFSK31", "MFSK32", "MFSK64", "MFSK64L", "MFSK128", "MFSK128L",
                "Q65",
            ],
        ),
        (
            "DIGITALVOICE",
            vec!["C4FM", "DMR", "DSTAR", "FREEDV", "M17"],
        ),
    ]
});

/// SOTA のモード(`CW` / `SSB` / `FM` / `AM` / `DATA` / `DV`。どれでもなければ `OTHER`)
pub fn mode_to_sota_mode(mode: &str) -> &'static str {
    let mode_upper = mode.to_uppercase();
    for (sota_mode, patterns) in SOTA_MODE_TABLE.iter() {
        if patterns.iter().any(|p| p.to_uppercase() == mode_upper) {
            return sota_mode;
        }
    }
    "OTHER"
}

/// ADIF のモードとサブモード。例: `FT4` → (`MFSK`, `FT4`)、`CW` → (`CW`, ``)
pub fn mode_to_adif_mode(mode: &str) -> (String, String) {
    let mut smode = mode.to_uppercase();

    // Normalize first
    for (normalized, patterns) in ADIF_NORMALIZE.iter() {
        if patterns.iter().any(|p| p.to_uppercase() == smode) {
            smode = normalized.to_string();
            break;
        }
    }

    // Find mode/submode
    for (adif_mode, patterns) in ADIF_MODE_TABLE.iter() {
        if patterns.iter().any(|p| p.to_uppercase() == smode) {
            return (adif_mode.to_string(), smode);
        }
    }

    (smode, String::new())
}

/// AirHamLog のモード(SSB は、周波数で LSB / USB を分ける)
pub fn mode_to_airham_mode(mode: &str, freq_str: &str) -> String {
    let freq: f64 = freq_str.parse().unwrap_or(0.0);
    let mode_upper = mode.to_uppercase();

    if mode_upper == "SSB" {
        if freq <= 7.2 {
            "SSB(LSB)".to_string()
        } else {
            "SSB(USB)".to_string()
        }
    } else {
        mode_upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_to_sota_mode() {
        assert_eq!(mode_to_sota_mode("CW"), "CW");
        assert_eq!(mode_to_sota_mode("ssb"), "SSB");
        assert_eq!(mode_to_sota_mode("FT8"), "DATA");
        assert_eq!(mode_to_sota_mode("D-STAR"), "DV");
        assert_eq!(mode_to_sota_mode("UNKNOWN"), "OTHER");
    }

    #[test]
    fn test_mode_to_adif_mode() {
        assert_eq!(mode_to_adif_mode("CW"), ("CW".to_string(), String::new()));
        assert_eq!(
            mode_to_adif_mode("FT4"),
            ("MFSK".to_string(), "FT4".to_string())
        );
        assert_eq!(
            mode_to_adif_mode("C4FM"),
            ("DIGITALVOICE".to_string(), "C4FM".to_string())
        );
    }

    #[test]
    fn test_mode_to_airham_mode() {
        assert_eq!(mode_to_airham_mode("SSB", "7.1"), "SSB(LSB)");
        assert_eq!(mode_to_airham_mode("SSB", "14.2"), "SSB(USB)");
        assert_eq!(mode_to_airham_mode("CW", "7.0"), "CW");
    }
}
