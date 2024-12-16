use async_trait::async_trait;
use shaku::Component;
use sqlx::PgConnection;

use common::config::AppConfig;
use common::error::{AppError, AppResult};
use domain::model::common::event::{CreateRef, DeleteRef, FindRef, FindResult, UpdateRef};
use domain::model::sota::{SOTARefOptInfo, SOTAReference, SummitCode};

use crate::database::ConnectionPool;
use crate::implement::querybuilder::query_builder;
use domain::repository::sota::SOTAReferenceReposity;

#[derive(Component)]
#[shaku(interface = SOTAReferenceReposity)]
pub struct SOTAReferenceReposityImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

impl SOTAReferenceReposityImpl {
    async fn create(&self, r: SOTAReference, db: &mut PgConnection) -> AppResult<()> {
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
                    points,
                    bonus_points,
                    valid_from,
                    valid_to,
                    activation_count,
                    activation_date,
                    activation_call
                )
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, ST_SetSRID(ST_MakePoint($12, $13), 4326), 
                $14, $15, $16, $17, $18, $19, $20)
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

    async fn update_opt(&self, r: SOTARefOptInfo, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                UPDATE sota_references SET
                    summit_name = $1,
                    summit_name_j = $2,
                    city = $3,
                    city_j = $4
               WHERE summit_code = $5
            "#,
            r.summit_name,
            r.summit_name_j,
            r.city,
            r.city_j,
            r.summit_code,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete_opt(&self, ref_id: SummitCode, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                UPDATE sota_references SET
                    summit_name_j = NULL,
                    city = NULL,
                    city_j = NULL
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

    async fn select_by_condition(
        &self,
        query: &str,
        // params: &Vec<String>,
    ) -> AppResult<Vec<SOTAReference>> {
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

        let sql_query = sqlx::query_as::<_, SOTAReference>(&select);

        let rows: Vec<SOTAReference> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;

        Ok(rows)
    }
}

#[async_trait]
impl SOTAReferenceReposity for SOTAReferenceReposityImpl {
    async fn import_reference(&self, event: CreateRef<SOTAReference>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.delete_all(&mut tx).await?;

        for r in event.requests.into_iter().enumerate() {
            self.create(r.1, &mut tx).await?;
            if r.0 % 1000 == 0 {
                eprintln!("insert db {} rescords", r.0);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn update_reference_opt(&self, event: UpdateRef<SOTARefOptInfo>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        for r in event.requests.into_iter().enumerate() {
            self.update_opt(r.1, &mut tx).await?;
            if r.0 % 1000 == 0 {
                eprintln!("update db {} rescords", r.0);
            }
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_reference_opt(&self, event: DeleteRef<SummitCode>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        self.delete_opt(event.ref_id, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn find_reference(&self, event: &FindRef) -> AppResult<FindResult<SOTAReference>> {
        let query = query_builder(event);
        let results = self.select_by_condition(&query).await?;
        Ok(FindResult {
            counts: results.len(),
            results: Some(results),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ConnectionPool;
    use common::config::AppConfigBuilder;
    use common::csv_reader::csv_reader;
    use serde::Deserialize;
    use std::{
        fs::{read, File},
        io::Read,
    };

    #[derive(Debug, Deserialize)]
    pub struct SOTACSVFile {
        pub summit_code: String,
        pub association_name: String,
        pub region_name: String,
        pub summit_name: String,
        pub alt_m: i32,
        pub alt_ft: i32,
        pub grid_ref1: String,
        pub grid_ref2: String,
        pub longitude: Option<f64>,
        pub latitude: Option<f64>,
        pub points: i32,
        pub bonus_points: i32,
        pub valid_from: Option<String>,
        pub valid_to: Option<String>,
        pub activation_count: i32,
        pub activation_date: Option<String>,
        pub activation_call: Option<String>,
    }
    #[sqlx::test]
    async fn upload_summit_list(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let pool = ConnectionPool::new(pool.clone());
        let config = AppConfigBuilder::default().database(None).build();
        let sotadb = SOTAReferenceReposityImpl { config, pool };

        let mut file = File::open("../data/summitslist.csv")?;
        let mut rdr = String::new();
        file.read_to_string(&mut rdr);

        let reflist: Vec<SOTACSVFile> = csv_reader(rdr, 2).unwrap();

        Ok(())
    }
}
