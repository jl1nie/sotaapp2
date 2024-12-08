use async_trait::async_trait;
use common::error::{AppError, AppResult};
use derive_new::new;
use geo_types::Rect;
use sqlx::PgConnection;

use crate::database::ConnectionPool;
use application::model::sota::{
    event::{CreateRef, CreateRefs, DeleteRef, SearchRefs, SearchResults, UpdateRef, UpdateRefs},
    SOTABriefReference, SOTAReference,
};
use application::SOTADatabase;

#[derive(new)]
pub struct SOTADatabaseImpl {
    db: ConnectionPool,
}

impl SOTADatabaseImpl {
    async fn create(&self, event: CreateRef, db: &mut PgConnection) -> AppResult<()> {
        let r = event.inner_ref();
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
                $14, $15, TO_DATE($16,'DD/MM/YYYY'), TO_DATE($17,'DD/MM/YYYY'), $18, TO_DATE($19,'DD/MM/YYYY'), $20)
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
            r.lattitude,
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

    async fn upsert(&self, event: CreateRef, db: &mut PgConnection) -> AppResult<()> {
        let r = event.inner_ref();
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
                $14, $15, TO_DATE($16,'DD/MM/YYYY'), TO_DATE($17,'DD/MM/YYYY'), $18, TO_DATE($19,'DD/MM/YYYY'), $20)
                ON CONFLICT(summit_code) DO UPDATE
                    SET
                        association_name = EXCLUDED.association_name,
                        region_name = EXCLUDED.region_name,
                        summit_name = EXCLUDED.summit_name,
                        summit_name_j = EXCLUDED.summit_name_j,
                        city = EXCLUDED.city,
                        city_j = EXCLUDED.city_j,
                        alt_m = EXCLUDED.alt_m,
                        alt_ft = EXCLUDED.alt_ft,
                        grid_ref1 = EXCLUDED.grid_ref1,
                        grid_ref2 = EXCLUDED.grid_ref2,
                        coordinates = EXCLUDED.coordinates,
                        points = EXCLUDED.points,
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
            r.lattitude,
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
                    grid_ref2,
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

