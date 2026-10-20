//! 備考・QTH の列の文字列から、参照番号(SOTA・POTA・WWFF)と、場所の注記などを取り出す(旧 FLE も使う)

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// 備考・QTH の列から取り出した参照
///
/// 変換(logconv)が使うのは `sota` / `wwff` / `pota` / `loc`。ほかは、旧 FLE が使う。
#[derive(Debug, Clone, Default)]
pub struct RefInfo {
    pub sota: String,
    pub portable: String,
    pub wwff: Vec<String>,
    pub pota: Vec<String>,
    pub loc: String,
    pub loc_org: String,
    pub sat: String,
    pub sat_oscar: String,
    pub sat_org: String,
    pub sat_down: String,
    pub org: String,
}

/// JA region code to area number mapping
pub static JA_REGION_TABLE: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Area 0
    m.insert("JA/NI", "0");
    m.insert("JA/NN", "0");
    // Area 1
    m.insert("JA/TK", "1");
    m.insert("JA/KN", "1");
    m.insert("JA/CB", "1");
    m.insert("JA/ST", "1");
    m.insert("JA/IB", "1");
    m.insert("JA/TG", "1");
    m.insert("JA/GM", "1");
    m.insert("JA/YN", "1");
    // Area 2
    m.insert("JA/SO", "2");
    m.insert("JA/GF", "2");
    m.insert("JA/AC", "2");
    m.insert("JA/ME", "2");
    // Area 3
    m.insert("JA/KT", "3");
    m.insert("JA/SI", "3");
    m.insert("JA/NR", "3");
    m.insert("JA/OS", "3");
    m.insert("JA/WK", "3");
    m.insert("JA/HG", "3");
    // Area 4
    m.insert("JA/OY", "4");
    m.insert("JA/SN", "4");
    m.insert("JA/YG", "4");
    m.insert("JA/TT", "4");
    m.insert("JA/HS", "4");
    // Area 5
    m.insert("JA5/KA", "5");
    m.insert("JA5/TS", "5");
    m.insert("JA5/EH", "5");
    m.insert("JA5/KC", "5");
    // Area 6
    m.insert("JA6/FO", "6");
    m.insert("JA6/SG", "6");
    m.insert("JA6/NS", "6");
    m.insert("JA6/KM", "6");
    m.insert("JA6/OT", "6");
    m.insert("JA6/MZ", "6");
    m.insert("JA6/KG", "6");
    m.insert("JA6/ON", "6");
    // Area 7
    m.insert("JA/AM", "7");
    m.insert("JA/IT", "7");
    m.insert("JA/AT", "7");
    m.insert("JA/YM", "7");
    m.insert("JA/MG", "7");
    m.insert("JA/FS", "7");
    // Area 8
    m.insert("JA8/SY", "8");
    m.insert("JA8/RM", "8");
    m.insert("JA8/KK", "8");
    m.insert("JA8/OH", "8");
    m.insert("JA8/SC", "8");
    m.insert("JA8/IS", "8");
    m.insert("JA8/NM", "8");
    m.insert("JA8/SB", "8");
    m.insert("JA8/TC", "8");
    m.insert("JA8/KR", "8");
    m.insert("JA8/HD", "8");
    m.insert("JA8/IR", "8");
    m.insert("JA8/HY", "8");
    m.insert("JA8/OM", "8");
    // Area 9
    m.insert("JA/TY", "9");
    m.insert("JA/FI", "9");
    m.insert("JA/IK", "9");
    m
});

/// 参照を見分ける正規表現
static COORD_RE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"(-?[0-9]+(\.[0-9]+)?[nsNS]?\s*,\s*-?[0-9]+(\.[0-9]+)?[ewEW]?)").unwrap()
});
static WWFF_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?i-u)([A-Z0-9]+FF-[0-9]+)").unwrap());
/// 公園の番号は4桁か5桁(`US-10995`)。数字が続けば公園でない(5桁を4桁で切って、別の公園と読まないように)
static POTA_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"([a-zA-Z0-9]+-[0-9]{4,5})(?:[^0-9]|$)").unwrap());
static SOTA_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(([a-zA-Z0-9]+/[a-zA-Z0-9]+)-[0-9]+)").unwrap());
static GRID_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"([a-zA-Z]{2}[0-9]{2}[a-zA-Z]{2})").unwrap());
static SAT_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(([a-zA-Z]+-[0-9]+)/([a-zA-Z]+/([a-zA-Z0-9_]+)))").unwrap());

