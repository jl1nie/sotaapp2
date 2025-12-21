use std::cmp::Ordering;

use async_trait::async_trait;
use common::utils::calculate_distance;
use shaku::Component;
use sqlx::SqliteConnection;

use common::error::{db_error, row_not_found, tx_error, AppResult};
use domain::model::event::{DeleteLog, DeleteRef, FindLog, FindRef, PagenatedResult};
use domain::model::sota::{SotaLog, SotaReference, SummitCode};
use domain::model::AwardProgram::SOTA;

use super::querybuilder::{findlog_query_builder, findref_query_builder};
use crate::database::connect::ConnectionPool;
use crate::database::model::sota::{SotaLogRow, SotaReferenceRow};

use domain::repository::sota::SotaRepository;

#[derive(Component)]
#[shaku(interface = SotaRepository)]
pub struct SotaRepositoryImpl {
    pool: ConnectionPool,
}

impl SotaRepositoryImpl {
    async fn create(&self, r: SotaReferenceRow, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO sota_references(
                    summit_code,
                    association_name,
                    region_name,
                    summit_name,
                    summit_name_j,
                    city,
                    city_j,
                    alt_m,
                    alt_ft,
                    grid_ref1,
                    grid_ref2,
                    longitude,
                    latitude,
                    maidenhead,
                    points,
                    bonus_points,
                    valid_from,
                    valid_to,
                    activation_count,
                    activation_date,
                    activation_call
                )
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            "#,
            r.summit_code,
            r.association_name,
            r.region_name,
            r.summit_name,
            r.summit_name_j,
            r.city,
            r.city_j,
            r.alt_m,
            r.alt_ft,
            r.grid_ref1,
            r.grid_ref2,
            r.longitude,
            r.latitude,
            r.maidenhead,
            r.points,
            r.bonus_points,
            r.valid_from,
            r.valid_to,
            r.activation_count,
            r.activation_date,
            r.activation_call)
        .execute(db)
        .await
        .map_err(db_error("insert sota_references"))?;
        Ok(())
    }

    async fn create_log(&self, l: SotaLogRow, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO sota_log(
                      user_id,
                      my_callsign,
                      operator,
                      my_summit_code,
                      time,
                      frequency,
                      mode,
                      his_callsign,
                      his_summit_code,
                      comment,
                      "update"
                )
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            l.user_id,
            l.my_callsign,
            l.operator,
            l.my_summit_code,
            l.time,
            l.frequency,
            l.mode,
            l.his_callsign,
            l.his_summit_code,
            l.comment,
            l.update
        )
        .execute(db)
        .await
        .map_err(db_error("insert sota_log"))?;
        Ok(())
    }

    async fn update(&self, r: SotaReferenceRow, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                UPDATE sota_references SET
                    association_name = $2,
                    region_name = $3,
                    summit_name = $4,
                    summit_name_j = $5,
                    city = $6,
                    city_j = $7,
                    alt_m = $8,
                    alt_ft = $9,
                    grid_ref1 = $10,
                    grid_ref2 = $11,
                    longitude = $12, 
                    latitude = $13,
                    maidenhead = $14,
                    points = $15,
                    bonus_points = $16,
                    valid_from = $17,
                    valid_to = $18,
                    activation_count = $19,
                    activation_date = $20,
                    activation_call = $21
                WHERE summit_code = $1
            "#,
            r.summit_code,
            r.association_name,
            r.region_name,
            r.summit_name,
            r.summit_name_j,
            r.city,
            r.city_j,
            r.alt_m,
            r.alt_ft,
            r.grid_ref1,
            r.grid_ref2,
            r.longitude,
            r.latitude,
            r.maidenhead,
            r.points,
            r.bonus_points,
            r.valid_from,
            r.valid_to,
            r.activation_count,
            r.activation_date,
            r.activation_call
        )
        .execute(db)
        .await
        .map_err(db_error("update sota_references"))?;
        Ok(())
    }

    async fn upsert_partial(
        &self,
        r: SotaReferenceRow,
        db: &mut SqliteConnection,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO sota_references (
                    summit_code,
                    association_name,
                    region_name,
                    summit_name,
                    summit_name_j,
                    city,
                    city_j,
                    alt_m,
                    alt_ft,
                    grid_ref1,
                    grid_ref2,
                    longitude,
                    latitude,
                    maidenhead,
                    points,
                    bonus_points,
                    valid_from,
                    valid_to,
                    activation_count,
                    activation_date,
                    activation_call
                ) VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
                ON CONFLICT (summit_code) DO UPDATE SET
                    association_name = EXCLUDED.association_name,
                    region_name = EXCLUDED.region_name,
                    summit_name = sota_references.summit_name,
                    summit_name_j = sota_references.summit_name_j,
                    city = sota_references.city,
                    city_j = sota_references.city_j,
                    alt_m = sota_references.alt_m,
                    alt_ft = EXCLUDED.alt_ft,
                    grid_ref1 = EXCLUDED.grid_ref1,
                    grid_ref2 = EXCLUDED.grid_ref2,
                    longitude = sota_references.longitude,
                    latitude = sota_references.latitude,
                    points = sota_references.points,
                    bonus_points = EXCLUDED.bonus_points,
                    valid_from = EXCLUDED.valid_from,
                    valid_to = EXCLUDED.valid_to,
                    activation_count = EXCLUDED.activation_count,
                    activation_date = EXCLUDED.activation_date,
                    activation_call = EXCLUDED.activation_call
               "#,
            r.summit_code,
            r.association_name,
            r.region_name,
            r.summit_name,
            r.summit_name_j,
            r.city,
            r.city_j,
            r.alt_m,
            r.alt_ft,
            r.grid_ref1,
            r.grid_ref2,
            r.longitude,
            r.latitude,
            r.maidenhead,
            r.points,
            r.bonus_points,
            r.valid_from,
            r.valid_to,
            r.activation_count,
            r.activation_date,
            r.activation_call
        )
        .execute(db)
        .await
        .map_err(db_error("upsert sota_references"))?;
        Ok(())
    }

    async fn delete(&self, ref_id: SummitCode, db: &mut SqliteConnection) -> AppResult<()> {
        let ref_id = ref_id.inner_ref();
        sqlx::query!(
            r#"
                DELETE FROM sota_references
               WHERE summit_code = $1
            "#,
            ref_id,
        )
        .execute(db)
        .await
        .map_err(db_error("delete sota_references"))?;
        Ok(())
    }

    async fn delete_all(&self, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM sota_references
            "#
        )
        .execute(db)
        .await
        .map_err(db_error("delete all sota_references"))?;
        Ok(())
    }

    async fn delete_log(&self, d: DeleteLog, db: &mut SqliteConnection) -> AppResult<()> {
        if let Some(before) = d.before {
            sqlx::query!(
                r#"
                DELETE FROM sota_log
                WHERE time < $1
            "#,
                before,
            )
            .execute(&mut *db)
            .await
            .map_err(db_error("delete sota_log"))?;
        }
        Ok(())
    }

    async fn select(&self, query: &FindRef) -> AppResult<SotaReferenceRow> {
        let select = r#"
            SELECT
                summit_code,
                association_name,
                region_name,
                summit_name,
                summit_name_j,
                city,
                city_j,
                alt_m,
                alt_ft,
                grid_ref1,
                grid_ref2,
                longitude,
                latitude,
                maidenhead,
                points,
                bonus_points,
                valid_from,
                valid_to,
                activation_count,
                activation_date,
                activation_call
            FROM sota_references WHERE "#;

        let mut builder = findref_query_builder(SOTA, None, select, query);
        let sql_query = builder.build_query_as::<SotaReferenceRow>();

        let row: SotaReferenceRow = sql_query
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch sota_references"))?;

        Ok(row)
    }

    async fn select_pagenated(&self, event: &FindRef) -> AppResult<(i64, Vec<SotaReferenceRow>)> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM sota_references")
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(db_error("count sota_references"))?;
        let total: i64 = row.count;

        let select = r#"
            SELECT
                summit_code,
                association_name,
                region_name,
                summit_name,
                summit_name_j,
                city,
                city_j,
                alt_m,
                alt_ft,
                grid_ref1,
                grid_ref2,
                longitude,
                latitude,
                maidenhead,
                points,
                bonus_points,
                valid_from,
                valid_to,
                activation_count,
                activation_date,
                activation_call
            FROM sota_references WHERE "#;

        let mut builder = findref_query_builder(SOTA, None, select, event);
        let sql_query = builder.build_query_as::<SotaReferenceRow>();

        let rows: Vec<SotaReferenceRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch sota_references pagenated"))?;

        Ok((total, rows))
    }

    async fn select_by_condition(&self, query: &FindRef) -> AppResult<Vec<SotaReferenceRow>> {
        let select = r#"
            SELECT
                summit_code,
                association_name,
                region_name,
                summit_name,
                summit_name_j,
                city,
                city_j,
                alt_m,
                alt_ft,
                grid_ref1,
                grid_ref2,
                longitude,
                latitude,
                maidenhead,
                points,
                bonus_points,
                valid_from,
                valid_to,
                activation_count,
                activation_date,
                activation_call
            FROM sota_references WHERE "#;

        let mut builder = findref_query_builder(SOTA, None, select, query);
        let sql_query = builder.build_query_as::<SotaReferenceRow>();
        let rows: Vec<SotaReferenceRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch sota_references by condition"))?;

        Ok(rows)
    }

    async fn count_by_condition(&self, event: &FindRef) -> AppResult<i64> {
        let select = r#"
            SELECT COUNT(*) FROM sota_references WHERE "#;

        let mut builder = findref_query_builder(SOTA, None, select, event);
        let sql_query = builder.build_query_scalar::<i64>();

        let row = sql_query.fetch_one(self.pool.inner_ref()).await;

        Ok(row.unwrap_or(0))
    }

    async fn select_log_by_condition(&self, query: &FindLog) -> AppResult<Vec<SotaLogRow>> {
        let select = r#"
            SELECT
                user_id,
                my_callsign,
                operator,
                my_summit_code,
                time,
                frequency,
                mode,
                his_callsign,
                his_summit_code,
                comment,
                "update"
            FROM sota_log WHERE "#;

        let mut builder = findlog_query_builder(select, query);
        let sql_query = builder.build_query_as::<SotaLogRow>();

        let rows: Vec<_> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch sota_log by condition"))?;
        Ok(rows)
    }
}

