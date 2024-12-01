use async_trait::async_trait;
use common::error::{AppError, AppResult};
use derive_new::new;
use geo_types::Rect;
use sqlx::PgConnection;

use crate::database::ConnectionPool;
use application::model::sota::{
    event::{CreateRef, CreateRefs, DeleteRef, UpdateRef, UpdateRefs},
    SOTAReference,
};
use application::SOTA;

#[derive(new)]
pub struct SOTAImpl {
    db: ConnectionPool,
}

impl SOTAImpl {
    async fn create(&self, event: CreateRef, db: &mut PgConnection) -> AppResult<()> {
        let SOTAReference {
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
            gird_ref2,
            longitude,
            lattitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call,
        } = event.0;
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
                    gird_ref2,
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
                $14, $15, TO_DATE($16,'DD/MM/YYYY'), TO_DATE($17,'DD/MM/YYYY'), $18, TO_DATE($19,'DD/MM/YYYY'), $20)
            "#,
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
            gird_ref2,
            longitude,
            lattitude,
            points,
            bonus_points,
            valid_from,
            valid_to,
            activation_count,
            activation_date,
            activation_call
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete(&self, event: DeleteRef, db: &mut PgConnection) -> AppResult<()> {
        let DeleteRef { summit_code } = event;

        sqlx::query!(
            r#"
                DELETE FROM sota_references
                WHERE summit_code = $1
            "#,
            summit_code
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn update(&self, event: UpdateRef, db: &mut PgConnection) -> AppResult<()> {
        let UpdateRef {
            summit_code,
            summit_name,
            summit_name_j,
            city,
            city_j,
            alt_m,
            longitude,
            lattitude,
        } = event;
        let mut query = String::from("UPDATE sota_references SET ");
        let mut params: Vec<String> = Vec::new();
        let mut index = 1;

        if let Some(val) = summit_name {
            query.push_str("summit_name = $1, ");
            params.push(val);
            index += 1;
        }

        if let Some(val) = summit_name_j {
            query.push_str("summit_name_j = $1, ");
            params.push(val);
            index += 1;
        }

        if let Some(val) = city {
            query.push_str("city = $1, ");
            params.push(val);
            index += 1;
        }

        if let Some(val) = city_j {
            query.push_str("city_j = $1, ");
            params.push(val);
            index += 1;
        }

        if let Some(val) = alt_m {
            query.push_str("alt_m = $1, ");
            params.push(val.to_string());
            index += 1;
        }

        if longitude.is_some() && lattitude.is_some() {
            let (longitude, lattitude) = (longitude.unwrap(), lattitude.unwrap());
            query.push_str("coordinates = ST_SetSRID(ST_MakePoint($1, $2), 4326), ");
            params.push(longitude.to_string());
            params.push(lattitude.to_string());
            index += 2;
        }

        query.pop();
        query.pop();
        query.push_str(&format!(" WHERE summit_code = ${}", index));
        params.push(summit_code);

        let mut sql_query = sqlx::query(&query);

        for param in params {
            sql_query = sql_query.bind(param);
        }

        sql_query
            .execute(db)
            .await
            .map_err(AppError::SpecificOperationError)?;

        Ok(())
    }
}

#[async_trait]
impl SOTA for SOTAImpl {
    async fn create_a_reference(&self, event: CreateRef) -> AppResult<()> {
        let mut tx = self
            .db
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        self.create(event, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn create_references(&self, event: CreateRefs) -> AppResult<()> {
        let mut tx = self
            .db
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        for r in event.refrences {
            self.create(CreateRef(r), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_a_reference(&self, event: DeleteRef) -> AppResult<()> {
        let mut tx = self
            .db
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        self.delete(event, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn update_a_reference(&self, event: UpdateRef) -> AppResult<()> {
        let mut tx = self
            .db
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        self.update(event, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn update_references(&self, event: UpdateRefs) -> AppResult<()> {
        let mut tx = self
            .db
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;
        for r in event.refrences {
            self.update(r, &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn find_by_summit_code(&self, summit_code: &str) -> AppResult<Option<SOTAReference>> {
        let row: Option<SOTAReference> = sqlx::query_as!(
            SOTAReference,
            r#"
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
                    gird_ref2,
                    ST_X(coordinates) AS longitude,
                    ST_Y(coordinates) AS lattitude,
                    points,
                    bonus_points,
                    TO_CHAR(valid_from,'DD/MM/YYYY') AS valid_from,
                    TO_CHAR(valid_to,'DD/MM/YYYY') AS valid_to,
                    activation_count,
                    TO_CHAR(activation_date,'DD/MM/YYYY') AS activation_date,
                    activation_call
                FROM sota_references
                WHERE summit_code = $1
            "#,
            summit_code
        )
        .fetch_optional(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(row)
    }

    async fn find_by_location(&self, rect: &Rect) -> AppResult<Vec<SOTAReference>> {
        let rows: Vec<SOTAReference> = sqlx::query_as!(
            SOTAReference,
            r#"
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
                    gird_ref2,
                    ST_X(coordinates) AS longitude,
                    ST_Y(coordinates) AS lattitude,
                    points,
                    bonus_points,
                    TO_CHAR(valid_from,'DD/MM/YYYY') AS valid_from,
                    TO_CHAR(valid_to,'DD/MM/YYYY') AS valid_to,
                    activation_count,
                    TO_CHAR(activation_date,'DD/MM/YYYY') AS activation_date,
                    activation_call
                FROM sota_references
                WHERE ST_Within(coordinates, ST_MakeEnvelope($1, $2, $3, $4, 4326))
            "#,
            rect.min().x,
            rect.min().y,
            rect.max().x,
            rect.max().y
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(rows)
    }
}
