use async_trait::async_trait;
use chrono::{Days, NaiveDateTime, Utc};
use shaku::Component;
use sqlx::{query_as, SqliteConnection, SqlitePool};
use std::time::{Duration, Instant};

use common::config::AppConfig;
use common::error::{db_error, row_not_found, tx_error, AppResult};
use domain::model::event::{DeleteLog, DeleteRef, FindRef, FindRefBuilder, PagenatedResult};
use domain::model::id::{LogId, UserId};
use domain::model::pota::{
    ParkCode, PotaActLog, PotaHuntLog, PotaLogHist, PotaLogStat, PotaLogStatEnt, PotaRefLog,
    PotaReference,
};
use domain::model::AwardProgram::POTA;
use domain::repository::pota::PotaRepository;

use super::querybuilder::findref_query_builder;
use crate::database::connect::ConnectionPool;
use crate::database::model::pota::{
    PotaLegcayLogHistRow, PotaLegcayLogRow, PotaLogHistRow, PotaLogRow, PotaRefLogRow,
    PotaReferenceRow,
};

#[derive(Component)]
#[shaku(interface = PotaRepository)]
pub struct PotaRepositoryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

impl PotaRepositoryImpl {
    async fn create(&self, r: PotaReferenceRow, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO pota_references(
                    pota_code,
                    wwff_code,
                    park_name,
                    park_name_j,
                    park_location,
                    park_locid,
                    park_type,
                    park_inactive,
                    park_area,
                    longitude,
                    latitude,
                    maidenhead,
                    "update"
                )
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9,$10, $11, $12, $13)
                ON CONFLICT (pota_code, wwff_code) DO UPDATE
                SET pota_code = EXCLUDED.pota_code,
                    wwff_code = EXCLUDED.wwff_code,
                    park_name = EXCLUDED.park_name,
                    park_name_j = EXCLUDED.park_name_j,
                    park_location = EXCLUDED.park_location,
                    park_locid = EXCLUDED.park_locid,
                    park_type = EXCLUDED.park_type,
                    park_inactive = EXCLUDED.park_inactive,
                    park_area = EXCLUDED.park_area,
                    longitude = EXCLUDED.longitude,
                    latitude = EXCLUDED.latitude,
                    maidenhead = EXCLUDED.maidenhead,
                    "update" = EXCLUDED."update"
            "#,
            r.pota_code,
            r.wwff_code,
            r.park_name,
            r.park_name_j,
            r.park_location,
            r.park_locid,
            r.park_type,
            r.park_inactive,
            r.park_area,
            r.longitude,
            r.latitude,
            r.maidenhead,
            r.update
        )
        .execute(db)
        .await
        .map_err(db_error("insert/update pota_references"))?;
        Ok(())
    }

    async fn update(
        &self,
        id: &str,
        r: PotaReferenceRow,
        db: &mut SqliteConnection,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
                UPDATE pota_references SET
                    pota_code = $2,
                    wwff_code = $3,
                    park_name = $4,
                    park_name_j = $5,
                    park_location = $6,
                    park_locid = $7,
                    park_type = $8,
                    park_inactive = $9,
                    park_area = $10,
                    longitude = $11,
                    latitude = $12,
                    maidenhead = $13,
                    "update" = $14
                WHERE pota_code = $1 OR (pota_code = '' AND wwff_code = $1)
            "#,
            id,
            r.pota_code,
            r.wwff_code,
            r.park_name,
            r.park_name_j,
            r.park_location,
            r.park_locid,
            r.park_type,
            r.park_inactive,
            r.park_area,
            r.longitude,
            r.latitude,
            r.maidenhead,
            r.update
        )
        .execute(db)
        .await
        .map_err(db_error("update pota_references"))?;
        Ok(())
    }

    async fn delete(&self, ref_id: ParkCode, db: &mut SqliteConnection) -> AppResult<()> {
        let ref_id = ref_id.inner_ref();
        sqlx::query!(
            r#"
                DELETE FROM pota_references
                WHERE pota_code = $1 OR (pota_code = '' AND wwff_code = $1)
            "#,
            ref_id,
        )
        .execute(db)
        .await
        .map_err(db_error("delete pota_references"))?;
        Ok(())
    }

    async fn delete_all(&self, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM pota_references
            "#
        )
        .execute(db)
        .await
        .map_err(db_error("delete all pota_references"))?;
        Ok(())
    }

    async fn update_log(&self, r: PotaLogRow, db: &mut SqliteConnection) -> AppResult<()> {
        let log_id = r.log_id.raw();
        sqlx::query!(
            r#"
                INSERT INTO pota_log (log_id, pota_code, first_qso_date, attempts, activations, qsos)
                VALUES($1, $2, $3, $4, $5, $6)
                ON CONFLICT (log_id, pota_code) DO UPDATE
                SET pota_code = EXCLUDED.pota_code,
                    first_qso_date = EXCLUDED.first_qso_date,
                    attempts = EXCLUDED.attempts,
                    activations = EXCLUDED.activations,
                    qsos = EXCLUDED.qsos
            "#,
            log_id,
            r.pota_code,
            r.first_qso_date,
            r.attempts,
            r.activations,
            r.qsos
        )
        .execute(db)
        .await
        .map_err(db_error("insert/update pota_log"))?;
        Ok(())
    }

    async fn select_logid(&self, log_id: LogId) -> AppResult<PotaLogHistRow> {
        sqlx::query_as!(
            PotaLogHistRow,
            r#"
                SELECT user_id as "user_id: UserId", log_id as "log_id: LogId", log_kind, "update" 
                FROM pota_log_user WHERE log_id = $1
            "#,
            log_id
        )
        .fetch_one(self.pool.inner_ref())
        .await
        .map_err(row_not_found("fetch pota_log_user by log_id"))
    }

    async fn update_logid(
        &self,
        entry: PotaLogHistRow,
        db: &mut SqliteConnection,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO pota_log_user (user_id, log_id, log_kind, "update")
                VALUES($1, $2, $3, $4)
                ON CONFLICT (log_id) DO UPDATE
                SET "update" = EXCLUDED."update",
                    log_kind = EXCLUDED.log_kind
            "#,
            entry.user_id,
            entry.log_id,
            entry.log_kind,
            entry.update
        )
        .execute(db)
        .await
        .map_err(db_error("insert/update pota_log_user"))?;
        Ok(())
    }

    async fn delete_log(&self, d: DeleteLog, db: &mut SqliteConnection) -> AppResult<()> {
        if let Some(before) = d.before {
            sqlx::query!(
                r#"
                DELETE FROM pota_log
                WHERE log_id IN (SELECT log_id FROM pota_log_user WHERE "update" < $1);
                DELETE FROM pota_log_user
                WHERE "update" < $2;
            "#,
                before,
                before
            )
            .execute(&mut *db)
            .await
            .map_err(db_error("delete pota_log by date"))?;
            return Ok(());
        }

        if let Some(log_id) = d.log_id {
            let log_id = log_id.raw();
            sqlx::query!(
                r#"
                DELETE FROM pota_log
                WHERE log_id = $1;
                DELETE FROM pota_log_user
                WHERE log_id = $2;
            "#,
                log_id,
                log_id
            )
            .execute(&mut *db)
            .await
            .map_err(db_error("delete pota_log by log_id"))?;
            return Ok(());
        }
        Ok(())
    }

    async fn log_stat(&self) -> AppResult<PotaLogStat> {
        let expire = Utc::now() - self.config.pota_log_expire;

        let r = sqlx::query!(r#"SELECT COUNT(log_id) as count FROM pota_log_user"#)
            .fetch_one(self.pool.inner_ref())
            .await;
        let log_uploaded = r.map_or(0, |v| v.count);

        let r = sqlx::query!(
            r#"SELECT COUNT(log_id) as count FROM pota_log_user WHERE "update" < $1"#,
            expire
        )
        .fetch_one(self.pool.inner_ref())
        .await;
        let log_expired = r.map_or(0, |r| r.count);

        let r = sqlx::query!(r#"SELECT COUNT(log_id) as count FROM pota_log"#)
            .fetch_one(self.pool.inner_ref())
            .await;
        let log_entries = r.map_or(0, |v| v.count);

        // N+1クエリ問題を解決: JOINで一括取得
        let (mut longest_id, mut longest_entry, mut log_error) =
            (Option::<LogId>::None, 0i64, 0i64);

        let r = sqlx::query!(
            r#"SELECT u.log_id as "log_id: LogId", COUNT(l.log_id) as count
               FROM pota_log_user u
               LEFT JOIN pota_log l ON u.log_id = l.log_id
               GROUP BY u.log_id"#
        )
        .fetch_all(self.pool.inner_ref())
        .await;

        if let Ok(logs) = r {
            for row in logs {
                let loglen = row.count;
                if loglen == 0 {
                    log_error += 1;
                } else if loglen > longest_entry {
                    longest_entry = loglen;
                    longest_id = Some(row.log_id)
                }
            }
        }

        let mut query_latency = Duration::from_millis(0);
        let mut log_history = Vec::new();

        if let Some(logid) = longest_id {
            let query = FindRefBuilder::default()
                .pota()
                .log_id(logid)
                .bbox(120.0, 20.0, 150.0, 46.0)
                .build();

            let now = Instant::now();
            let _res = self.find_reference(&query).await;
            query_latency = now.elapsed();
        }

        let end_date = Utc::now().naive_utc();

        // N+1クエリ問題を解決: 日付ごとの統計を一括取得
        let days: Vec<NaiveDateTime> = (0..14)
            .filter_map(|i| end_date.checked_sub_days(Days::new(i)))
            .collect();

        for day in days {
            // JOINで一括取得
            let r = sqlx::query!(
                r#"SELECT COUNT(DISTINCT u.log_id) as users, COUNT(l.log_id) as logs
                   FROM pota_log_user u
                   LEFT JOIN pota_log l ON u.log_id = l.log_id
                   WHERE u."update" <= $1"#,
                day
            )
            .fetch_one(self.pool.inner_ref())
            .await;

            let (users, logs) = r.map_or((0i64, 0i64), |r| (r.users, r.logs));
            let time = day.and_utc().to_rfc3339();
            log_history.push(PotaLogStatEnt { time, users, logs });
        }

        Ok(PotaLogStat {
            log_uploaded,
            log_entries,
            log_expired,
            log_error,
            longest_id: longest_id.unwrap_or_default(),
            longest_entry,
            query_latency,
            log_history,
        })
    }

    async fn migrate_legacy(&self, dbname: &str) -> anyhow::Result<()> {
        let pool = SqlitePool::connect_lazy(dbname)?;

        tracing::info!("Migrate legacy database:{}", dbname);

        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin migrate_legacy pota_log_user"))?;

        let data = query_as!(PotaLegcayLogHistRow, r#"SELECT uuid,time FROM potauser"#)
            .fetch_all(&pool)
            .await?;

        tracing::info!("Found {} user records from legacy DB.", data.len());

        for d in data {
            let row: PotaLogHistRow = match d.try_into() {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("Skipping invalid legacy log hist record: {}", e);
                    continue;
                }
            };
            sqlx::query!(
                r#"INSERT INTO pota_log_user (user_id, log_id, log_kind, "update")
                            VALUES($1, $2, $3, $4)
                            ON CONFLICT (log_id) DO UPDATE
                            SET "update" = EXCLUDED."update",
                            log_kind = EXCLUDED.log_kind"#,
                row.user_id,
                row.log_id,
                row.log_kind,
                row.update
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit()
            .await
            .map_err(tx_error("commit migrate_legacy pota_log_user"))?;

        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin migrate_legacy pota_log"))?;

        let limit = 5000;
        let mut offset = 0;

        loop {
            tracing::info!("reading log offset = {}", offset);

            let data = sqlx::query_as!(
                PotaLegcayLogRow,
                r#"SELECT uuid,ref as "pota_code",type as "log_type",date,qso,attempt,activate
                FROM potalog
                LIMIT $1 OFFSET $2"#,
                limit,
                offset
            )
            .fetch_all(&pool)
            .await?;

            if data.is_empty() {
                break;
            }

            for (idx, d) in data.into_iter().enumerate() {
                let row: PotaLogRow = match d.try_into() {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("Skipping invalid legacy log record at {}: {}", idx, e);
                        continue;
                    }
                };
                sqlx::query!(
                 r#"
                INSERT INTO pota_log (log_id, pota_code, first_qso_date, attempts, activations, qsos)
                VALUES($1, $2, $3, $4, $5, $6)
                ON CONFLICT (log_id, pota_code) DO UPDATE
                SET pota_code = EXCLUDED.pota_code,
                    first_qso_date = EXCLUDED.first_qso_date,
                    attempts = EXCLUDED.attempts,
                    activations = EXCLUDED.activations,
                    qsos = EXCLUDED.qsos
            "#,
            row.log_id, row.pota_code,
            row.first_qso_date,
            row.attempts,
            row.activations,
            row.qsos)
            .execute(&mut *tx).await?;
            }

            offset += limit;
        }

        tx.commit()
            .await
            .map_err(tx_error("commit migrate_legacy pota_log"))?;
        tracing::info!("done");

        Ok(())
    }

    async fn select(&self, query: &FindRef) -> AppResult<PotaReferenceRow> {
        let select = r#"
            SELECT
                pota_code,
                wwff_code,
                park_name,
                park_name_j,
                park_location,
                park_locid,
                park_type,
                park_inactive,
                park_area,
                longitude,
                latitude,
                maidenhead,
                "update"
            FROM pota_references AS p WHERE "#;

        let mut builder = findref_query_builder(POTA, None, select, query);
        let sql_query = builder.build_query_as::<PotaReferenceRow>();

        let row: PotaReferenceRow = sql_query
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch pota_references"))?;

        Ok(row)
    }

    async fn select_pagenated(&self, query: &FindRef) -> AppResult<(i64, Vec<PotaReferenceRow>)> {
        let count_select = r#"SELECT COUNT(*) FROM pota_references AS p WHERE "#;
        let mut count_query = query.clone();
        count_query.limit = None;
        count_query.offset = None;
        let mut count_builder = findref_query_builder(POTA, None, count_select, &count_query);
        let total: i64 = count_builder
            .build_query_scalar::<i64>()
            .fetch_one(self.pool.inner_ref())
            .await
            .unwrap_or(0);

        let select = r#"
            SELECT
                pota_code,
                wwff_code,
                park_name,
                park_name_j,
                park_location,
                park_locid,
                park_type,
                park_inactive,
                park_area,
                longitude,
                latitude,
                maidenhead,
                "update"
            FROM pota_references AS p WHERE "#;

        let mut builder = findref_query_builder(POTA, None, select, query);
        let sql_query = builder.build_query_as::<PotaReferenceRow>();

        let rows: Vec<PotaReferenceRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch pota_references pagenated"))?;

        Ok((total, rows))
    }

    async fn count_by_condition(&self, query: &FindRef) -> AppResult<i64> {
        let select = r#"
            SELECT COUNT(*) FROM pota_references WHERE "#;

        let mut builder = findref_query_builder(POTA, None, select, query);
        let sql_query = builder.build_query_scalar::<i64>();

        let row: Result<i64, _> = sql_query.fetch_one(self.pool.inner_ref()).await;
        Ok(row.unwrap_or(0))
    }

    async fn select_by_condition(
        &self,
        log_id: Option<LogId>,
        query: &FindRef,
    ) -> AppResult<Vec<PotaRefLogRow>> {
        let select = if log_id.is_none() {
            r#"
                SELECT
                    pota_code,
                    wwff_code,
                    park_name,
                    park_name_j,
                    park_location,
                    park_locid,
                    park_type,
                    park_inactive,
                    park_area,
                    longitude,
                    latitude,
                    maidenhead,
                    NULL as attempts,
                    NULL as activations,
                    NULL as first_qso_date,
                    NULL as qsos
                FROM pota_references AS p WHERE "#
        } else {
            r#"
                SELECT
                    p.pota_code AS pota_code,
                    p.wwff_code AS wwff_code,
                    p.park_name AS park_name,
                    p.park_name_j AS park_name_j,
                    p.park_location AS park_location,
                    p.park_locid AS park_locid,
                    p.park_type AS park_type,
                    p.park_inactive AS park_inactive,
                    p.park_area AS park_area,
                    p.longitude AS longitude,
                    p.latitude AS latitude,
                    p.maidenhead AS maidenhead,
                    l.attempts as attempts,
                    l.activations AS activations,
                    l.first_qso_date AS first_qso_date,
                    l.qsos AS qsos
                FROM pota_references AS p 
                LEFT JOIN pota_log AS l ON p.pota_code = l.pota_code AND l.log_id = "#
        };

        let mut builder = findref_query_builder(POTA, log_id, select, query);
        let sql_query = builder.build_query_as::<PotaRefLogRow>();

        let rows: Vec<PotaRefLogRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(row_not_found("fetch pota_references by condition"))?;
        Ok(rows)
    }
}

#[async_trait]
impl PotaRepository for PotaRepositoryImpl {
    async fn count_reference(&self, event: &FindRef) -> AppResult<i64> {
        Ok(self.count_by_condition(event).await?)
    }

    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<PotaRefLog>> {
        let log_id = event.log_id;
        let results = self.select_by_condition(log_id, event).await?;
        let results = results.into_iter().map(PotaRefLog::from).collect();
        Ok(results)
    }

    async fn create_reference(&self, references: Vec<PotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin create_reference pota"))?;

        let len = references.len();
        for r in references.into_iter().enumerate() {
            self.create(PotaReferenceRow::from(r.1), &mut tx).await?;
            if r.0 % 10000 == 0 {
                tracing::info!("upsert pota {}/{}", r.0, len);
            }
        }
        tx.commit()
            .await
            .map_err(tx_error("commit create_reference pota"))?;
        Ok(())
    }

    async fn show_reference(&self, event: &FindRef) -> AppResult<PotaReference> {
        let result = self.select(event).await?;
        Ok(result.into())
    }

    async fn show_all_references(
        &self,
        event: &FindRef,
    ) -> AppResult<PagenatedResult<PotaReference>> {
        let limit = event.limit.unwrap_or(10);
        let offset = event.offset.unwrap_or(0);
        let (total, results) = self.select_pagenated(event).await?;
        Ok(PagenatedResult {
            total,
            limit,
            offset,
            results: results.into_iter().map(PotaReference::from).collect(),
        })
    }

    async fn update_reference(&self, id: &str, references: Vec<PotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin update_reference pota"))?;
        for r in references.into_iter() {
            self.update(id, PotaReferenceRow::from(r), &mut tx).await?;
        }
        tx.commit()
            .await
            .map_err(tx_error("commit update_reference pota"))?;
        Ok(())
    }

    async fn delete_reference(&self, query: DeleteRef<ParkCode>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin delete_reference pota"))?;
        match query {
            DeleteRef::Delete(code) => self.delete(code, &mut tx).await?,
            DeleteRef::DeleteAll => self.delete_all(&mut tx).await?,
        }
        tx.commit()
            .await
            .map_err(tx_error("commit delete_reference pota"))?;
        Ok(())
    }

    async fn upload_activator_log(&self, logs: Vec<PotaActLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin upload_activator_log pota"))?;

        tracing::info!("upload activator log {} rescords", logs.len());

        for r in logs.into_iter() {
            self.update_log(PotaLogRow::from(r), &mut tx).await?;
        }
        tx.commit()
            .await
            .map_err(tx_error("commit upload_activator_log pota"))?;
        Ok(())
    }

    async fn upload_hunter_log(&self, logs: Vec<PotaHuntLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin upload_hunter_log pota"))?;

        tracing::info!("upload hunter log {} rescords", logs.len());

        for r in logs.into_iter() {
            self.update_log(PotaLogRow::from(r), &mut tx).await?;
        }
        tx.commit()
            .await
            .map_err(tx_error("commit upload_hunter_log pota"))?;
        Ok(())
    }

    async fn delete_log(&self, query: DeleteLog) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin delete_log pota"))?;
        self.delete_log(query, &mut tx).await?;
        tx.commit()
            .await
            .map_err(tx_error("commit delete_log pota"))?;
        Ok(())
    }

    async fn log_statistics(&self) -> AppResult<PotaLogStat> {
        self.log_stat().await
    }

    async fn migrate_legacy_log(&self, dbname: String) -> AppResult<()> {
        let res = self.migrate_legacy(&dbname).await;
        if res.is_err() {
            tracing::error!("Legacy DB:{} migration failed.", dbname)
        } else {
            tokio::time::sleep(Duration::from_secs(10)).await;
            tracing::info!("Sending graceful shutdown signal.");
            let _ = self.config.shutdown_tx.send(true);
        }
        Ok(())
    }

    async fn find_logid(&self, query: LogId) -> AppResult<PotaLogHist> {
        let result = self.select_logid(query).await?;
        Ok(result.into())
    }

    async fn update_logid(&self, log: PotaLogHist) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin update_logid pota"))?;
        self.update_logid(PotaLogHistRow::from(log), &mut tx)
            .await?;
        tx.commit()
            .await
            .map_err(tx_error("commit update_logid pota"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::model::event::FindRefBuilder;
    use sqlx::migrate::Migrator;
    use sqlx::sqlite::SqlitePool;
    use std::path::Path;
    use tempfile::tempdir;

    async fn setup_test_db() -> (SqlitePool, tempfile::TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite:{}", db_path.display());
        std::fs::File::create(&db_path).expect("Failed to create db file");
        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("Failed to connect to test db");
        let migration_path = Path::new("migrations/sqlite");
        let migrator = Migrator::new(migration_path)
            .await
            .expect("Failed to load migrations");
        migrator.run(&pool).await.expect("Failed to run migrations");
        (pool, temp_dir)
    }

    fn make_pota_ref(pota_code: &str, wwff_code: &str, park_name: &str) -> PotaReference {
        PotaReference {
            pota_code: pota_code.to_string(),
            wwff_code: wwff_code.to_string(),
            park_name: park_name.to_string(),
            park_name_j: park_name.to_string(),
            park_location: "Japan".to_string(),
            park_locid: "JP-TK".to_string(),
            park_type: "National Park".to_string(),
            park_inactive: false,
            park_area: 100,
            longitude: 139.0,
            latitude: 35.0,
            maidenhead: "PM95wv".to_string(),
            update: Utc::now(),
        }
    }

    fn make_repo(pool: SqlitePool) -> PotaRepositoryImpl {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let d0 = chrono::Duration::zero();
        PotaRepositoryImpl {
            pool: ConnectionPool::new(pool),
            config: AppConfig {
                host: String::new(),
                port: 8080,
                database: String::new(),
                run_migration: false,
                migration_path: String::new(),
                cors_origin: None,
                firebase_api_key: String::new(),
                auth_token_ttl: d0,
                log_level: String::new(),
                sota_alert_endpoint: String::new(),
                sota_spot_endpoint: String::new(),
                pota_alert_endpoint: String::new(),
                pota_spot_endpoint: String::new(),
                sota_summitlist_endpoint: String::new(),
                sota_summitlist_update_schedule: String::new(),
                pota_parklist_endpoint: String::new(),
                pota_parklist_update_schedule: String::new(),
                geomag_endpoint: String::new(),
                geomag_update_schedule: String::new(),
                mapcode_endpoint: String::new(),
                alert_update_interval: 0,
                alert_expire: d0,
                spot_update_interval: 0,
                spot_expire: d0,
                aprs_log_expire: d0,
                pota_log_expire: d0,
                aprs_host: String::new(),
                aprs_user: String::new(),
                aprs_password: String::new(),
                aprs_exclude_user: None,
                aprs_arrival_mesg_regex: None,
                openapi_level: common::config::OpenApiLevel::None,
                award_template_dir: String::new(),
                award_config_path: String::new(),
                shutdown_tx,
                shutdown_rx,
            },
        }
    }

    /// ページネーション時にtotalが正しく返ること
    #[tokio::test]
    async fn test_pagination_total_is_correct_across_pages() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = make_repo(pool);

        let refs: Vec<PotaReference> = (1..=5)
            .map(|i| make_pota_ref(&format!("JP-{:04}", i), "", &format!("Park {}", i)))
            .collect();
        repo.create_reference(refs).await.expect("create");

        // page1: total=5
        let q1 = FindRefBuilder::default().pota().limit(2).offset(0).build();
        let r1 = repo.show_all_references(&q1).await.expect("page1");
        assert_eq!(r1.total, 5, "page1 total should be 5");
        assert_eq!(r1.results.len(), 2);

        // page2 (offset=2): total must still be 5, not 0
        let q2 = FindRefBuilder::default().pota().limit(2).offset(2).build();
        let r2 = repo.show_all_references(&q2).await.expect("page2");
        assert_eq!(
            r2.total, 5,
            "page2 total must remain 5 (pagination bug regression)"
        );
        assert_eq!(r2.results.len(), 2);

        // page3 (offset=4): last page
        let q3 = FindRefBuilder::default().pota().limit(2).offset(4).build();
        let r3 = repo.show_all_references(&q3).await.expect("page3");
        assert_eq!(r3.total, 5, "page3 total must remain 5");
        assert_eq!(r3.results.len(), 1);
    }

    /// wwff_code の前方一致検索が POTA mode で機能すること
    #[tokio::test]
    async fn test_wwff_code_partial_search() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = make_repo(pool);

        let refs = vec![
            make_pota_ref("", "JAFF-0100", "Park A"),
            make_pota_ref("", "JAFF-0101", "Park B"),
            make_pota_ref("JP-0001", "", "JP Park"),
        ];
        repo.create_reference(refs).await.expect("create");

        // 前方一致: "JAFF-010" → JAFF-0100, JAFF-0101 の2件
        let q = FindRefBuilder::default()
            .pota()
            .wwff_code("JAFF-010".to_string())
            .limit(10)
            .offset(0)
            .build();
        let r = repo.show_all_references(&q).await.expect("search");
        assert_eq!(
            r.total, 2,
            "prefix wwff_code 'JAFF-010' should match 2 JAFF entries"
        );

        // 前方一致: "JAFF-" → 2件
        let q_prefix = FindRefBuilder::default()
            .pota()
            .wwff_code("JAFF-".to_string())
            .limit(10)
            .offset(0)
            .build();
        let r_prefix = repo
            .show_all_references(&q_prefix)
            .await
            .expect("search_prefix");
        assert_eq!(
            r_prefix.total, 2,
            "prefix wwff_code 'JAFF-' should match all JAFF entries"
        );

        // 前方一致: "JAFF-0100" → 1件
        let q2 = FindRefBuilder::default()
            .pota()
            .wwff_code("JAFF-0100".to_string())
            .limit(10)
            .offset(0)
            .build();
        let r2 = repo.show_all_references(&q2).await.expect("search2");
        assert_eq!(r2.total, 1);
        assert_eq!(r2.results[0].wwff_code, "JAFF-0100");
    }

    /// pota_code の前方一致検索が機能すること
    #[tokio::test]
    async fn test_pota_code_partial_search() {
        let (pool, _temp_dir) = setup_test_db().await;
        let repo = make_repo(pool);

        let refs = vec![
            make_pota_ref("JP-0011", "", "Park 11"),
            make_pota_ref("JP-0012", "", "Park 12"),
            make_pota_ref("JP-0099", "", "Park 99"),
        ];
        repo.create_reference(refs).await.expect("create");

        // 前方一致: "JP-001" → JP-0011, JP-0012 の2件
        let q = FindRefBuilder::default()
            .pota()
            .pota_code("JP-001".to_string())
            .limit(10)
            .offset(0)
            .build();
        let r = repo.show_all_references(&q).await.expect("search");
        assert_eq!(
            r.total, 2,
            "prefix pota_code 'JP-001' should match JP-0011 and JP-0012"
        );

        // 前方一致: "JP-" → 全3件
        let q2 = FindRefBuilder::default()
            .pota()
            .pota_code("JP-".to_string())
            .limit(10)
            .offset(0)
            .build();
        let r2 = repo.show_all_references(&q2).await.expect("search2");
        assert_eq!(
            r2.total, 3,
            "prefix pota_code 'JP-' should match all 3 parks"
        );
    }
}