#[async_trait]
impl SotaRepository for SotaRepositoryImpl {
    async fn create_reference(&self, references: Vec<SotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin create_reference sota"))?;

        let len = references.len();
        for r in references.into_iter().enumerate() {
            self.create(SotaReferenceRow::from(r.1), &mut tx).await?;
            if r.0 % 10000 == 0 {
                tracing::info!("insert sota with {}/{} rescords", r.0, len);
            }
        }
        tx.commit()
            .await
            .map_err(tx_error("commit create_reference sota"))?;
        Ok(())
    }

    async fn show_reference(&self, event: &FindRef) -> AppResult<SotaReference> {
        let result = self.select(event).await?;
        Ok(result.into())
    }

    async fn show_all_references(
        &self,
        event: &FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>> {
        let limit = event.limit.unwrap_or(10);
        let offset = event.offset.unwrap_or(0);
        let (total, results) = self.select_pagenated(event).await?;
        Ok(PagenatedResult {
            total,
            limit,
            offset,
            results: results.into_iter().map(SotaReference::from).collect(),
        })
    }

    async fn update_reference(&self, references: Vec<SotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin update_reference sota"))?;

        tracing::info!("update sota with {} rescords", references.len());

        for r in references.into_iter() {
            self.update(SotaReferenceRow::from(r), &mut tx).await?;
        }
        tx.commit()
            .await
            .map_err(tx_error("commit update_reference sota"))?;
        Ok(())
    }

    async fn upsert_reference(&self, references: Vec<SotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin upsert_reference sota"))?;

        for r in references.into_iter().enumerate() {
            self.upsert_partial(SotaReferenceRow::from(r.1), &mut tx)
                .await?;
        }
        tx.commit()
            .await
            .map_err(tx_error("commit upsert_reference sota"))?;
        Ok(())
    }

    async fn delete_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin delete_reference sota"))?;
        match query {
            DeleteRef::Delete(code) => self.delete(code, &mut tx).await?,
            DeleteRef::DeleteAll => self.delete_all(&mut tx).await?,
        }
        tx.commit()
            .await
            .map_err(tx_error("commit delete_reference sota"))?;
        Ok(())
    }

