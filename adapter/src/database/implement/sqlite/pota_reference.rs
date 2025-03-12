use async_trait::async_trait;
use chrono::{Days, NaiveDateTime, Utc};
use shaku::Component;
use sqlx::{query_as, SqliteConnection, SqlitePool};
use std::time::{Duration, Instant};

use common::config::AppConfig;
use common::error::{AppError, AppResult};
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
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn update(&self, r: PotaReferenceRow, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                UPDATE pota_references SET
                    wwff_code = $2,
                    park_name = $3,
                    park_name_j = $4,
                    park_location = $5,
                    park_locid = $6,
                    park_type = $7,
                    park_inactive = $8,
                    park_area = $9,
                    longitude = $10, 
                    latitude = $11,
                    maidenhead = $12,
                    "update" = $13
                WHERE pota_code = $1
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
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete(&self, ref_id: ParkCode, db: &mut SqliteConnection) -> AppResult<()> {
        let ref_id = ref_id.inner_ref();
        sqlx::query!(
            r#"
                DELETE FROM pota_references
               WHERE pota_code = $1
            "#,
            ref_id,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })
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
        .map_err(AppError::SpecificOperationError)?;
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
            .map_err(AppError::SpecificOperationError)?;
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
            .map_err(AppError::SpecificOperationError)?;
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

        let (mut longest_id, mut longest_entry, mut log_error) =
            (Option::<LogId>::None, 0i64, 0i64);

        let r = sqlx::query_as!(
            PotaLogHistRow,
            r#"SELECT user_id as "user_id: UserId", log_id as "log_id: LogId", log_kind, "update" 
            FROM pota_log_user"#
        )
        .fetch_all(self.pool.inner_ref())
        .await;

        if let Ok(logs) = r {
            for l in logs {
                let r = sqlx::query!(
                    r#"SELECT COUNT(log_id) as count FROM pota_log WHERE log_id = $1"#,
                    l.log_id
                )
                .fetch_one(self.pool.inner_ref())
                .await;
                if r.is_err() {
                    log_error += 1;
                } else {
                    let loglen = r.unwrap().count;
                    if loglen == 0 {
                        log_error += 1;
                    } else if loglen > longest_entry {
                        longest_entry = loglen;
                        longest_id = Some(l.log_id)
                    }
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

        let days: Vec<NaiveDateTime> = (0..14)
            .map(|i| end_date.checked_sub_days(Days::new(i)).unwrap())
            .collect();

        for day in days {
            let loglist = sqlx::query!(
                r#"SELECT log_id as "log_id: LogId" FROM pota_log_user WHERE "update" <= $1"#,
                day
            )
            .fetch_all(self.pool.inner_ref())
            .await
            .map_or(Vec::new(), |r| r.into_iter().map(|r| r.log_id).collect());

            let (mut logs, mut users) = (0i64, 0i64);
            for id in loglist {
                let r = sqlx::query!(
                    r#"SELECT COUNT(*) as count FROM pota_log WHERE log_id = $1"#,
                    id
                )
                .fetch_one(self.pool.inner_ref())
                .await;
                if let Ok(r) = r {
                    logs += r.count;
                    users += 1;
                }
            }
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
            .map_err(AppError::TransactionError)?;

        let data = query_as!(PotaLegcayLogHistRow, r#"SELECT uuid,time FROM potauser"#)
            .fetch_all(&pool)
            .await?;

        tracing::info!("Found {} user records from legacy DB.", data.len());

        for d in data {
            let row: PotaLogHistRow = d.into();
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
        tx.commit().await.map_err(AppError::TransactionError)?;

        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

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

            for d in data.into_iter().enumerate() {
                let row: PotaLogRow = d.1.into();
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
        
        tx.commit().await.map_err(AppError::TransactionError)?;
        tracing::info!("done");

        Ok(())
    }

    async fn select(&self, query: &str) -> AppResult<PotaReferenceRow> {
        let mut select = r#"
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
                update
            FROM pota_references AS p WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, PotaReferenceRow>(&select);
        let row: PotaReferenceRow = sql_query
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;
        Ok(row)
    }

    async fn select_pagenated(&self, query: &str) -> AppResult<(i64, Vec<PotaReferenceRow>)> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM pota_references")
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;
        let total: i64 = row.count;

        let mut select = r#"
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
                update
            FROM pota_references AS p WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, PotaReferenceRow>(&select);
        let rows: Vec<PotaReferenceRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;
        Ok((total, rows))
    }

     async fn count_by_condition(&self, query: &str) -> AppResult<i64> {
        let mut select = r#"
            SELECT COUNT(*) FROM pota_references WHERE "#
            .to_string();
        select.push_str(query);

        let row: Result<i64, _> = sqlx::query_scalar(&select)
            .fetch_one(self.pool.inner_ref())
            .await;
        Ok(row.unwrap_or(0))
    }

    async fn select_by_condition(
        &self,
        log_id: Option<LogId>,
        query: &str,
    ) -> AppResult<Vec<PotaRefLogRow>> {
        let mut select = String::new();
        if log_id.is_none() {
            select.push_str(
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
                FROM pota_references AS p WHERE "#,
            );
        } else {
            select.push_str(
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
                LEFT JOIN pota_log AS l ON p.pota_code = l.pota_code AND l.log_id = ?
                WHERE "#,
            );
        }
        select.push_str(query);
        let mut sql_query = sqlx::query_as::<_, PotaRefLogRow>(&select);

        if let Some(log_id) = log_id {
            sql_query = sql_query.bind(log_id);
        }

        let rows: Vec<PotaRefLogRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;
        Ok(rows)
    }
}

#[async_trait]
impl PotaRepository for PotaRepositoryImpl {

    async fn count_reference(&self, event: &FindRef) -> AppResult<i64> {
        let query = findref_query_builder(POTA, event);
        Ok(self.count_by_condition(&query).await?)
    }

    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<PotaRefLog>> {
        let log_id = event.log_id;
        let query = findref_query_builder(POTA, event);
        let results = self.select_by_condition(log_id, &query).await?;
        let results = results.into_iter().map(PotaRefLog::from).collect();
        Ok(results)
    }

    async fn create_reference(&self, references: Vec<PotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        let len = references.len();
        for r in references.into_iter().enumerate() {
            self.create(PotaReferenceRow::from(r.1), &mut tx).await?;
            if r.0 % 5000 == 0 {
                tracing::info!("upsert pota {}/{}", r.0, len);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn show_reference(&self, event: &FindRef) -> AppResult<PotaReference> {
        let query = findref_query_builder(POTA, event);
        let result = self.select(&query).await?;
        Ok(result.into())
    }

    async fn show_all_references(
        &self,
        event: &FindRef,
    ) -> AppResult<PagenatedResult<PotaReference>> {
        let limit = event.limit.unwrap_or(10);
        let offset = event.offset.unwrap_or(0);
        let query = findref_query_builder(POTA, event);
        let (total, results) = self.select_pagenated(&query).await?;
        Ok(PagenatedResult {
            total,
            limit,
            offset,
            results: results.into_iter().map(PotaReference::from).collect(),
        })
    }

    async fn update_reference(&self, references: Vec<PotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        for r in references.into_iter() {
            self.update(PotaReferenceRow::from(r), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_reference(&self, query: DeleteRef<ParkCode>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        match query {
            DeleteRef::Delete(code) => self.delete(code, &mut tx).await?,
            DeleteRef::DeleteAll => self.delete_all(&mut tx).await?,
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn upload_activator_log(&self, logs: Vec<PotaActLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        tracing::info!("upload activator log {} rescords", logs.len());

        for r in logs.into_iter() {
            self.update_log(PotaLogRow::from(r), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn upload_hunter_log(&self, logs: Vec<PotaHuntLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        tracing::info!("upload hunter log {} rescords", logs.len());

        for r in logs.into_iter() {
            self.update_log(PotaLogRow::from(r), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_log(&self, query: DeleteLog) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        self.delete_log(query, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
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
            .map_err(AppError::TransactionError)?;
        self.update_logid(PotaLogHistRow::from(log), &mut tx)
            .await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }
}