/// 文字列(空白かカンマで区切った語)から、参照を取り出す。コードは大文字にする
///
/// 語ごとに、WWFF → POTA → SOTA → グリッド → 衛星の順に調べる。どれでもない語は `org` に集める。
pub fn get_ref(s: &str) -> RefInfo {
    let mut info = RefInfo::default();

    // Check for lat/lon coordinates
    if let Some(caps) = COORD_RE.captures(s) {
        info.loc = format!("%QTH%{}% ", &caps[1]);
    }

    // Split by comma or whitespace
    let parts: Vec<&str> = s.split(|c: char| c == ',' || c.is_whitespace()).collect();

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // WWFF reference (e.g., JAFF-0001)
        if let Some(caps) = WWFF_RE.captures(part) {
            info.wwff.push(caps[1].to_uppercase());
            continue;
        }

        // POTA reference (e.g., JA-0001)
        if let Some(caps) = POTA_RE.captures(part) {
            info.pota.push(caps[1].to_uppercase());
            continue;
        }

        // SOTA reference (e.g., JA/TK-001)
        if let Some(caps) = SOTA_RE.captures(part) {
            info.sota = caps[1].to_uppercase();
            let prefix = caps[2].to_uppercase();
            if let Some(area) = JA_REGION_TABLE.get(prefix.as_str()) {
                info.portable = area.to_string();
            } else {
                info.portable = "P".to_string();
            }
            continue;
        }

        // Grid locator (e.g., PM95vq)
        if let Some(caps) = GRID_RE.captures(part) {
            info.loc = format!("%QRA%{}% ", &caps[1]);
            info.loc_org = caps[1].to_string();
            continue;
        }

        // Satellite reference
        if let Some(caps) = SAT_RE.captures(part) {
            info.sat = format!("%SAT%{}%,{}", caps[2].to_uppercase(), &caps[3]);
            info.sat_oscar = caps[2].to_uppercase();
            info.sat_org = caps[1].to_string();
            info.sat_down = caps[4].to_string();
            continue;
        }

        // Unrecognized - add to org
        if !info.org.is_empty() {
            info.org.push(' ');
        }
        info.org.push_str(part);
    }

    info
}

/// `get_ref` の前に、全角の英数字・記号を半角にする(列と、画面で入れた参照を読むとき)
pub fn get_ref_normalized(s: &str) -> RefInfo {
    get_ref(&super::callsign::normalize_width(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ref() {
        let info = get_ref("JA/TK-001");
        assert_eq!(info.sota, "JA/TK-001");
        assert_eq!(info.portable, "1");

        let info = get_ref("JAFF-0001");
        assert_eq!(info.wwff, vec!["JAFF-0001"]);

        let info = get_ref("JA-0001");
        assert_eq!(info.pota, vec!["JA-0001"]);

        let info = get_ref("PM95vq");
        assert!(info.loc.contains("PM95vq"));
        assert_eq!(info.loc_org, "PM95vq");
    }

    #[test]
    fn park_numbers_have_four_or_five_digits() {
        assert_eq!(get_ref("US-10995").pota, vec!["US-10995"]);
        assert_eq!(get_ref("JA-0001 K-0123").pota, vec!["JA-0001", "K-0123"]);
        // 日本語や記号が続いても読む
        assert_eq!(get_ref("JA-0001表").pota, vec!["JA-0001"]);
        assert_eq!(get_ref("(JA-0001)").pota, vec!["JA-0001"]);
        // 6桁以上・3桁以下は公園ではない(5桁や4桁で切って、別の公園と読まない)
        assert!(get_ref("US-109951").pota.is_empty());
        assert!(get_ref("JA-001").pota.is_empty());
    }

    #[test]
    fn several_references_in_one_string() {
        let info = get_ref("ja/kn-001,JA-0123  JA-0124 jaff-0456 (TNX)");
        assert_eq!(info.sota, "JA/KN-001");
        assert_eq!(info.pota, vec!["JA-0123", "JA-0124"]);
        assert_eq!(info.wwff, vec!["JAFF-0456"]);
    }

    #[test]
    fn coordinates_become_the_location_note() {
        assert_eq!(
            get_ref("JA/KN-001 35.6N, 139.2E").loc,
            "%QTH%35.6N, 139.2E% "
        );
    }

    #[test]
    fn nothing_found_in_a_plain_comment() {
        let info = get_ref("Tokyo ありがとう");
        assert!(info.sota.is_empty() && info.pota.is_empty() && info.wwff.is_empty());
        assert!(info.loc.is_empty());
    }
}