    async fn count_reference(&self, event: &FindRef) -> AppResult<i64> {
        Ok(self.count_by_condition(event).await?)
    }

    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<SotaReference>> {
        let mut results = self.select_by_condition(event).await?;

        if let Some(center) = &event.center {
            let lat = center.lat;
            let lon = center.lon;

            results.sort_by(|a, b| {
                let dist1 = calculate_distance(
                    lat,
                    lon,
                    a.latitude.unwrap_or_default(),
                    a.longitude.unwrap_or_default(),
                );
                let dist2 = calculate_distance(
                    lat,
                    lon,
                    b.latitude.unwrap_or_default(),
                    b.longitude.unwrap_or_default(),
                );
                dist1.partial_cmp(&dist2).unwrap_or(Ordering::Equal)
            });
        }

        Ok(results.into_iter().map(SotaReference::from).collect())
    }

    async fn upload_log(&self, logs: Vec<SotaLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin upload_log sota"))?;

        for l in logs.into_iter().enumerate() {
            self.create_log(SotaLogRow::from(l.1), &mut tx).await?;
            if l.0 % 500 == 0 {
                tracing::info!("insert sota log {} rescords", l.0);
            }
        }
        tx.commit()
            .await
            .map_err(tx_error("commit upload_log sota"))?;
        Ok(())
    }

