use std::cmp::Ordering;

use async_trait::async_trait;
use common::utils::calculate_distance;
use shaku::Component;
use sqlx::SqliteConnection;

use common::error::{AppError, AppResult};
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
        .map_err(AppError::SpecificOperationError)?;
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
            .map_err(AppError::SpecificOperationError)?;
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

        let row: SotaReferenceRow =
            sql_query
                .fetch_one(self.pool.inner_ref())
                .await
                .map_err(|e| AppError::RowNotFound {
                    source: e,
                    location: format!("{}:{}", file!(), line!()),
                })?;

        Ok(row)
    }

    async fn select_pagenated(&self, event: &FindRef) -> AppResult<(i64, Vec<SotaReferenceRow>)> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM sota_references")
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;
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
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;

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
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;

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
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;
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
            .map_err(AppError::TransactionError)?;

        let len = references.len();
        for r in references.into_iter().enumerate() {
            self.create(SotaReferenceRow::from(r.1), &mut tx).await?;
            if r.0 % 10000 == 0 {
                tracing::info!("insert sota with {}/{} rescords", r.0, len);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
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
            .map_err(AppError::TransactionError)?;

        tracing::info!("update sota with {} rescords", references.len());

        for r in references.into_iter() {
            self.update(SotaReferenceRow::from(r), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn upsert_reference(&self, references: Vec<SotaReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        for r in references.into_iter().enumerate() {
            self.upsert_partial(SotaReferenceRow::from(r.1), &mut tx)
                .await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_reference(&self, query: DeleteRef<SummitCode>) -> AppResult<()> {
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

    async fn count_reference(&self, event: &FindRef) -> AppResult<i64> {
        Ok(self.count_by_condition(event).await?)
    }

    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<SotaReference>> {
        let mut results = self.select_by_condition(event).await?;

        if event.center.is_some() {
            let lat = event.center.as_ref().unwrap().lat;
            let lon = event.center.as_ref().unwrap().lon;

            results.sort_by(|a, b| {
                let dist1 = calculate_distance(lat, lon, a.latitude.unwrap(), a.longitude.unwrap());
                let dist2 = calculate_distance(lat, lon, b.latitude.unwrap(), b.longitude.unwrap());
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
            .map_err(AppError::TransactionError)?;

        for l in logs.into_iter().enumerate() {
            self.create_log(SotaLogRow::from(l.1), &mut tx).await?;
            if l.0 % 500 == 0 {
                tracing::info!("insert sota log {} rescords", l.0);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
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
            .map_err(AppError::TransactionError)?;

        self.delete_log(query, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;

        Ok(())
    }
}
