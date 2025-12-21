use aprs_message::AprsCallsign;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use shaku::Component;
use sqlx::SqliteConnection;

use crate::database::connect::ConnectionPool;
use crate::database::model::aprslog::AprsLogRow;
use common::error::{db_error, tx_error, AppResult};
use domain::model::{aprslog::AprsLog, event::FindAprs};
use domain::repository::aprs::AprsLogRepository;

#[derive(Component)]
#[shaku(interface = AprsLogRepository)]
pub struct AprsLogRepositoryImpl {
    pool: ConnectionPool,
}

impl AprsLogRepositoryImpl {
    async fn select_by_callsign(&self, callsign: &AprsCallsign) -> AppResult<Vec<AprsLogRow>> {
        let result = if let Some(ssid) = &callsign.ssid {
            sqlx::query_as!(
                AprsLogRow,
                r#"
                SELECT
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    message,
                    longitude,
                    latitude
                FROM aprs_log WHERE callsign = $1 AND ssid = $2
                ORDER BY time DESC
            "#,
                callsign.callsign,
                ssid
            )
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(db_error("fetch aprs_log by callsign+ssid"))?
        } else {
            sqlx::query_as!(
                AprsLogRow,
                r#"
                SELECT
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    message,
                    longitude,
                    latitude
                FROM aprs_log WHERE callsign = $1
                ORDER BY time DESC
            "#,
                callsign.callsign
            )
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(db_error("fetch aprs_log by callsign"))?
        };

        Ok(result)
    }

    async fn select_by_reference_time(
        &self,
        region: &String,
        after: &NaiveDateTime,
    ) -> AppResult<Vec<AprsLogRow>> {
        let result = sqlx::query_as!(
            AprsLogRow,
            r#"
                SELECT
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    message,
                    longitude,
                    latitude
                FROM aprs_log WHERE time > $1 AND destination LIKE $2
                ORDER BY time DESC
            "#,
            after,
            region
        )
        .fetch_all(self.pool.inner_ref())
        .await
        .map_err(db_error("fetch aprs_log by reference_time"))?;

        Ok(result)
    }

    async fn insert(&self, log: AprsLogRow, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO aprs_log (
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    message,
                    longitude,
                    latitude
                ) VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            log.time,
            log.callsign,
            log.ssid,
            log.destination,
            log.distance,
            log.state,
            log.message,
            log.longitude,
            log.latitude,
        )
        .execute(db)
        .await
        .map_err(db_error("insert aprs_log"))?;
        Ok(())
    }

    async fn delete(&self, before: &NaiveDateTime, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                DELETE FROM aprs_log WHERE time < $1
            "#,
            before,
        )
        .execute(db)
        .await
        .map_err(db_error("delete aprs_log"))?;
        Ok(())
    }
}

#[async_trait]
impl AprsLogRepository for AprsLogRepositoryImpl {
    async fn find_aprs_log(&self, query: &FindAprs) -> AppResult<Vec<AprsLog>> {
        let result = if let Some(ref callsign) = query.callsign {
            self.select_by_callsign(callsign).await?
        } else {
            let after = query.after.unwrap_or_default();

            let mut reference = query.reference.clone().unwrap_or_default();
            reference.push('%');

            self.select_by_reference_time(&reference, &after.naive_utc())
                .await?
        };

        let logs = result.into_iter().map(AprsLog::from).collect();

        Ok(logs)
    }

    async fn insert_aprs_log(&self, aprs_log: AprsLog) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin insert_aprs_log"))?;

        self.insert(aprs_log.into(), &mut tx).await?;
        tx.commit()
            .await
            .map_err(tx_error("commit insert_aprs_log"))?;

        Ok(())
    }

    async fn delete_aprs_log(&self, before: &NaiveDateTime) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(tx_error("begin delete_aprs_log"))?;

        self.delete(before, &mut tx).await?;
        tx.commit()
            .await
            .map_err(tx_error("commit delete_aprs_log"))?;

        Ok(())
    }
}
