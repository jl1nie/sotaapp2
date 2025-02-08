use async_trait::async_trait;
use chrono::Utc;
use common::config::AppConfig;
use common::csv_reader::csv_reader;
use common::error::AppResult;
use domain::model::id::UserId;
use regex::Regex;
use shaku::Component;
use std::collections::HashMap;
use std::sync::Arc;

use crate::model::pota::{
    POTAActivatorLogCSV, POTAHunterLogCSV, UploadActivatorCSV, UploadHunterCSV,
};
use crate::services::UserService;

use domain::model::activation::{Alert, Spot};
use domain::model::aprslog::AprsLog;
use domain::model::event::{DeleteLog, FindAct, FindAprs, FindRef, FindResult, GroupBy};
use domain::model::geomag::GeomagIndex;
use domain::model::locator::MunicipalityCenturyCode;

use domain::repository::{
    activation::ActivationRepositry, aprs::AprsLogRepository, geomag::GeoMagRepositry,
    locator::LocatorRepositry, pota::POTARepository, sota::SOTARepository,
};

#[derive(Component)]
#[shaku(interface = UserService)]
pub struct UserServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SOTARepository>,
    #[shaku(inject)]
    pota_repo: Arc<dyn POTARepository>,
    #[shaku(inject)]
    act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    locator_repo: Arc<dyn LocatorRepositry>,
    #[shaku(inject)]
    aprs_log_repo: Arc<dyn AprsLogRepository>,
    #[shaku(inject)]
    geomag_repo: Arc<dyn GeoMagRepositry>,
    config: AppConfig,
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

    async fn find_spots(&self, event: FindAct) -> AppResult<HashMap<GroupBy, Vec<Spot>>> {
        let mut result = HashMap::new();
        if event.group_by.is_some() {
            let mut spots = self.act_repo.find_spots(&event).await?;
            if let Some(loc_regex) = &event.pattern {
                let pat = Regex::new(loc_regex);
                if let Ok(pat) = pat {
                    spots.retain(|r| pat.is_match(&r.reference));
                }
            }
            for spot in spots {
                result
                    .entry(get_spot_group(&event, &spot))
                    .or_insert(Vec::new())
                    .push(spot);
            }
        }
        Ok(result)
    }

    async fn upload_activator_csv(
        &self,
        user_id: UserId,
        UploadActivatorCSV { data }: UploadActivatorCSV,
    ) -> AppResult<()> {
        let requests: Vec<POTAActivatorLogCSV> = csv_reader(data, 1)?;
        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| POTAActivatorLogCSV::to_log(user_id, l))
            .collect();
        self.pota_repo.upload_activator_log(newlog).await?;
        self.pota_repo
            .delete_log(DeleteLog {
                before: Utc::now() - self.config.pota_log_expire,
            })
            .await?;
        Ok(())
    }

    async fn upload_hunter_csv(
        &self,
        user_id: UserId,
        UploadHunterCSV { data }: UploadHunterCSV,
    ) -> AppResult<()> {
        let requests: Vec<POTAHunterLogCSV> = csv_reader(data, 1)?;
        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| POTAHunterLogCSV::to_log(user_id, l))
            .collect();
        self.pota_repo.upload_hunter_log(newlog).await?;
        self.pota_repo
            .delete_log(DeleteLog {
                before: Utc::now() - self.config.pota_log_expire,
            })
            .await?;
        Ok(())
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

    async fn find_aprslog(&self, event: FindAprs) -> AppResult<Vec<AprsLog>> {
        if event.callsign.is_some() {
            self.aprs_log_repo
                .get_aprs_log_by_callsign(&event.callsign.unwrap())
                .await
        } else if event.after.is_some() {
            let after = event.after.unwrap().naive_utc();
            self.aprs_log_repo.get_aprs_log_by_time(&after).await
        } else {
            Err(common::error::AppError::APRSError)
        }
    }
}
