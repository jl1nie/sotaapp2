use async_trait::async_trait;
use shaku::Component;
use sqlx::PgConnection;

use common::config::AppConfig;
use common::error::{AppError, AppResult};

use domain::model::pota::{POTAActivatorLog, POTAHunterLog, POTAReference, ParkCode};

use domain::model::common::event::{DeleteLog, DeleteRef, FindRef, FindResult};

use crate::database::model::pota::{POTAActivatorLogImpl, POTAHunterLogImpl, POTAReferenceImpl};
use crate::database::ConnectionPool;

use domain::repository::pota::POTAReferenceRepositry;

#[derive(Component)]
#[shaku(interface = POTAReferenceRepositry)]
pub struct POTAReferenceRepositryImpl {
    config: AppConfig,
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
    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<POTAReference>> {
        tracing::info!("Find POTA references with {:?}.", event);
        todo!()
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
        tracing::info!("Upload POTA activator log.");
        Ok(())
    }

    async fn upload_hunter_log(&self, logs: Vec<POTAHunterLog>) -> AppResult<()> {
        tracing::info!("Upload POTA hunter log.");
        Ok(())
    }

    async fn delete_activator_log(&self, query: DeleteLog) -> AppResult<()> {
        tracing::info!("Delete Activator log.");
        Ok(())
    }

    async fn delete_hunter_log(&self, query: DeleteLog) -> AppResult<()> {
        tracing::info!("Delete Hunter log.");
        Ok(())
    }
}
