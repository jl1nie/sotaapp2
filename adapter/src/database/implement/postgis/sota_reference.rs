use async_trait::async_trait;
use shaku::Component;
use sqlx::PgConnection;

use common::error::{AppError, AppResult};
use domain::model::event::{DeleteRef, FindRef, PagenatedResult};
use domain::model::sota::{SotaReference, SummitCode};
use domain::model::AwardProgram::SOTA;

use super::querybuilder::findref_query_builder;
use crate::database::connect::ConnectionPool;
use crate::database::model::sota::SotaReferenceRow;
use domain::repository::sota::SotaRepository;

#[derive(Component)]
#[shaku(interface = SotaRepository)]
pub struct SotaRepositoryImpl {
    pool: ConnectionPool,
}

impl SotaRepositoryImpl {
    async fn create(&self, r: SotaReferenceRow, db: &mut PgConnection) -> AppResult<()> {
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
                    coordinates,
                    maidenhead,
                    points,
                    bonus_points,
                    valid_from,
                    valid_to,
                    activation_count,
                    activation_date,
                    activation_call
                )
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, ST_SetSRID(ST_MakePoint($12, $13), 4326), 
                $14, $15, $16, $17, $18, $19, $20, $21)
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

    async fn update(&self, r: SotaReferenceRow, db: &mut PgConnection) -> AppResult<()> {
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
                    coordinates = ST_SetSRID(ST_MakePoint($12, $13), 4326),
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

    async fn upsert_partial(&self, r: SotaReferenceRow, db: &mut PgConnection) -> AppResult<()> {
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
                    coordinates,
                    maidenhead,
                    points,
                    bonus_points,
                    valid_from,
                    valid_to,
                    activation_count,
                    activation_date,
                    activation_call
                ) VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, ST_SetSRID(ST_MakePoint($12, $13), 4326), 
                $14, $15, $16, $17, $18, $19, $20, $21)
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
                    coordinates = sota_references.coordinates,
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

    async fn delete(&self, ref_id: SummitCode, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM sota_references
               WHERE summit_code = $1
            "#,
            ref_id.inner_ref(),
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete_all(&self, db: &mut PgConnection) -> AppResult<()> {
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

    async fn select(&self, query: &str) -> AppResult<SotaReferenceRow> {
        let mut select = r#"
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
                ST_X(coordinates) AS longitude,
                ST_Y(coordinates) AS latitude,
                maidenhead,
                points,
                bonus_points,
                valid_from,
                valid_to,
                activation_count,
                activation_date,
                activation_call
            FROM sota_references WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, SotaReferenceRow>(&select);
        let row: SotaReferenceRow = sql_query
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
        Ok(row)
    }

    async fn select_pagenated(&self, query: &str) -> AppResult<(i64, Vec<SotaReferenceRow>)> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM sota_references")
            .fetch_one(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
        let total: i64 = row.count.unwrap_or(0);

        let mut select = r#"
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
                ST_X(coordinates) AS longitude,
                ST_Y(coordinates) AS latitude,
                maidenhead,
                points,
                bonus_points,
                valid_from,
                valid_to,
                activation_count,
                activation_date,
                activation_call
            FROM sota_references WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, SotaReferenceRow>(&select);
        let rows: Vec<SotaReferenceRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
        Ok((total, rows))
    }

    async fn select_by_condition(&self, query: &str) -> AppResult<Vec<SotaReferenceRow>> {
        let mut select = r#"
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
                ST_X(coordinates) AS longitude,
                ST_Y(coordinates) AS latitude,
                maidenhead,
                points,
                bonus_points,
                valid_from,
                valid_to,
                activation_count,
                activation_date,
                activation_call
            FROM sota_references WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, SotaReferenceRow>(&select);
        let rows: Vec<SotaReferenceRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::RowNotFound)?;
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

        for r in references.into_iter().enumerate() {
            self.create(SotaReferenceRow::from(r.1), &mut tx).await?;
            if r.0 % 500 == 0 {
                tracing::info!("insert sota {} rescords", r.0);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn show_reference(&self, event: &FindRef) -> AppResult<SOTAReference> {
        let query = findref_query_builder(SOTA, event);
        let result = self.select(&query).await?;
        Ok(result.into())
    }

    async fn show_all_references(
        &self,
        event: &FindRef,
    ) -> AppResult<PagenatedResult<SotaReference>> {
        let limit = event.limit.unwrap_or(10);
        let offset = event.offset.unwrap_or(0);
        let query = findref_query_builder(SOTA, event);
        let (total, results) = self.select_pagenated(&query).await?;
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

        tracing::info!("update sota {} rescords", references.len());

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
            if r.0 % 500 == 0 {
                tracing::info!("upsert partial sota {} rescords", r.0);
            }
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

    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<SotaReference>> {
        let query = findref_query_builder(SOTA, event);
        let results = self.select_by_condition(&query).await?;
        let results = results.into_iter().map(SotaReference::from).collect();
        Ok(results)
    }
}
