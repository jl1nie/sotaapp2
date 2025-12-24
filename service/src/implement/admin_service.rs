use async_trait::async_trait;
use chrono::Local;
use csv::ReaderBuilder;

use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use common::error::{AppError, AppResult};
use common::utils::csv_reader;

use domain::model::event::{DeleteRef, FindRef, FindRefBuilder, PagenatedResult};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{ParkCode, PotaReference};
use domain::model::sota::{SotaReference, SummitCode};
use domain::repository::{
    healthcheck::HealthCheckRepositry, locator::LocatorRepositry, pota::PotaRepository,
    sota::SotaRepository,
};

use crate::model::locator::{MuniCSVFile, UploadMuniCSV};
use crate::model::pota::{POTAAllCSVFile, POTACSVFile, UploadPOTAReference};
use crate::model::sota::{SOTASumitOptCSV, SOTASummitCSV};
use crate::model::sota::{UploadSOTASummit, UploadSOTASummitOpt};

use crate::services::AdminService;

#[derive(Component)]
#[shaku(interface = AdminService)]
pub struct AdminServiceImpl {
    #[shaku(inject)]
    sota_repo: Arc<dyn SotaRepository>,
    #[shaku(inject)]
    pota_repo: Arc<dyn PotaRepository>,
    #[shaku(inject)]
    check_repo: Arc<dyn HealthCheckRepositry>,
    #[shaku(inject)]
    loc_repo: Arc<dyn LocatorRepositry>,
}