    async fn find_log(&self, query: &FindLog) -> AppResult<Vec<SotaLog>> {
        let results = self.select_log_by_condition(query).await?;
        Ok(results.into_iter().map(SotaLog::from).collect())
    }

    async fn delete_log(&self, query: DeleteLog) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin delete_log sota"))?;

        self.delete_log(query, &mut tx).await?;
        tx.commit()
            .await
            .map_err(tx_error("commit delete_log sota"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::model::event::FindRefBuilder;
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::SqlitePool;
    use std::path::Path;
    use tempfile::tempdir;

    /// テスト用の一時データベースを作成
    async fn setup_test_db() -> (SqlitePool, tempfile::TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());

        // データベースファイル作成
        std::fs::File::create(&db_path).expect("Failed to create db file");

        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test db");

        // マイグレーション実行
        let migration_path = Path::new("migrations/sqlite");
        let migrator = Migrator::new(migration_path)
            .await
            .expect("Failed to load migrations");
        migrator.run(&pool).await.expect("Failed to run migrations");

        (pool, temp_dir)
    }

    /// テスト用SotaReferenceを作成
    fn make_test_reference(code: &str, name: &str) -> SotaReference {
        SotaReference {
            summit_code: code.to_string(),
            association_name: "Japan".to_string(),
            region_name: "Tokyo".to_string(),
            summit_name: name.to_string(),
            summit_name_j: Some("テスト山".to_string()),
            city: Some("Tokyo".to_string()),
            city_j: Some("東京都".to_string()),
            alt_m: 1000,
            alt_ft: 3280,
            grid_ref1: "PM95".to_string(),
            grid_ref2: "".to_string(),
            longitude: 139.0,
            latitude: 35.0,
            maidenhead: "PM95wv".to_string(),
            points: 10,
            bonus_points: 3,
            valid_from: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            valid_to: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            activation_count: 0,
            activation_date: None,
            activation_call: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_find_reference() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // テストデータ作成
        let reference = make_test_reference("JA/TK-001", "Mt. Test");
        repo.create_reference(vec![reference.clone()])
            .await
            .expect("Failed to create reference");

        // 検索
        let query = FindRefBuilder::default()
            .sota()
            .name("JA/TK-001".to_string())
            .build();
        let result = repo.find_reference(&query).await.expect("Failed to find");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].summit_code, "JA/TK-001");
        assert_eq!(result[0].summit_name, "Mt. Test");
    }

