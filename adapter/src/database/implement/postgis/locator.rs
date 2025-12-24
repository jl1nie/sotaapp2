use async_trait::async_trait;
use serde_json::json;
use serde_json::Value;
use shaku::Component;
use sqlx::PgConnection;

use common::config::AppConfig;
use common::error::{db_error, tx_error, AppResult};
use common::http;
use domain::model::locator::MunicipalityCenturyCode;

use crate::database::connect::ConnectionPool;
use crate::database::model::locator::MunicipalityCenturyCodeRow;
use domain::repository::locator::LocatorRepositry;

#[derive(Component)]
#[shaku(interface = LocatorRepositry)]
pub struct LocatorRepositryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

impl LocatorRepositryImpl {
    async fn update(&self, m: MunicipalityCenturyCodeRow, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO municipality_century_codes(muni_code, prefecture, municipality, jcc_code, ward_code, jcc_text, jcg_code, jcg_text, hamlog_code)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (muni_code) DO UPDATE
                SET muni_code = EXCLUDED.muni_code,
                    prefecture = EXCLUDED.prefecture,
                    municipality = EXCLUDED.municipality,
                    jcc_code = EXCLUDED.jcc_code,
                    ward_code = EXCLUDED.ward_code,
                    jcc_text = EXCLUDED.jcc_text,
                    jcg_code = EXCLUDED.jcg_code,
                    jcg_text = EXCLUDED.jcg_text,
                    hamlog_code = EXCLUDED.hamlog_code
            "#,
            m.muni_code as i32,
            m.prefecture,
            m.municipality,
            m.jcc_code,
            m.ward_code,
            m.jcc_text,
            m.jcg_code,
            m.jcg_text,
            m.hamlog_code
        )
        .execute(db)
        .await
        .map_err(db_error("locator operation postgis"))?;
        Ok(())
    }

    async fn find_location_by_muni_code(
        &self,
        muni_code: i32,
    ) -> AppResult<MunicipalityCenturyCode> {
        let results = sqlx::query_as!(
            MunicipalityCenturyCodeRow,
            r#"
                SELECT muni_code, prefecture, municipality, jcc_code, ward_code, jcc_text, jcg_code, jcg_text, hamlog_code
                FROM municipality_century_codes
                WHERE muni_code = $1            
                "#,
            muni_code
        )
        .fetch_one(self.pool.inner_ref())
        .await
        .map_err(db_error("locator operation postgis"))?;
        Ok(results.into())
    }
}

#[async_trait]
impl LocatorRepositry for LocatorRepositryImpl {
    async fn upload_muni_century_list(&self, table: Vec<MunicipalityCenturyCode>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("locator transaction postgis"))?;

        for r in table.into_iter().enumerate() {
            self.update(MunicipalityCenturyCodeRow::from(r.1), &mut tx)
                .await?;
            if r.0 % 100 == 0 {
                tracing::info!("insert db {} rescords", r.0);
            }
        }
        tx.commit()
            .await
            .map_err(tx_error("locator transaction postgis"))?;
        Ok(())
    }

    async fn find_location_by_muni_code(
        &self,
        muni_code: i32,
    ) -> AppResult<MunicipalityCenturyCode> {
        let result = self.find_location_by_muni_code(muni_code).await?;
        Ok(result)
    }

    async fn find_mapcode(&self, lon: f64, lat: f64) -> AppResult<String> {
        let client = http::client();
        let response = client
            .post(self.config.mapcode_endpoint.clone())
            .json(&json!({
                "lng": lon,
                "lat": lat,
            }))
            .send()
            .await
            .map_err(AppError::PostError)?;
        let response_json = response
            .json::<Value>()
            .await
            .map_err(AppError::PostError)?;
        let mapcode = response_json["mapcode"].as_str().unwrap_or("----------");
        Ok(mapcode.to_string())
    }
}
