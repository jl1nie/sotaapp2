use async_trait::async_trait;
use chrono::NaiveDateTime;
use shaku::Component;
use sqlx::SqliteConnection;

use crate::database::connect::ConnectionPool;
use crate::database::model::aprslog::AprsLogImpl;
use common::error::{AppError, AppResult};
use domain::model::aprslog::AprsLog;
use domain::repository::aprs::AprsLogRepository;

#[derive(Component)]
#[shaku(interface = AprsLogRepository)]
pub struct AprsLogRepositoryImpl {
    pool: ConnectionPool,
}

impl AprsLogRepositoryImpl {
    async fn select_by_call(&self, callsign: &str) -> AppResult<Vec<AprsLogImpl>> {
        let result = sqlx::query_as!(
            AprsLogImpl,
            r#"
                SELECT
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    longitude,
                    latitude
                FROM aprs_log WHERE callsign = $1
            "#,
            callsign
        )
        .fetch_all(self.pool.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(result)
    }

    async fn select_by_time(&self, after: &NaiveDateTime) -> AppResult<Vec<AprsLogImpl>> {
        let result = sqlx::query_as!(
            AprsLogImpl,
            r#"
                SELECT
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    longitude,
                    latitude
                FROM aprs_log WHERE time > $1
            "#,
            after
        )
        .fetch_all(self.pool.inner_ref())
        .await
        .map_err(AppError::SpecificOperationError)?;

        Ok(result)
    }

    async fn insert(&self, log: AprsLogImpl, db: &mut SqliteConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO aprs_log (
                    time,
                    callsign,
                    ssid,
                    destination,
                    distance,
                    state,
                    longitude,
                    latitude
                ) VALUES($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            log.time,
            log.callsign,
            log.ssid,
            log.destination,
            log.distance,
            log.state,
            log.longitude,
            log.latitude,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;

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
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }
}

#[async_trait]
impl AprsLogRepository for AprsLogRepositoryImpl {
    async fn get_aprs_log_by_callsign(&self, callsign: &str) -> AppResult<Vec<AprsLog>> {
        let result = self.select_by_call(callsign).await?;
        let mut logs = Vec::new();
        for log in result {
            logs.push(log.into());
        }

        Ok(logs)
    }

    async fn get_aprs_log_by_time(&self, after: &NaiveDateTime) -> AppResult<Vec<AprsLog>> {
        let result = self.select_by_time(after).await?;
        let mut logs = Vec::new();
        for log in result {
            logs.push(log.into());
        }

        Ok(logs)
    }

    async fn insert_aprs_log(&self, aprs_log: AprsLog) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.insert(aprs_log.into(), &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;

        Ok(())
    }

    async fn delete_aprs_log(&self, before: &NaiveDateTime) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.delete(before, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;

        Ok(())
    }
}
