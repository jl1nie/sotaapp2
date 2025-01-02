use async_trait::async_trait;
use shaku::Component;
use sqlx::PgConnection;

use common::error::{AppError, AppResult};

use domain::model::common::event::{DeleteLog, DeleteRef, FindRef};
use domain::model::pota::{POTAActivatorLog, POTAHunterLog, POTAReference, ParkCode};

use crate::database::model::pota::{POTAActivatorLogImpl, POTAHunterLogImpl, POTAReferenceImpl};
use crate::database::ConnectionPool;
use crate::implement::querybuilder::findref_query_builder;

use domain::repository::pota::POTAReferenceRepositry;

#[derive(Component)]
#[shaku(interface = POTAReferenceRepositry)]
pub struct POTAReferenceRepositryImpl {
    pool: ConnectionPool,
}

impl POTAReferenceRepositryImpl {
    async fn create(&self, r: POTAReferenceImpl, db: &mut PgConnection) -> AppResult<()> {
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
                    coordinates,
                    update
                )
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9,ST_SetSRID(ST_MakePoint($10, $11), 4326), 
                $12)
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
            r.lattitude,
            r.update
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn update(&self, r: POTAReferenceImpl, db: &mut PgConnection) -> AppResult<()> {
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
                    coordinates = ST_SetSRID(ST_MakePoint($10, $11), 4326),
                    update = $12
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
            r.lattitude,
            r.update
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete(&self, ref_id: ParkCode, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM pota_references
               WHERE pota_code = $1
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
                DELETE FROM pota_references
            "#
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn update_activator_log(
        &self,
        r: POTAActivatorLogImpl,
        db: &mut PgConnection,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO pota_activator_log (user_id, dx_entity, location, hasc, pota_code, park_name, first_qso_date, attempts, activations, qsos, upload)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (user_id, pota_code) DO UPDATE
                SET dx_entity = EXCLUDED.dx_entity,
                    location = EXCLUDED.location,
                    hasc = EXCLUDED.hasc,
                    pota_code = EXCLUDED.pota_code,
                    park_name = EXCLUDED.park_name,
                    first_qso_date = EXCLUDED.first_qso_date,
                    attempts = EXCLUDED.attempts,
                    activations = EXCLUDED.activations,
                    qsos = EXCLUDED.qsos,
                    upload = EXCLUDED.upload
            "#,
            r.user_id.raw(),
            r.dx_entity,
            r.location,
            r.hasc,
            r.pota_code,
            r.park_name,
            r.first_qso_date,
            r.attempts,
            r.activations,
            r.qsos,
            r.upload
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn update_hunter_log(
        &self,
        r: POTAHunterLogImpl,
        db: &mut PgConnection,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO pota_hunter_log (user_id, dx_entity, location, hasc, pota_code, park_name, first_qso_date, qsos, upload)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (user_id, pota_code) DO UPDATE
                SET dx_entity = EXCLUDED.dx_entity,
                    location = EXCLUDED.location,
                    hasc = EXCLUDED.hasc,
                    pota_code = EXCLUDED.pota_code,
                    park_name = EXCLUDED.park_name,
                    first_qso_date = EXCLUDED.first_qso_date,
                    qsos = EXCLUDED.qsos,
                    upload = EXCLUDED.upload
            "#,
            r.user_id.raw(),
            r.dx_entity,
            r.location,
            r.hasc,
            r.pota_code,
            r.park_name,
            r.first_qso_date,
            r.qsos,
            r.upload
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete_log(&self, d: DeleteLog, db: &mut PgConnection) -> AppResult<()> {
        let before = d.before;
        sqlx::query!(
            r#"
                DELETE FROM pota_activator_log
                WHERE upload < $1
            "#,
            before,
        )
        .execute(&mut *db)
        .await
        .map_err(AppError::SpecificOperationError)?;

        sqlx::query!(
            r#"
                DELETE FROM pota_hunter_log
                WHERE upload < $1
            "#,
            before,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(())
    }

    async fn select_by_condition(
        &self,
        query: &str,
        // params: &Vec<String>,
    ) -> AppResult<Vec<POTAReferenceImpl>> {
        let mut select = r#"
            SELECT
                pota_code,
                wwff_code,
                park_name,
                park_name_j,
                park_location,
                park_locid,
                park_type,
                park_status,
                park_area,
                ST_X(coordinates) AS longitude,
                ST_Y(coordinates) AS latitude,
                update
            FROM pota_references WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, POTAReferenceImpl>(&select);
        let rows: Vec<POTAReferenceImpl> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;
        Ok(rows)
    }
}

#[async_trait]
impl POTAReferenceRepositry for POTAReferenceRepositryImpl {
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
                tracing::info!("insert db {} rescords", r.0);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn find_reference(&self, event: &FindRef) -> AppResult<Vec<POTAReference>> {
        let query = findref_query_builder(event);
        let results = self.select_by_condition(&query).await?;
        let results = results.into_iter().map(POTAReference::from).collect();
        Ok(results)
    }

    async fn update_reference(&self, references: Vec<POTAReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        for r in references.into_iter().enumerate() {
            self.update(POTAReferenceImpl::from(r.1), &mut tx).await?;
            if r.0 % 500 == 0 {
                tracing::info!("update db {} rescords", r.0);
            }
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
        for r in logs.into_iter().enumerate() {
            self.update_activator_log(POTAActivatorLogImpl::from(r.1), &mut tx)
                .await?;
            if r.0 % 50 == 0 {
                tracing::info!("update activator log {} rescords", r.0);
            }
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
        for r in logs.into_iter().enumerate() {
            self.update_hunter_log(POTAHunterLogImpl::from(r.1), &mut tx)
                .await?;
            if r.0 % 50 == 0 {
                tracing::info!("update hunter log {} rescords", r.0);
            }
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
}
