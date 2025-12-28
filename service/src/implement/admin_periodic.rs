use aprs_message::AprsData;
use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use csv::ReaderBuilder;
use domain::repository::aprs::AprsLogRepository;
use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use common::{config::AppConfig, error::AppError, error::AppResult};
use domain::model::event::{DeleteRef, FindRefBuilder};
use domain::model::pota::PotaReference;
use domain::model::sota::{SotaReference, SummitCode};
use domain::model::{activation::Alert, activation::Spot, event::DeleteAct};
use domain::repository::{
    activation::ActivationRepositry, aprs::AprsRepositry, pota::PotaRepository,
    sota::SotaRepository,
};

use crate::model::pota::POTAAllCSVFile;
use crate::model::sota::SOTASummitCSV;
use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    pub act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    pub aprs_repo: Arc<dyn AprsRepositry>,
    #[shaku(inject)]
    pub aprs_log_repo: Arc<dyn AprsLogRepository>,
    #[shaku(inject)]
    pub sota_repo: Arc<dyn SotaRepository>,
    #[shaku(inject)]
    pub pota_repo: Arc<dyn PotaRepository>,

    pub config: AppConfig,
}

fn is_valid_summit(r: &SotaReference) -> bool {
    let today = chrono::Local::now().date_naive();
    today <= r.valid_to && today >= r.valid_from
}

/// SotaReferenceの比較対象フィールドからハッシュ値を計算
fn compute_summit_hash(r: &SotaReference) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    r.summit_code.hash(&mut hasher);
    r.activation_count.hash(&mut hasher);
    r.activation_date.hash(&mut hasher);
    r.activation_call.hash(&mut hasher);
    r.association_name.hash(&mut hasher);
    r.region_name.hash(&mut hasher);
    r.alt_ft.hash(&mut hasher);
    r.grid_ref1.hash(&mut hasher);
    r.grid_ref2.hash(&mut hasher);
    r.points.hash(&mut hasher);
    r.bonus_points.hash(&mut hasher);
    r.valid_from.hash(&mut hasher);
    r.valid_to.hash(&mut hasher);
    hasher.finish()
}