fn is_valid_summit(r: &SotaReference) -> bool {
    let today = Local::now().date_naive();
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
impl AdminService for AdminServiceImpl {
    async fn import_summit_list(
        &self,
        UploadSOTASummit { data }: UploadSOTASummit,
    ) -> AppResult<usize> {
        let csv: Vec<SOTASummitCSV> = csv_reader(data, false, 2)?;
        let req: Vec<_> = csv
            .into_iter()
            .map(SotaReference::from)
            .filter(is_valid_summit)
            .collect();

        let count = req.len();
        tracing::info!("import {} references.", count);
        self.sota_repo
            .delete_reference(DeleteRef::DeleteAll)
            .await?;

        self.sota_repo.create_reference(req).await?;

        Ok(count)
    }

    async fn update_summit_list(
        &self,
        UploadSOTASummit { data }: UploadSOTASummit,
    ) -> AppResult<usize> {
        let partial_equal = |r: &SotaReference, other: &SotaReference| {
            r.activation_count == other.activation_count
                && r.activation_date == other.activation_date
                && r.activation_call == other.activation_call
                && r.summit_code == other.summit_code
                && r.association_name == other.association_name
                && r.region_name == other.region_name
                && r.alt_ft == other.alt_ft
                && r.grid_ref1 == other.grid_ref1
                && r.grid_ref2 == other.grid_ref2
                && r.points == other.points
                && r.bonus_points == other.bonus_points
                && r.valid_from == other.valid_from
                && r.valid_to == other.valid_to
        };

        let csv: Vec<SOTASummitCSV> = csv_reader(data, false, 2)?;

        tracing::info!("Latest summit list length = {}", csv.len());

        let mut new_hash: HashMap<_, _> = csv
            .into_iter()
            .map(SotaReference::from)
            .filter(is_valid_summit)
            .map(|r| (r.summit_code.clone(), r))
            .collect();

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
                let n = new_hash.get(&r.summit_code);
                if let Some(summit) = n {
                    if partial_equal(&r, summit) {
                        new_hash.remove(&r.summit_code);
                    }
                }
            }
            offset += limit;
        }

        let updated: Vec<_> = new_hash.into_values().collect();
        let count = updated.len();

        tracing::info!("update {} summits.", count);
        self.sota_repo.upsert_reference(updated).await?;

        Ok(count)
    }

    async fn update_summit_list_from_file(&self, path: &Path) -> AppResult<usize> {
        // Pass 1: ファイルを読んで軽量データを構築
        // - valid_hashes: HashMap<summit_code, hash> - 有効なサミットのハッシュ
        // - invalid_codes: HashSet<summit_code> - 無効になったサミット
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
            // summitはここでdrop → メモリ解放
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
                    // CSVにあって有効 → ハッシュ比較
                    if compute_summit_hash(&r) == new_hash {
                        // 変更なし → 更新対象から除外
                        to_update.remove(&r.summit_code);
                    }
                } else if invalid_codes.contains(&r.summit_code) {
                    // CSVにあるが無効になった → 削除対象
                    to_delete.push(SummitCode::new(r.summit_code.clone()));
                }
                // CSVにない → 別アソシエーション等、そのまま残す
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

    async fn import_summit_opt_list(
        &self,
        UploadSOTASummitOpt { data }: UploadSOTASummitOpt,
    ) -> AppResult<usize> {
        let csv: Vec<SOTASumitOptCSV> = csv_reader(data, false, 1)?;

        let ja_hash: HashMap<_, _> = csv
            .into_iter()
            .map(|r| (r.summit_code.clone(), r))
            .collect();

        let associations: HashSet<String> = ja_hash
            .keys()
            .map(|s| s.split("/").next().unwrap_or("").to_owned() + "/")
            .collect();

        let mut total_count = 0;
        for assoc in associations {
            let query = FindRefBuilder::new().sota().name(assoc).build();
            let result = self.sota_repo.find_reference(&query).await?;
            let newref: Vec<_> = result
                .into_iter()
                .filter(|r| ja_hash.contains_key(&r.summit_code))
                .filter_map(|mut r| {
                    // filterで確認済みなのでget()は成功するはずだが、安全のためfilter_mapを使用
                    let ja = ja_hash.get(&r.summit_code)?;
                    r.summit_name = ja.summit_name.clone();
                    r.summit_name_j = Some(ja.summit_name_j.clone());
                    r.city = Some(ja.city.clone());
                    r.city_j = Some(ja.city_j.clone());
                    r.longitude = ja.longitude;
                    r.latitude = ja.latitude;
                    r.alt_m = ja.alt_m;
                    Some(r)
                })
                .collect();
            total_count += newref.len();
            self.sota_repo.update_reference(newref).await?;
        }

        Ok(total_count)
    }

    async fn import_pota_park_list(
        &self,
        UploadPOTAReference { data }: UploadPOTAReference,
    ) -> AppResult<usize> {
        let requests: Vec<POTAAllCSVFile> = csv_reader(data, false, 1)?;
        let newref: Vec<_> = requests
            .into_iter()
            .filter_map(|r| PotaReference::try_from(r).ok())
            .filter(|r| !r.pota_code.starts_with("JP-"))
            .collect();

        let count = newref.len();
        tracing::info!("update {} parks.", count);
        self.pota_repo.create_reference(newref).await?;

        Ok(count)
    }

    async fn update_pota_park_list_from_file(&self, path: &Path) -> AppResult<usize> {
        // Pass 1: ファイルを読んで軽量データを構築
        // - park_hashes: HashMap<pota_code, hash> - 全公園のハッシュ（inactive含む）
        // 注: inactiveは一時的なメンテナンス状態のため削除しない
        let mut park_hashes: HashMap<String, u64> = HashMap::new();

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .map_err(AppError::CSVReadError)?;

        for result in rdr.records().skip(1) {
            let record = result.map_err(AppError::CSVReadError)?;
            let row: POTAAllCSVFile = record.deserialize(None).map_err(AppError::CSVReadError)?;

            // JP-で始まるものは除外（別途JAリストで管理）
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
                // PotaRefLogからPotaReferenceに変換してハッシュ計算
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
                // CSVにない公園はそのまま残す（手動追加の可能性）
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

    async fn import_pota_park_list_ja(
        &self,
        UploadPOTAReference { data }: UploadPOTAReference,
    ) -> AppResult<usize> {
        let requests: Vec<POTACSVFile> = csv_reader(data, false, 1)?;
        let newref: Vec<_> = requests.into_iter().map(PotaReference::from).collect();

        let count = newref.len();
        tracing::info!("update {} JA parks.", count);
        self.pota_repo.create_reference(newref).await?;

        Ok(count)
    }

    async fn import_muni_century_list(
        &self,
        UploadMuniCSV { data }: UploadMuniCSV,
    ) -> AppResult<usize> {
        let requests: Vec<MuniCSVFile> = csv_reader(data, false, 1)?;
        let newtable: Vec<_> = requests
            .into_iter()
            .map(MunicipalityCenturyCode::from)
            .collect();
        let count = newtable.len();
        self.loc_repo.upload_muni_century_list(newtable).await?;

        Ok(count)
    }

    async fn show_sota_reference(&self, event: FindRef) -> AppResult<SotaReference> {
        Ok(self.sota_repo.show_reference(&event).await?)
    }

    async fn show_all_sota_references(
        &self,
        event: FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>> {
        Ok(self.sota_repo.show_all_references(&event).await?)
    }

    async fn update_sota_reference(&self, references: Vec<SotaReference>) -> AppResult<()> {
        self.sota_repo.update_reference(references).await?;
        Ok(())
    }

    async fn delete_sota_reference(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        self.sota_repo.delete_reference(event).await?;
        Ok(())
    }

    async fn show_pota_reference(&self, event: FindRef) -> AppResult<PotaReference> {
        Ok(self.pota_repo.show_reference(&event).await?)
    }

    async fn show_all_pota_references(
        &self,
        event: FindRef,
    ) -> AppResult<PagenatedResult<PotaReference>> {
        Ok(self.pota_repo.show_all_references(&event).await?)
    }

    async fn update_pota_reference(&self, references: Vec<PotaReference>) -> AppResult<()> {
        self.pota_repo.update_reference(references).await?;
        Ok(())
    }

    async fn delete_pota_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        self.pota_repo.delete_reference(event).await?;
        Ok(())
    }
    async fn health_check(&self) -> AppResult<bool> {
        Ok(self.check_repo.check_database().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::model::sota::SotaReference;

    /// is_valid_summit関数のテスト用ヘルパー
    fn make_test_summit(valid_from: NaiveDate, valid_to: NaiveDate) -> SotaReference {
        SotaReference {
            summit_code: "JA/TK-001".to_string(),
            association_name: "Japan".to_string(),
            region_name: "Tokyo".to_string(),
            summit_name: "Mt. Test".to_string(),
            summit_name_j: None,
            alt_m: 1000,
            alt_ft: 3280,
            grid_ref1: "PM95".to_string(),
            grid_ref2: "".to_string(),
            longitude: 139.0,
            latitude: 35.0,
            maidenhead: "PM95wv".to_string(),
            points: 10,
            bonus_points: 0,
            valid_from,
            valid_to,
            activation_count: 0,
            activation_date: None,
            activation_call: None,
            city: None,
            city_j: None,
        }
    }

    #[test]
    fn test_is_valid_summit_currently_valid() {
        let today = Local::now().date_naive();
        let valid_from = today - chrono::Duration::days(30);
        let valid_to = today + chrono::Duration::days(30);
        let summit = make_test_summit(valid_from, valid_to);

        assert!(is_valid_summit(&summit));
    }

    #[test]
    fn test_is_valid_summit_today_is_start_date() {
        let today = Local::now().date_naive();
        let valid_to = today + chrono::Duration::days(30);
        let summit = make_test_summit(today, valid_to);

        assert!(is_valid_summit(&summit));
    }

    #[test]
    fn test_is_valid_summit_today_is_end_date() {
        let today = Local::now().date_naive();
        let valid_from = today - chrono::Duration::days(30);
        let summit = make_test_summit(valid_from, today);

        assert!(is_valid_summit(&summit));
    }

    #[test]
    fn test_is_valid_summit_expired() {
        let today = Local::now().date_naive();
        let valid_from = today - chrono::Duration::days(60);
        let valid_to = today - chrono::Duration::days(30);
        let summit = make_test_summit(valid_from, valid_to);

        assert!(!is_valid_summit(&summit));
    }

    #[test]
    fn test_is_valid_summit_not_yet_valid() {
        let today = Local::now().date_naive();
        let valid_from = today + chrono::Duration::days(30);
        let valid_to = today + chrono::Duration::days(60);
        let summit = make_test_summit(valid_from, valid_to);

        assert!(!is_valid_summit(&summit));
    }
}