    #[tokio::test]
    async fn test_count_reference() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // 複数のテストデータを作成
        let refs = vec![
            make_test_reference("JA/TK-001", "Mt. A"),
            make_test_reference("JA/TK-002", "Mt. B"),
            make_test_reference("JA/TK-003", "Mt. C"),
        ];
        repo.create_reference(refs)
            .await
            .expect("Failed to create references");

        // カウント
        let query = FindRefBuilder::default().sota().build();
        let count = repo.count_reference(&query).await.expect("Failed to count");

        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_update_reference() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // 作成
        let reference = make_test_reference("JA/TK-001", "Mt. Test");
        repo.create_reference(vec![reference])
            .await
            .expect("Failed to create");

        // 更新
        let mut updated = make_test_reference("JA/TK-001", "Mt. Updated");
        updated.points = 20;
        repo.update_reference(vec![updated])
            .await
            .expect("Failed to update");

        // 確認
        let query = FindRefBuilder::default()
            .sota()
            .name("JA/TK-001".to_string())
            .build();
        let result = repo.find_reference(&query).await.expect("Failed to find");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].summit_name, "Mt. Updated");
        assert_eq!(result[0].points, 20);
    }

    #[tokio::test]
    async fn test_delete_reference() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // 作成
        let refs = vec![
            make_test_reference("JA/TK-001", "Mt. A"),
            make_test_reference("JA/TK-002", "Mt. B"),
        ];
        repo.create_reference(refs).await.expect("Failed to create");

        // 1つ削除
        repo.delete_reference(DeleteRef::Delete(SummitCode::new("JA/TK-001".to_string())))
            .await
            .expect("Failed to delete");

        // 確認
        let query = FindRefBuilder::default().sota().build();
        let count = repo.count_reference(&query).await.expect("Failed to count");
        assert_eq!(count, 1);

        let result = repo.find_reference(&query).await.expect("Failed to find");
        assert_eq!(result[0].summit_code, "JA/TK-002");
    }

    #[tokio::test]
    async fn test_delete_all_references() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // 作成
        let refs = vec![
            make_test_reference("JA/TK-001", "Mt. A"),
            make_test_reference("JA/TK-002", "Mt. B"),
            make_test_reference("JA/TK-003", "Mt. C"),
        ];
        repo.create_reference(refs).await.expect("Failed to create");

        // 全削除
        repo.delete_reference(DeleteRef::DeleteAll)
            .await
            .expect("Failed to delete all");

        // 確認
        let query = FindRefBuilder::default().sota().build();
        let count = repo.count_reference(&query).await.expect("Failed to count");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_upsert_reference() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // 新規作成
        let reference = make_test_reference("JA/TK-001", "Mt. Test");
        repo.upsert_reference(vec![reference])
            .await
            .expect("Failed to upsert");

        // 存在確認
        let query = FindRefBuilder::default().sota().build();
        let count = repo.count_reference(&query).await.expect("Failed to count");
        assert_eq!(count, 1);

        // 再度upsert（更新）
        let mut updated = make_test_reference("JA/TK-001", "Mt. Test");
        updated.activation_count = 5;
        repo.upsert_reference(vec![updated])
            .await
            .expect("Failed to upsert again");

        // カウント変わらず
        let count = repo.count_reference(&query).await.expect("Failed to count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_show_all_references_pagenated() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = SotaRepositoryImpl {
            pool: crate::database::connect::ConnectionPool::new(pool),
        };

        // 10件作成
        let refs: Vec<_> = (1..=10)
            .map(|i| make_test_reference(&format!("JA/TK-{:03}", i), &format!("Mt. {}", i)))
            .collect();
        repo.create_reference(refs).await.expect("Failed to create");

        // ページネーション確認
        let query = FindRefBuilder::default().sota().limit(3).offset(0).build();
        let result = repo
            .show_all_references(&query)
            .await
            .expect("Failed to show all");

        assert_eq!(result.total, 10);
        assert_eq!(result.limit, 3);
        assert_eq!(result.offset, 0);
        assert_eq!(result.results.len(), 3);
    }
}
