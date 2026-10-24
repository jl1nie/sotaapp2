use async_trait::async_trait;
use chrono::Local;

use shaku::Component;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use common::error::{AppError, AppResult};
use common::utils::csv_reader;

use domain::model::event::{DeleteRef, FindRef, FindRefBuilder, PagenatedResult};
use domain::model::locator::MunicipalityCenturyCode;
use domain::model::pota::{ParkCode, PotaReference};
use domain::model::sota::{SotaReference, SummitCode, SummitMap, SummitPota};
use domain::repository::{
    healthcheck::HealthCheckRepositry, locator::LocatorRepositry, pota::PotaRepository,
    sota::SotaRepository,
};

use crate::model::locator::{MuniCSVFile, UploadMuniCSV};
use crate::model::pota::{POTAAllCSVFile, POTACSVFile, POTAExportRow, UploadPOTAReference};
use crate::model::sota::{
    parse_summit_map_line, SummitPotaCSV, UploadSOTASummit, UploadSOTASummitOpt, UploadSummitMaps,
    UploadSummitPota,
};
use crate::model::sota::{SOTAExportRow, SOTASumitOptCSV, SOTASummitCSV};

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

    async fn import_summit_pota(
        &self,
        UploadSummitPota { data }: UploadSummitPota,
    ) -> AppResult<usize> {
        let csv: Vec<SummitPotaCSV> = csv_reader(data, false, 1)?;
        let rows = csv
            .into_iter()
            .map(SummitPota::try_from)
            .collect::<AppResult<Vec<_>>>()?;
        let count = rows.len();
        self.sota_repo.replace_summit_pota(rows).await?;
        Ok(count)
    }

    async fn import_summit_maps(
        &self,
        UploadSummitMaps { data }: UploadSummitMaps,
    ) -> AppResult<usize> {
        // 全部で数十MBになるので、まとめて展開せず、1 行ずつ読んで塊ごとに入れる。
        // 途中の行が壊れていたら何も入れないように、先に全行を確かめてから入れる
        let mut count = 0;
        for m in summit_map_lines(&data) {
            m?;
            count += 1;
        }
        let mut lines = summit_map_lines(&data);
        loop {
            let chunk = lines
                .by_ref()
                .take(MAP_CHUNK)
                .collect::<AppResult<Vec<_>>>()?;
            if chunk.is_empty() {
                break;
            }
            self.sota_repo.upsert_summit_maps(chunk).await?;
        }
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

    async fn create_pota_reference(&self, references: Vec<PotaReference>) -> AppResult<()> {
        self.pota_repo.create_reference(references).await?;
        Ok(())
    }

    async fn update_pota_reference(
        &self,
        id: String,
        references: Vec<PotaReference>,
    ) -> AppResult<()> {
        self.pota_repo.update_reference(&id, references).await?;
        Ok(())
    }

    async fn delete_pota_reference(&self, event: DeleteRef<ParkCode>) -> AppResult<()> {
        self.pota_repo.delete_reference(event).await?;
        Ok(())
    }

    async fn export_pota_parks_csv(&self) -> AppResult<String> {
        let limit = 5000;
        let mut offset = 0;
        let mut wtr = csv::Writer::from_writer(vec![]);

        loop {
            let query = FindRefBuilder::new()
                .pota()
                .pota_code("JP-".to_string())
                .limit(limit)
                .offset(offset)
                .build();
            let result = self.pota_repo.show_all_references(&query).await?;
            let is_last = result.results.len() < limit as usize;
            for park in result.results {
                wtr.serialize(POTAExportRow::from(park))
                    .map_err(|e| common::error::AppError::ConversionEntityError(e.to_string()))?;
            }
            if is_last {
                break;
            }
            offset += limit;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| common::error::AppError::ConversionEntityError(e.to_string()))?;
        String::from_utf8(data)
            .map_err(|e| common::error::AppError::ConversionEntityError(e.to_string()))
    }

    async fn export_sota_summits_csv(&self) -> AppResult<String> {
        let limit = 5000;
        let mut offset = 0;
        let mut wtr = csv::Writer::from_writer(vec![]);

        loop {
            // JA系アソシエーション全体 (JA, JA5, JA6, JA7, JA8, JA9) を対象
            let query = FindRefBuilder::new()
                .sota()
                .association("JA".to_string())
                .association("JA5".to_string())
                .association("JA6".to_string())
                .association("JA7".to_string())
                .association("JA8".to_string())
                .association("JA9".to_string())
                .limit(limit)
                .offset(offset)
                .build();
            let result = self.sota_repo.show_all_references(&query).await?;
            let is_last = result.results.len() < limit as usize;
            for summit in result.results {
                wtr.serialize(SOTAExportRow::from(summit))
                    .map_err(|e| common::error::AppError::ConversionEntityError(e.to_string()))?;
            }
            if is_last {
                break;
            }
            offset += limit;
        }

        let data = wtr
            .into_inner()
            .map_err(|e| common::error::AppError::ConversionEntityError(e.to_string()))?;
        String::from_utf8(data)
            .map_err(|e| common::error::AppError::ConversionEntityError(e.to_string()))
    }

    async fn health_check(&self) -> AppResult<bool> {
        Ok(self.check_repo.check_database().await?)
    }
}

