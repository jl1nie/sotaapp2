use async_trait::async_trait;
use domain::model::id::{LogId, UserId};
use shaku::Component;
use sqlx::SqliteConnection;

use common::error::{AppError, AppResult};

use domain::model::event::{DeleteLog, DeleteRef, FindRef, PagenatedResult};
use domain::model::pota::{
    POTAActivatorLog, POTAHunterLog, POTALogUser, POTAReference, POTAReferenceWithLog, ParkCode,
};
use domain::model::AwardProgram::POTA;

use super::querybuilder::findref_query_builder;
use crate::database::connect::ConnectionPool;
use crate::database::model::pota::{
    POTALogImpl, POTALogUserImpl, POTAReferenceImpl, POTAReferenceWithLogImpl,
};

use domain::repository::pota::POTARepository;

#[derive(Component)]
#[shaku(interface = POTARepository)]
pub struct POTARepositoryImpl {
    pool: ConnectionPool,
}

impl POTARepositoryImpl {
    async fn create(&self, r: POTAReferenceImpl, db: &mut SqliteConnection) -> AppResult<()> {
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
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9,$10, $11,$12,$13)
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

    async fn update(&self, r: POTAReferenceImpl, db: &mut SqliteConnection) -> AppResult<()> {
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

    async fn update_log(&self, r: POTALogImpl, db: &mut SqliteConnection) -> AppResult<()> {
        let log_id = r.log_id.raw();
        sqlx::query!(
            r#"
                INSERT INTO pota_log (log_id, dx_entity, location, hasc, pota_code, park_name, first_qso_date, attempts, activations, qsos)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (log_id, pota_code) DO UPDATE
                SET dx_entity = EXCLUDED.dx_entity,
                    location = EXCLUDED.location,
                    hasc = EXCLUDED.hasc,
                    pota_code = EXCLUDED.pota_code,
                    park_name = EXCLUDED.park_name,
                    first_qso_date = EXCLUDED.first_qso_date,
                    attempts = EXCLUDED.attempts,
                    activations = EXCLUDED.activations,
                    qsos = EXCLUDED.qsos
            "#,
            log_id,
            r.dx_entity,
            r.location,
            r.hasc,
            r.pota_code,
            r.park_name,
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

    async fn select_logid(&self, log_id: LogId) -> AppResult<POTALogUserImpl> {
        sqlx::query_as!(
            POTALogUserImpl,
            r#"
                SELECT user_id as "user_id: UserId", log_id as "log_id: LogId", log_kind, "update" 
                FROM pota_log_user WHERE log_id = $1
            "#,
            log_id
        )
        .fetch_one(self.pool.inner_ref())
        .await
        .map_err(AppError::RowNotFound)
    }

    async fn update_logid(
        &self,
        entry: POTALogUserImpl,
        db: &mut SqliteConnection,
    ) -> AppResult<()> {
        tracing::info!("update logid {:?}", entry);
        sqlx::query!(
            r#"
                INSERT INTO pota_log_user (user_id, log_id, "update")
                VALUES($1, $2, $3)
                ON CONFLICT (log_id) DO UPDATE
                SET "update" = EXCLUDED."update"
            "#,
            entry.user_id,
            entry.log_id,
            entry.update
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        tracing::info!("update logid {:?} done", entry);
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

    async fn select(&self, query: &str) -> AppResult<POTAReferenceImpl> {
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

        let sql_query = sqlx::query_as::<_, POTAReferenceImpl>(&select);
        let row: POTAReferenceImpl = sql_query
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
        Ok(row)
    }

    async fn select_pagenated(&self, query: &str) -> AppResult<(i64, Vec<POTAReferenceImpl>)> {
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

        let sql_query = sqlx::query_as::<_, POTAReferenceImpl>(&select);
        let rows: Vec<POTAReferenceImpl> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
        Ok((total, rows))
    }

    async fn select_by_condition(
        &self,
        log_id: Option<LogId>,
        query: &str,
    ) -> AppResult<Vec<POTAReferenceWithLogImpl>> {
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
        let mut sql_query = sqlx::query_as::<_, POTAReferenceWithLogImpl>(&select);

        if let Some(log_id) = log_id {
            sql_query = sql_query.bind(log_id);
        }

        let rows: Vec<POTAReferenceWithLogImpl> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
        Ok(rows)
    }
}

#[async_trait]
impl POTARepository for POTARepositoryImpl {
    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<POTAReferenceWithLog>> {
        let log_id = event.log_id;
        let query = findref_query_builder(POTA, event);
        let results = self.select_by_condition(log_id, &query).await?;
        let results = results
            .into_iter()
            .map(POTAReferenceWithLog::from)
            .collect();
        Ok(results)
    }

    async fn create_reference(&self, references: Vec<POTAReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        for r in references.into_iter().enumerate() {
            self.create(POTAReferenceImpl::from(r.1), &mut tx).await?;
            if r.0 % 100 == 0 {
                tracing::info!("insert pota {} rescords", r.0);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn show_reference(&self, event: &FindRef) -> AppResult<POTAReference> {
        let query = findref_query_builder(POTA, event);
        let result = self.select(&query).await?;
        Ok(result.into())
    }

    async fn show_all_references(
        &self,
        event: &FindRef,
    ) -> AppResult<PagenatedResult<POTAReference>> {
        let limit = event.limit.unwrap_or(10);
        let offset = event.offset.unwrap_or(0);
        let query = findref_query_builder(POTA, event);
        let (total, results) = self.select_pagenated(&query).await?;
        Ok(PagenatedResult {
            total,
            limit,
            offset,
            results: results.into_iter().map(POTAReference::from).collect(),
        })
    }

    async fn update_reference(&self, references: Vec<POTAReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        for r in references.into_iter() {
            self.update(POTAReferenceImpl::from(r), &mut tx).await?;
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

    async fn upload_activator_log(&self, logs: Vec<POTAActivatorLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        tracing::info!("upload activator log {} rescords", logs.len());

        for r in logs.into_iter() {
            self.update_log(POTALogImpl::from(r), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn upload_hunter_log(&self, logs: Vec<POTAHunterLog>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        tracing::info!("upload hunter log {} rescords", logs.len());

        for r in logs.into_iter() {
            self.update_log(POTALogImpl::from(r), &mut tx).await?;
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

    async fn find_logid(&self, query: LogId) -> AppResult<POTALogUser> {
        let result = self.select_logid(query).await?;
        Ok(result.into())
    }

    async fn update_logid(&self, log: POTALogUser) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        self.update_logid(POTALogUserImpl::from(log), &mut tx)
            .await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }
}
