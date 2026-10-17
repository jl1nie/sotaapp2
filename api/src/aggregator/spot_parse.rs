//! 取得したスポット JSON の解析。ネットワークや DB に依存しない純粋な処理(テストしやすいよう分離)。
use common::error::AppResult;
use domain::model::activation::Spot;

use crate::model::spots::{PotaSpot, SotaSpot};

/// 取得した JSON 配列を1件ずつデシリアライズする。
/// 形式が不正な1件のために、同じ応答の他の要素まで捨てないようにする。
fn parse_items<T: serde::de::DeserializeOwned>(body: &[u8], label: &str) -> Vec<T> {
    let items: Vec<serde_json::Value> = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to parse {} response as an array: {:?}", label, e);
            return vec![];
        }
    };
    let total = items.len();
    let parsed: Vec<T> = items
        .into_iter()
        .filter_map(|v| match serde_json::from_value::<T>(v) {
            Ok(t) => Some(t),
            Err(e) => {
                tracing::warn!("Skipped an invalid {} item: {:?}", label, e);
                None
            }
        })
        .collect();
    if parsed.len() != total {
        tracing::warn!("{}: {}/{} items parsed", label, parsed.len(), total);
    }
    parsed
}

/// SOTA の旧スポット API は廃止後、`associationCode: "DEPRECATED"` のダミーを返す。
fn is_deprecated_placeholder(body: &[u8]) -> bool {
    serde_json::from_slice::<Vec<serde_json::Value>>(body)
        .map(|items| {
            !items.is_empty()
                && items.iter().all(|v| {
                    v.get("associationCode").and_then(|c| c.as_str()) == Some("DEPRECATED")
                })
        })
        .unwrap_or(false)
}

pub fn sota_spots_from_body(body: &[u8]) -> Vec<Spot> {
    if is_deprecated_placeholder(body) {
        tracing::warn!(
            "SOTA spot endpoint returned a DEPRECATED placeholder; no SOTA spots updated"
        );
        return vec![];
    }
    parse_items::<SotaSpot>(body, "SOTA spot")
        .into_iter()
        .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
        .collect()
}

pub fn pota_spots_from_body(body: &[u8]) -> Vec<Spot> {
    parse_items::<PotaSpot>(body, "POTA spot")
        .into_iter()
        .filter_map(|ss| AppResult::<Spot>::from(ss).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::model::AwardProgram;

    // SOTA 旧スポット API が実際に返した廃止ダミー(2026-10-17 取得)
    const SOTA_DEPRECATED: &str = r#"[{"id":null,"userID":null,"timeStamp":"2026-10-17T12:22:48","comments":"This API endpoint is deprecated.","callsign":null,"associationCode":"DEPRECATED","summitCode":"DEPRECATED","activatorCallsign":"DEPRECATED","activatorName":null,"frequency":"","mode":"","summitDetails":"DEPRECATED","highlightColor":null}]"#;

    // POTA 本家 API の実際の応答(2026-10-17 取得、日本のスポット)
    const POTA_JP: &str = r#"[{"spotId":58864802,"activator":"JI1IZS/7","frequency":"3512","mode":"CW","reference":"JP-1872","parkName":null,"spotTime":"2026-10-17T12:20:41","spotter":"JI1IZS/7","comments":"","source":"Ham2K Portable Logger","invalid":null,"name":"Rachihama Bosai Ryokuchi Prefectural Park","locationDesc":"JP-FS","grid4":"QM07","grid6":"QM07lv","latitude":37.8871,"longitude":140.932,"count":2,"expire":1699}]"#;

    const SOTA_OK: &str = r#"[{"id":1234,"userID":1,"timeStamp":"2026-10-17T12:00:00","comments":"CQ","callsign":"JA1AAA","associationCode":"JA","summitCode":"TK-001","activatorCallsign":"JA1BBB/P","activatorName":"Taro","frequency":"14.285","mode":"SSB","summitDetails":"Mt. Takao","highlightColor":null}]"#;

    #[test]
    fn deprecated_sota_placeholder_yields_no_spots() {
        assert!(sota_spots_from_body(SOTA_DEPRECATED.as_bytes()).is_empty());
    }

    #[test]
    fn pota_spots_are_parsed_independently_of_sota() {
        // SOTA が廃止ダミーでも、POTA は取得できる(以前は SOTA の失敗で POTA まで止まっていた)
        let mut requests = sota_spots_from_body(SOTA_DEPRECATED.as_bytes());
        requests.extend(pota_spots_from_body(POTA_JP.as_bytes()));
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].program, AwardProgram::POTA);
        assert_eq!(requests[0].reference, "JP-1872");
        assert_eq!(requests[0].activator, "JI1IZS/7");
    }

    #[test]
    fn valid_sota_spots_are_still_parsed() {
        let spots = sota_spots_from_body(SOTA_OK.as_bytes());
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].program, AwardProgram::SOTA);
        assert_eq!(spots[0].reference, "JA/TK-001");
    }

    #[test]
    fn one_invalid_item_does_not_drop_the_others() {
        let body = format!(
            "[{},{{\"spotId\":null}},{}]",
            &POTA_JP[1..POTA_JP.len() - 1],
            POTA_JP[1..POTA_JP.len() - 1].replace("58864802", "58864803")
        );
        let spots = pota_spots_from_body(body.as_bytes());
        assert_eq!(spots.len(), 2);
    }

    #[test]
    fn non_array_or_garbage_yields_no_spots() {
        assert!(pota_spots_from_body(b"{\"error\":\"x\"}").is_empty());
        assert!(pota_spots_from_body(b"<html>").is_empty());
        assert!(sota_spots_from_body(b"").is_empty());
        assert!(pota_spots_from_body(b"[]").is_empty());
    }
}