const MAP_CHUNK: usize = 500;

/// 簡易地図の JSON Lines(gzip でもよい)を 1 行ずつ確かめて返す。空の行は飛ばす
fn summit_map_lines(data: &[u8]) -> impl Iterator<Item = AppResult<SummitMap>> + Send + '_ {
    use std::io::{BufRead, BufReader, Read};
    let reader: Box<dyn Read + Send + '_> = if data.starts_with(&[0x1f, 0x8b]) {
        Box::new(flate2::read::GzDecoder::new(data))
    } else {
        Box::new(data)
    };
    BufReader::new(reader)
        .lines()
        .enumerate()
        .filter(|(_, line)| !matches!(line, Ok(l) if l.trim().is_empty()))
        .map(|(i, line)| {
            let line = line.map_err(|e| {
                AppError::UnprocessableEntity(format!("{} 行目を読めません: {}", i + 1, e))
            })?;
            parse_summit_map_line(&line)
                .map_err(|e| AppError::UnprocessableEntity(format!("{} 行目: {}", i + 1, e)))
        })
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

    fn make_full_summit() -> SotaReference {
        SotaReference {
            summit_code: "JA/TK-001".to_string(),
            association_name: "Japan".to_string(),
            region_name: "Tokyo".to_string(),
            summit_name: "Mt. Takao".to_string(),
            summit_name_j: Some("高尾山".to_string()),
            city: Some("Hachioji".to_string()),
            city_j: Some("八王子市".to_string()),
            alt_m: 599,
            alt_ft: 1965,
            grid_ref1: "PM95".to_string(),
            grid_ref2: "".to_string(),
            longitude: 139.2431,
            latitude: 35.6254,
            maidenhead: "PM95wv".to_string(),
            points: 1,
            bonus_points: 0,
            valid_from: NaiveDate::from_ymd_opt(2010, 1, 1).unwrap(),
            valid_to: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            activation_count: 42,
            activation_date: Some("20/03/2024".to_string()),
            activation_call: Some("JA1ABC".to_string()),
        }
    }

    #[test]
    fn test_sota_export_row_field_mapping() {
        use crate::model::sota::SOTAExportRow;
        let summit = make_full_summit();
        let row = SOTAExportRow::from(summit);

        assert_eq!(row.summit_code, "JA/TK-001");
        assert_eq!(row.association_name, "Japan");
        assert_eq!(row.region_name, "Tokyo");
        assert_eq!(row.summit_name, "Mt. Takao");
        assert_eq!(row.alt_m, 599);
        assert_eq!(row.alt_ft, 1965);
        assert_eq!(row.longitude, 139.2431);
        assert_eq!(row.latitude, 35.6254);
        assert_eq!(row.points, 1);
        assert_eq!(row.bonus_points, 0);
        assert_eq!(row.activation_count, 42);
        assert_eq!(row.activation_date, Some("20/03/2024".to_string()));
        assert_eq!(row.activation_call, Some("JA1ABC".to_string()));
    }

    #[test]
    fn test_sota_export_row_date_format() {
        use crate::model::sota::SOTAExportRow;
        let summit = make_full_summit();
        let row = SOTAExportRow::from(summit);

        // SOTA標準フォーマット: dd/mm/yyyy
        assert_eq!(row.valid_from, "01/01/2010");
        assert_eq!(row.valid_to, "31/12/2099");
    }

    #[test]
    fn test_sota_export_row_optional_fields_none() {
        use crate::model::sota::SOTAExportRow;
        let mut summit = make_full_summit();
        summit.activation_date = None;
        summit.activation_call = None;
        let row = SOTAExportRow::from(summit);

        assert!(row.activation_date.is_none());
        assert!(row.activation_call.is_none());
    }

    #[test]
    fn test_sota_export_row_csv_header() {
        use crate::model::sota::SOTAExportRow;
        let summit = make_full_summit();
        let row = SOTAExportRow::from(summit);

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(&row).unwrap();
        let csv = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
        let first_line = csv.lines().next().unwrap();

        // PascalCase ヘッダーがSOTA標準フォーマットと一致すること
        assert!(first_line.contains("SummitCode"));
        assert!(first_line.contains("AssociationName"));
        assert!(first_line.contains("ValidFrom"));
        assert!(first_line.contains("ValidTo"));
        assert!(first_line.contains("ActivationCount"));
    }

    #[test]
    fn test_sota_export_row_csv_roundtrip() {
        use crate::model::sota::SOTAExportRow;
        use crate::model::sota::SOTASummitCSV;
        let summit = make_full_summit();
        let row = SOTAExportRow::from(summit);

        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.serialize(&row).unwrap();
        let csv = String::from_utf8(wtr.into_inner().unwrap()).unwrap();

        // エクスポートしたCSVをそのままインポートできること
        let mut rdr = csv::Reader::from_reader(csv.as_bytes());
        let records: Vec<SOTASummitCSV> = rdr.deserialize().collect::<Result<_, _>>().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].summit_code, "JA/TK-001");
        assert_eq!(records[0].valid_from, "01/01/2010");
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    const MAPS: &str = concat!(
        r#"{"type":"FeatureCollection","summit":"JA/KN-001","features":[]}"#,
        "\n\n",
        r#"{"type":"FeatureCollection","summit":"JA/KN-002","features":[]}"#,
        "\n"
    );

    #[test]
    fn test_summit_map_lines_plain_and_gzip() {
        for data in [MAPS.as_bytes().to_vec(), gzip(MAPS.as_bytes())] {
            let maps: Vec<_> = summit_map_lines(&data)
                .collect::<AppResult<Vec<_>>>()
                .unwrap();
            let codes: Vec<_> = maps.iter().map(|m| m.summit_code.as_str()).collect();
            assert_eq!(codes, vec!["JA/KN-001", "JA/KN-002"]);
            assert!(maps[0]
                .geojson
                .starts_with(r#"{"type":"FeatureCollection""#));
        }
    }

    #[test]
    fn test_summit_map_lines_reports_bad_line() {
        let data = format!("{}{}\n", MAPS, r#"{"type":"Feature","summit":"JA/KN-003"}"#);
        let err = summit_map_lines(data.as_bytes())
            .collect::<AppResult<Vec<_>>>()
            .unwrap_err();
        assert!(err.to_string().contains("4 行目"), "{}", err);

        let data = r#"{"type":"FeatureCollection","features":[]}"#;
        assert!(summit_map_lines(data.as_bytes()).next().unwrap().is_err());
    }

    #[test]
    fn test_summit_pota_csv() {
        let data = "summit_code,muni_key,muni_name,pota_code,coverage,verified\n\
                    JA/KN-017,JCG#11001B,神奈川県清川村,JP-1289,part,1\n"
            .to_string();
        let rows: Vec<SummitPotaCSV> = csv_reader(data, false, 1).unwrap();
        let p = SummitPota::try_from(rows.into_iter().next().unwrap()).unwrap();
        assert_eq!(p.muni_key, "JCG#11001B");
        assert_eq!(p.coverage, domain::model::sota::ParkCoverage::Part);
        assert!(p.verified);

        let bad = SummitPotaCSV {
            summit_code: "JA/KN-017".to_string(),
            muni_key: "JCG#11001B".to_string(),
            muni_name: "".to_string(),
            pota_code: "JP-1289".to_string(),
            coverage: "none".to_string(),
            verified: 0,
        };
        assert!(SummitPota::try_from(bad).is_err());
    }
}