/// PotaReferenceの比較対象フィールドからハッシュ値を計算
fn compute_park_hash(r: &PotaReference) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    r.pota_code.hash(&mut hasher);
    r.wwff_code.hash(&mut hasher);
    r.park_name.hash(&mut hasher);
    r.park_name_j.hash(&mut hasher);
    r.park_location.hash(&mut hasher);
    r.park_locid.hash(&mut hasher);
    r.park_type.hash(&mut hasher);
    r.park_inactive.hash(&mut hasher);
    r.park_area.hash(&mut hasher);
    hasher.finish()
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()> {
        let now: DateTime<Utc> = Utc::now();
        let alert_window_start = now - TimeDelta::hours(5);
        let alert_window_end = now + TimeDelta::hours(6);
        let mut buddy: Vec<_> = alerts
            .iter()
            .filter(|a| {
                a.program == domain::model::AwardProgram::SOTA
                    && a.start_time > alert_window_start
                    && a.start_time < alert_window_end
            })
            .map(|a| format!("{}-*", a.operator))
            .collect();

        if let Some(callsign) = self.config.aprs_user.split('-').next() {
            buddy.push(format!("{}-*", callsign));
        }

        self.aprs_repo.set_buddy_list(buddy).await?;

        self.act_repo.update_alerts(alerts).await?;

        let expire = now - self.config.alert_expire;
        self.act_repo
            .delete_alerts(DeleteAct { before: expire })
            .await?;

        let expire = now - self.config.aprs_log_expire;
        self.aprs_log_repo
            .delete_aprs_log(&expire.naive_utc())
            .await?;

        Ok(())
    }

    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()> {
        self.act_repo.update_spots(spots).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.act_repo
            .delete_spots(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn aprs_packet_received(&self, packet: AprsData) -> AppResult<()> {
        match packet {
            AprsData::AprsMessage {
                callsign,
                addressee,
                message,
            } => {
                tracing::info!(
                    "APRS message from = {:?} to = {:} message = {:}",
                    callsign,
                    addressee,
                    message
                );
                return self.process_message(&callsign, message).await;
            }
            AprsData::AprsPosition {
                callsign,
                latitude,
                longitude,
            } => {
                if let Some(ssid) = callsign.ssid {
                    if [5, 6, 7, 8, 9].contains(&ssid) {
                        return self.process_position(callsign, latitude, longitude).await;
                    }
                }
            }
        };
        Ok(())
    }

    async fn update_summit_list_from_file(&self, path: &Path) -> AppResult<usize> {
        // Pass 1: ファイルを読んで軽量データを構築
        let mut valid_hashes: HashMap<String, u64> = HashMap::new();
        let mut invalid_codes: HashSet<String> = HashSet::new();

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .map_err(AppError::CSVReadError)?;

        for result in rdr.records().skip(2) {
            let record = result.map_err(AppError::CSVReadError)?;
            let row: SOTASummitCSV = record.deserialize(None).map_err(AppError::CSVReadError)?;
            let summit = SotaReference::from(row);

            if is_valid_summit(&summit) {
                let hash = compute_summit_hash(&summit);
                valid_hashes.insert(summit.summit_code.clone(), hash);
            } else {
                invalid_codes.insert(summit.summit_code.clone());
            }
        }

        tracing::info!(
            "Pass 1: {} valid summits, {} invalid summits",
            valid_hashes.len(),
            invalid_codes.len()
        );

        // Pass 2: DBと比較して更新/削除対象を特定
        let mut to_update: HashSet<String> = valid_hashes.keys().cloned().collect();
        let mut to_delete: Vec<SummitCode> = Vec::new();

        let limit = 5000;
        let mut offset = 0;

        loop {
            let query = FindRefBuilder::new()
                .sota()
                .limit(limit)
                .offset(offset)
                .build();
            let result = self.sota_repo.find_reference(&query).await?;

            if result.is_empty() {
                break;
            }

            for r in result {
                if let Some(&new_hash) = valid_hashes.get(&r.summit_code) {
                    if compute_summit_hash(&r) == new_hash {
                        to_update.remove(&r.summit_code);
                    }
                } else if invalid_codes.contains(&r.summit_code) {
                    to_delete.push(SummitCode::new(r.summit_code.clone()));
                }
            }
            offset += limit;
        }

        tracing::info!(
            "Pass 2: {} to update, {} to delete",
            to_update.len(),
            to_delete.len()
        );

        // 削除実行
        for code in to_delete {
            self.sota_repo
                .delete_reference(DeleteRef::Delete(code))
                .await?;
        }

        // Pass 3: ファイルを再度読んで更新対象だけ収集
        let mut updates: Vec<SotaReference> = Vec::new();

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .map_err(AppError::CSVReadError)?;

        for result in rdr.records().skip(2) {
            let record = result.map_err(AppError::CSVReadError)?;
            let row: SOTASummitCSV = record.deserialize(None).map_err(AppError::CSVReadError)?;
            let summit = SotaReference::from(row);

            if to_update.contains(&summit.summit_code) {
                updates.push(summit);
            }
        }

        let count = updates.len();
        tracing::info!("Pass 3: upserting {} summits", count);
        self.sota_repo.upsert_reference(updates).await?;

        Ok(count)
    }

    async fn update_pota_park_list_from_file(&self, path: &Path) -> AppResult<usize> {
        // Pass 1: ファイルを読んで軽量データを構築
        let mut park_hashes: HashMap<String, u64> = HashMap::new();

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .map_err(AppError::CSVReadError)?;

        for result in rdr.records().skip(1) {
            let record = result.map_err(AppError::CSVReadError)?;
            let row: POTAAllCSVFile = record.deserialize(None).map_err(AppError::CSVReadError)?;

            if row.reference.starts_with("JP-") {
                continue;
            }

            match PotaReference::try_from(row) {
                Ok(park) => {
                    let hash = compute_park_hash(&park);
                    park_hashes.insert(park.pota_code.clone(), hash);
                }
                Err(_) => continue,
            }
        }

        tracing::info!("Pass 1: {} parks in CSV", park_hashes.len());

        // Pass 2: DBと比較して更新対象を特定
        let mut to_update: HashSet<String> = park_hashes.keys().cloned().collect();

        let limit = 5000;
        let mut offset = 0;

        loop {
            let query = FindRefBuilder::new()
                .pota()
                .limit(limit)
                .offset(offset)
                .build();
            let result = self.pota_repo.find_reference(&query).await?;

            if result.is_empty() {
                break;
            }

            for r in result {
                let park_ref = PotaReference {
                    pota_code: r.pota_code.clone(),
                    wwff_code: r.wwff_code.clone(),
                    park_name: r.park_name.clone(),
                    park_name_j: r.park_name_j.clone(),
                    park_location: r.park_location.clone(),
                    park_locid: r.park_locid.clone(),
                    park_type: r.park_type.clone(),
                    park_inactive: r.park_inactive,
                    park_area: r.park_area,
                    longitude: r.longitude,
                    latitude: r.latitude,
                    maidenhead: r.maidenhead.clone(),
                    update: chrono::Utc::now(),
                };

                if let Some(&new_hash) = park_hashes.get(&r.pota_code) {
                    if compute_park_hash(&park_ref) == new_hash {
                        to_update.remove(&r.pota_code);
                    }
                }
            }
            offset += limit;
        }

        tracing::info!("Pass 2: {} to update", to_update.len());

        // Pass 3: ファイルを再度読んで更新対象だけ収集
        let mut updates: Vec<PotaReference> = Vec::new();

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .map_err(AppError::CSVReadError)?;

        for result in rdr.records().skip(1) {
            let record = result.map_err(AppError::CSVReadError)?;
            let row: POTAAllCSVFile = record.deserialize(None).map_err(AppError::CSVReadError)?;

            if row.reference.starts_with("JP-") {
                continue;
            }

            if let Ok(park) = PotaReference::try_from(row) {
                if to_update.contains(&park.pota_code) {
                    updates.push(park);
                }
            }
        }

        let count = updates.len();
        tracing::info!("Pass 3: upserting {} parks", count);
        self.pota_repo.create_reference(updates).await?;

        Ok(count)
    }
}
