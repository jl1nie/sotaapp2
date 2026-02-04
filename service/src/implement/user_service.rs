use async_trait::async_trait;
use domain::model::AwardProgram;
use regex::Regex;
use shaku::Component;
use std::collections::HashMap;
use std::sync::Arc;

use crate::services::UserService;
use common::error::AppResult;
use domain::model::activation::{Alert, Spot, SpotLog};
use domain::model::aprslog::{AprsLog, AprsTrack};
use domain::model::event::{FindAct, FindAprs, FindRef, FindRefBuilder, FindResult, GroupBy};
use domain::model::geomag::GeomagIndex;
use domain::model::locator::MunicipalityCenturyCode;
use domain::repository::{
    activation::ActivationRepositry, aprs::AprsLogRepository, geomag::GeoMagRepositry,
    locator::LocatorRepositry, pota::PotaRepository, sota::SotaRepository,
};

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SotaRepository>,
    #[shaku(inject)]
    pota_repo: Arc<dyn PotaRepository>,
    #[shaku(inject)]
    pub act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    locator_repo: Arc<dyn LocatorRepositry>,
    #[shaku(inject)]
    pub aprs_log_repo: Arc<dyn AprsLogRepository>,
    #[shaku(inject)]
    geomag_repo: Arc<dyn GeoMagRepositry>,
}

fn get_alert_group(event: &FindAct, r: &Alert) -> GroupBy {
    if let Some(g) = &event.group_by {
        match g {
            GroupBy::Callsign(_) => GroupBy::Callsign(Some(r.activator.clone())),
            GroupBy::Reference(_) => GroupBy::Reference(Some(r.reference.clone())),
        }
    } else {
        GroupBy::Callsign(None)
    }
}