    async fn count_by_keywokrd(&self, keyword: &str) -> AppResult<usize> {
        let keyword = format!("%{}%", keyword);
        let row = sqlx::query!(
            r#"
                SELECT COUNT(*) FROM sota_references 
                WHERE summit_code LIKE $1 OR summit_name LIKE $1
                   OR summit_name_j LIKE $1
            "#,
            keyword
        )
        .fetch_one(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(row.count.unwrap_or(0) as usize)
    }

    async fn find_by_keyword(
        &self,
        keyword: &str,
        max_results: usize,
    ) -> AppResult<Vec<SOTABriefReference>> {
        let keyword = format!("%{}%", keyword);
        let rows: Vec<SOTABriefReference> = sqlx::query_as!(
            SOTABriefReference,
            r#"
                SELECT 
                    summit_code,
                    summit_name,
                    summit_name_j,
                    alt_m,
                    alt_ft,
                    ST_X(coordinates) AS longitude,
                    ST_Y(coordinates) AS lattitude,
                    points
                FROM sota_references
                WHERE summit_code LIKE $1 OR summit_name LIKE $1
                   OR summit_name_j LIKE $1
                LIMIT $2
            "#,
            keyword,
            max_results as i32
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(rows)
    }

    async fn find_by_location(
        &self,
        rect: &Rect,
        elevation: i32,
        max_results: usize,
    ) -> AppResult<Vec<SOTABriefReference>> {
        let rows: Vec<SOTABriefReference> = sqlx::query_as!(
            SOTABriefReference,
            r#"
                SELECT
                   summit_code,
                    summit_name,
                    summit_name_j,
                    alt_m,
                    alt_ft,
                    ST_X(coordinates) AS longitude,
                    ST_Y(coordinates) AS lattitude,
                    points
                FROM sota_references
                WHERE alt_m >= $5 AND ST_Within(coordinates, ST_MakeEnvelope($1, $2, $3, $4, 4326))
                ORDER BY alt_m DESC
                LIMIT $6
            "#,
            rect.min().x,
            rect.min().y,
            rect.max().x,
            rect.max().y,
            elevation,
            max_results as i32
        )
        .fetch_all(self.db.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(rows)
    }
}

#[async_trait]
impl SOTADatabase for SOTADatabaseImpl {
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
        for r in event.requests {
            self.create(r, &mut tx).await?;
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
        for r in event.requests {
            self.update(r, &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn search(&self, event: SearchRefs) -> AppResult<SearchResults> {
        let SearchRefs {
            summit_code,
            keyword,
            max_results,
            elevation,
            region,
        } = event;
        if summit_code.is_some() {
            if let Some(result) = self.find_by_summit_code(&summit_code.unwrap()).await? {
                Ok(SearchResults {
                    results: Some(vec![result]),
                    counts: 1,
                    ..Default::default()
                })
            } else {
                Ok(SearchResults {
                    ..Default::default()
                })
            }
        } else if keyword.is_some() {
            let keyword = keyword.unwrap();
            let counts = self.count_by_keywokrd(&keyword).await?;
            let max_results: usize = max_results.unwrap_or(100000);
            if counts > max_results {
                Ok(SearchResults {
                    counts,
                    ..Default::default()
                })
            } else {
                let brief_results = self.find_by_keyword(&keyword, max_results).await?;
                Ok(SearchResults {
                    counts: brief_results.len(),
                    brief_results: Some(brief_results),
                    ..Default::default()
                })
            }
        } else if region.is_some() {
            let elevation = elevation.unwrap_or(0);
            let max_results: usize = max_results.unwrap_or(100000);
            let brief_results = self
                .find_by_location(&region.unwrap(), elevation, max_results)
                .await?;
            Ok(SearchResults {
                counts: brief_results.len(),
                brief_results: Some(brief_results),
                ..Default::default()
            })
        } else {
            Ok(SearchResults {
                ..Default::default()
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ConnectionPool;
    use crate::sota::SOTADatabaseImpl;
    use application::model::sota::event::CreateRefs;
    use csv::ReaderBuilder;
    use std::fs::File;

    use web_api::model::sota::CreateRefRequest;

    #[sqlx::test]
    async fn upload_summit_list(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = ConnectionPool::new(pool.clone());
        let sotadb = SOTADatabaseImpl { db };

        sqlx::query!(r#"DELETE FROM sota_references"#)
            .execute(&pool)
            .await?;

        let start = std::time::Instant::now();

        let file = File::open("../data/summitslist.csv")?;

        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(file);

        let mut sota_ref_list: Vec<CreateRef> = Vec::new();
        for result in rdr.records().skip(2) {
            let req: CreateRefRequest = result?.deserialize(None)?;
            sota_ref_list.push(req.into());
        }

        println!("read csv = {:?}", start.elapsed());

        let reqs = CreateRefs {
            requests: sota_ref_list,
        };

        sotadb.create_references(reqs).await?;
        println!("create table = {:?}", start.elapsed());

        let req = SearchRefs {
            keyword: Some("JA".to_string()),
            ..Default::default()
        };
        let counts = sotadb.search(req).await?;
        println!("search1 = {:?}", start.elapsed());

        let SearchResults { counts, .. } = counts;
        println!("counts = {:?}", counts);
        assert_eq!(counts, 7205usize);

        let req = SearchRefs {
            keyword: Some("JA/KN-0".to_string()),
            max_results: Some(100usize),
            ..Default::default()
        };
        let counts = sotadb.search(req).await?;
        println!("search2 = {:?}", start.elapsed());
        let SearchResults { counts, .. } = counts;
        println!("counts = {:?}", counts);
        assert_eq!(counts, 34usize);

        let req = SearchRefs {
            keyword: Some("JA".to_string()),
            max_results: Some(100usize),
            ..Default::default()
        };
        let counts = sotadb.search(req).await?;
        println!("search3 = {:?}", start.elapsed());
        let SearchResults { counts, .. } = counts;
        println!("counts = {:?}", counts);
        assert_eq!(counts, 7205usize);
        //panic!();
        Ok(())
    }
}
