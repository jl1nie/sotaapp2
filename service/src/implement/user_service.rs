use aprs_message::AprsCallsign;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use regex::Regex;
use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use crate::model::pota::{POTAActivatorLogCSV, POTAHunterLogCSV, UploadPOTALog};
use crate::model::sota::{SOTALogCSV, UploadSOTALog};
use crate::services::UserService;
use common::config::AppConfig;
use common::error::AppResult;
use common::utils::{call_to_operator, csv_reader};
use domain::model::activation::{Alert, Spot};
use domain::model::aprslog::AprsLog;
use domain::model::event::{DeleteLog, FindAct, FindAprs, FindLog, FindRef, FindResult, GroupBy};
use domain::model::geomag::GeomagIndex;
use domain::model::id::{LogId, UserId};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{POTALogKind, POTALogUser};
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

    async fn upload_pota_log(
        &self,
        UploadPOTALog {
            activator_logid,
            hunter_logid,
            data,
        }: UploadPOTALog,
    ) -> AppResult<POTALogUser> {
        let logid = if data.contains("Attempts") {
            tracing::info!("Upload activator log");
            activator_logid
        } else {
            tracing::info!("Upload hunter log");
            hunter_logid
        };

        let log_id = LogId::from_str(&logid).unwrap_or(LogId::default());

        let mut update_id = self.pota_repo.find_logid(log_id).await;

        if let Ok(ref mut id) = update_id {
            let expire = Utc::now() - self.config.pota_log_expire;
            if id.update < expire.naive_utc() {
                let query = DeleteLog {
                    log_id: Some(id.log_id),
                    ..Default::default()
                };
                self.pota_repo.delete_log(query).await?;
                *id = POTALogUser::new(None);
                self.pota_repo.update_logid(id.clone()).await?;
            }
        } else {
            update_id = Ok(POTALogUser::new(None));
        }

        let mut update_id = update_id?;
        let log_id = update_id.log_id;

        if data.contains("Attempts") {
            let requests: Vec<POTAActivatorLogCSV> = csv_reader(data, true, 1)?;

            let newlog: Vec<_> = requests
                .into_iter()
                .map(|l| POTAActivatorLogCSV::to_log(log_id, l))
                .collect();

            tracing::info!("Upload activator log {} entries", newlog.len());
            self.pota_repo.upload_activator_log(newlog).await?;

            update_id.log_kind = Some(POTALogKind::ActivatorLog);
        } else {
            let requests: Vec<POTAHunterLogCSV> = csv_reader(data, false, 1)?;

            let newlog: Vec<_> = requests
                .into_iter()
                .map(|l| POTAHunterLogCSV::to_log(log_id, l))
                .collect();

            tracing::info!("Upload hunter log {} entries", newlog.len());
            self.pota_repo.upload_hunter_log(newlog).await?;

            update_id.log_kind = Some(POTALogKind::HunterLog);
        }

        self.pota_repo.update_logid(update_id.clone()).await?;

        self.pota_repo
            .delete_log(DeleteLog {
                before: Some(Utc::now() - self.config.pota_log_expire),
                ..Default::default()
            })
            .await?;

        Ok(update_id)
    }

    async fn find_logid(&self, log_id: LogId) -> AppResult<POTALogUser> {
        self.pota_repo.find_logid(log_id).await
    }

    async fn delete_pota_log(&self, log_id: LogId) -> AppResult<()> {
        self.pota_repo
            .delete_log(DeleteLog {
                log_id: Some(log_id),
                ..Default::default()
            })
            .await?;
        Ok(())
    }

    async fn upload_sota_csv(
        &self,
        user_id: UserId,
        UploadSOTALog { data }: UploadSOTALog,
    ) -> AppResult<()> {
        let requests: Vec<SOTALogCSV> = csv_reader(data, false, 0)?;

        let from = Utc.with_ymd_and_hms(2024, 7, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();

        let newlog: Vec<_> = requests
            .into_iter()
            .map(|l| SOTALogCSV::to_log(user_id, l))
            .filter(|l| l.time >= from && l.time < to)
            .collect();
        self.sota_repo.upload_log(newlog).await?;
        Ok(())
    }

    async fn delete_sota_log(&self, _user_id: UserId) -> AppResult<()> {
        self.sota_repo
            .delete_log(DeleteLog {
                before: Some(Utc::now()),
                ..Default::default()
            })
            .await?;
        Ok(())
    }

    async fn award_progress(&self, _user_id: UserId, mut query: FindLog) -> AppResult<String> {
        let mut response = String::new();

        let after = query.after.unwrap_or_default();
        let before = query.before.unwrap_or_default();

        query.activation = true;
        let act_log = self.sota_repo.find_log(&query).await?;
        query.activation = false;
        let chase_log = self.sota_repo.find_log(&query).await?;

        let mut act_hash = HashMap::new();

        for a in act_log {
            let my_summit_code = a.my_summit_code.unwrap();
            let act_count = act_hash.entry((a.operator, my_summit_code)).or_insert(0);
            *act_count += 1;
        }

        let mut chase_summit_hash = HashMap::new();
        let mut chase_operator_hash = HashMap::new();
        let mut chase_callsign_hash = HashMap::new();

        for c in chase_log {
            let my_operator = c.operator.clone();
            let his_summit_code = c.his_summit_code.unwrap();
            let his_operator = call_to_operator(&c.his_callsign);
            let his_callsign = c.his_callsign.clone();

            let chase_count = chase_summit_hash
                .entry(c.operator)
                .or_insert(HashSet::new());
            chase_count.insert(his_summit_code);

            let chase_op_count = chase_operator_hash
                .entry(my_operator.clone())
                .or_insert(HashSet::new());
            chase_op_count.insert(his_operator);

            let chase_call_count = chase_callsign_hash
                .entry(my_operator.clone())
                .or_insert(HashSet::new());
            chase_call_count.insert(his_callsign);
        }

        let act_result: Vec<_> = act_hash
            .into_iter()
            .filter(|&(_, count)| count >= 10)
            .map(|((call, summit), count)| (call, format!("{} {} qsos", summit, count)))
            .collect();

        let mut act_hash = HashMap::new();
        for (call, summit) in act_result {
            act_hash.entry(call).or_insert(Vec::new()).push(summit);
        }

        let act_result: Vec<_> = act_hash
            .into_iter()
            .filter(|(_, summits)| summits.len() >= 10)
            .collect();

        response.push_str(&format!("集計期間 {} - {}\n", after, before));

        for a in act_result {
            response.push_str(&format!(
                "アクティベータ：{} activate {} summits ",
                a.0,
                a.1.len()
            ));
            for s in a.1 {
                response.push_str(&format!("{} ", s));
            }
            response.push('\n');
        }

        let chase_summit10: Vec<_> = chase_summit_hash
            .clone()
            .into_iter()
            .filter(|(_, h)| h.len() >= 10)
            .map(|(call, h)| (call, format!("{} summits", h.len())))
            .collect();

        let chase_op10: Vec<_> = chase_operator_hash
            .clone()
            .into_iter()
            .filter(|(_, h)| h.len() >= 10)
            .map(|(call, h)| (call, format!("{} operators", h.len())))
            .collect();

        let chase_call10: Vec<_> = chase_callsign_hash
            .clone()
            .into_iter()
            .filter(|(_, h)| h.len() >= 10)
            .map(|(call, h)| (call, format!("{} stations", h.len())))
            .collect();

        for op in chase_op10 {
            if let Some((_, summit)) = chase_summit10.iter().find(|&item| item.0 == op.0) {
                response.push_str(&format!(
                    "チェイサー10x10(operator) {} {} {}\n",
                    op.0, op.1, summit
                ));
            }
        }

        for call in chase_call10 {
            if let Some((_, summit)) = chase_summit10.iter().find(|&item| item.0 == call.0) {
                response.push_str(&format!(
                    "チェイサー10x10(callsign) {} {} {}\n",
                    call.0, call.1, summit
                ));
            }
        }
        tracing::info!("Result = {}", response);

        Ok(response)
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
            let callsign = AprsCallsign::from(&event.callsign.unwrap());
            self.aprs_log_repo.get_aprs_log_by_callsign(&callsign).await
        } else if event.after.is_some() {
            let after = event.after.unwrap().naive_utc();
            self.aprs_log_repo.get_aprs_log_by_time(&after).await
        } else {
            Err(common::error::AppError::APRSError)
        }
    }
}