fn get_spot_group(event: &FindAct, r: &Spot) -> GroupBy {
    if let Some(g) = &event.group_by {
        match g {
            GroupBy::Callsign(_) => GroupBy::Callsign(Some(r.activator.clone())),
            GroupBy::Reference(_) => GroupBy::Reference(Some(r.reference.clone())),
        }
    } else {
        GroupBy::Callsign(None)
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn count_references(&self, event: &FindRef) -> AppResult<i64> {
        let mut result = 0i64;

        if event.is_sota() {
            result += self.sota_repo.count_reference(event).await?;
        }

        if event.is_pota() {
            result += self.pota_repo.count_reference(event).await?;
        }

        Ok(result)
    }

    async fn find_references(&self, event: FindRef) -> AppResult<FindResult> {
        let mut result = FindResult::default();

        if event.is_sota() {
            let sota_ref = self.sota_repo.find_reference(&event).await?;
            result.sota = Some(sota_ref)
        }

        if event.is_pota() {
            let active_ref: Vec<_> = self
                .pota_repo
                .find_reference(&event)
                .await?
                .into_iter()
                .filter(|r| !r.park_inactive)
                .collect();
            result.pota = Some(active_ref)
        }

        Ok(result)
    }

    async fn find_alerts(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<Alert>>> {
        let mut result = HashMap::new();
        if event.group_by.is_some() {
            let mut alerts = self.act_repo.find_alerts(&event).await?;
            if let Some(loc_regex) = &event.pattern {
                let pat = Regex::new(loc_regex);
                if let Ok(pat) = pat {
                    alerts.retain(|r| pat.is_match(&r.location));
                }
            }
            for alert in alerts {
                result
                    .entry(get_alert_group(&event, &alert))
                    .or_insert(Vec::new())
                    .push(alert);
            }
        }
        Ok(result)
    }

    async fn find_spots(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<SpotLog>>> {
        let mut result = HashMap::new();
        if event.group_by.is_some() {
            let mut spots = self.act_repo.find_spots(&event).await?;

            if let Some(loc_regex) = &event.pattern {
                let pat = Regex::new(loc_regex);
                if let Ok(pat) = pat {
                    spots.retain(|r| pat.is_match(&r.reference));
                }
            }
            let mut pota_hash: HashMap<String, SpotLog> = HashMap::new();

            for spot in spots {
                let mut spotlog = SpotLog::new(spot.clone(), None);

                match get_spot_group(&event, &spot) {
                    GroupBy::Callsign(_) => {
                        result
                            .entry(get_spot_group(&event, &spot))
                            .or_insert(Vec::new())
                            .push(spotlog);
                    }
                    GroupBy::Reference(code) => {
                        if let Some(log_id) = &event.log_id {
                            if spot.program == AwardProgram::POTA {
                                let code = code.unwrap_or_default();
                                if let Some(v) = pota_hash.get(&code) {
                                    spotlog.qsos = v.qsos;
                                } else {
                                    let builder = FindRefBuilder::default();
                                    let query = builder
                                        .pota()
                                        .pota_code(code.clone())
                                        .log_id(*log_id)
                                        .build();

                                    let parks = self.find_references(query).await?;
                                    if let FindResult { pota: Some(p), .. } = parks {
                                        if let Some(pota) = p.first() {
                                            spotlog.qsos = pota.qsos;
                                            pota_hash.insert(code, spotlog.clone());
                                        }
                                    }
                                }
                            }
                            result
                                .entry(get_spot_group(&event, &spot))
                                .or_insert(Vec::new())
                                .push(spotlog);
                        } else {
                            result
                                .entry(get_spot_group(&event, &spot))
                                .or_insert(Vec::new())
                                .push(spotlog);
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    async fn find_century_code(&self, muni_code: i32) -> AppResult<MunicipalityCenturyCode> {
        let result = self
            .locator_repo
            .find_location_by_muni_code(muni_code)
            .await?;
        Ok(result)
    }

    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String> {
        Ok(self.locator_repo.find_mapcode(lon, lat).await?)
    }

    async fn get_geomagnetic(&self) -> AppResult<Option<GeomagIndex>> {
        Ok(self.geomag_repo.get_geomag().await?)
    }

    async fn find_aprs_log(&self, event: FindAprs) -> AppResult<Vec<AprsLog>> {
        Ok(self.aprs_log_repo.find_aprs_log(&event).await?)
    }

    async fn get_aprs_track(&self, event: FindAprs) -> AppResult<Vec<AprsTrack>> {
        let aprslog = self.find_aprs_log(event).await?;
        self.generate_track(aprslog).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use domain::model::activation::Spot;
    use domain::model::event::{FindActBuilder, GroupBy};
    use domain::model::AwardProgram;

    /// テスト用Alertを生成するヘルパー
    fn make_test_alert(activator: &str, reference: &str) -> Alert {
        Alert {
            program: AwardProgram::SOTA,
            alert_id: 1,
            user_id: 1,
            activator: activator.to_string(),
            activator_name: None,
            operator: activator.to_string(),
            reference: reference.to_string(),
            reference_detail: "Test Summit".to_string(),
            location: "Tokyo".to_string(),
            start_time: Utc::now(),
            end_time: None,
            frequencies: "14.280".to_string(),
            comment: Some("Test".to_string()),
            poster: Some(activator.to_string()),
        }
    }

    /// テスト用Spotを生成するヘルパー
    fn make_test_spot(activator: &str, reference: &str) -> Spot {
        Spot {
            program: AwardProgram::SOTA,
            spot_id: 1,
            activator: activator.to_string(),
            activator_name: None,
            operator: activator.to_string(),
            reference: reference.to_string(),
            reference_detail: "Test Summit".to_string(),
            spot_time: Utc::now(),
            frequency: "14.280".to_string(),
            mode: "SSB".to_string(),
            spotter: "JA2XYZ".to_string(),
            comment: Some("Test".to_string()),
        }
    }

    // ==================== ヘルパー関数テスト ====================

    #[test]
    fn test_get_alert_group_by_callsign() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_callsign(Some("JA1ABC".to_string()))
            .build();

        let alert = make_test_alert("JA1ABC", "JA/TK-001");
        let group = get_alert_group(&event, &alert);

        match group {
            GroupBy::Callsign(Some(call)) => assert_eq!(call, "JA1ABC"),
            _ => panic!("Expected GroupBy::Callsign"),
        }
    }

    #[test]
    fn test_get_alert_group_by_reference() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_reference(Some("JA/TK-001".to_string()))
            .build();

        let alert = make_test_alert("JA1ABC", "JA/TK-001");
        let group = get_alert_group(&event, &alert);

        match group {
            GroupBy::Reference(Some(ref_code)) => assert_eq!(ref_code, "JA/TK-001"),
            _ => panic!("Expected GroupBy::Reference"),
        }
    }

    #[test]
    fn test_get_alert_group_no_group_defaults_to_callsign() {
        let event = FindActBuilder::default().sota().build();
        let alert = make_test_alert("JA1ABC", "JA/TK-001");
        let group = get_alert_group(&event, &alert);

        match group {
            GroupBy::Callsign(None) => {} // デフォルトはCallsign(None)
            _ => panic!("Expected GroupBy::Callsign(None)"),
        }
    }

    #[test]
    fn test_get_spot_group_by_callsign() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_callsign(Some("JA1ABC".to_string()))
            .build();

        let spot = make_test_spot("JA1ABC", "JA/TK-001");
        let group = get_spot_group(&event, &spot);

        match group {
            GroupBy::Callsign(Some(call)) => assert_eq!(call, "JA1ABC"),
            _ => panic!("Expected GroupBy::Callsign"),
        }
    }

    #[test]
    fn test_get_spot_group_by_reference() {
        let event = FindActBuilder::default()
            .sota()
            .group_by_reference(Some("JA/TK-001".to_string()))
            .build();

        let spot = make_test_spot("JA1ABC", "JA/TK-001");
        let group = get_spot_group(&event, &spot);

        match group {
            GroupBy::Reference(Some(ref_code)) => assert_eq!(ref_code, "JA/TK-001"),
            _ => panic!("Expected GroupBy::Reference"),
        }
    }
}
