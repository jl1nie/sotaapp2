use async_trait::async_trait;
use shaku::Component;
use sqlx::SqliteConnection;

use common::error::{AppError, AppResult};

use domain::model::activation::{Alert, Spot};
use domain::model::event::{DeleteAct, FindAct};
use domain::repository::activation::ActivationRepositry;

use super::querybuilder::findact_query_builder;
use crate::database::connect::ConnectionPool;
use crate::database::model::activation::{AlertRow, SpotRow};

#[derive(Component)]
#[shaku(interface = ActivationRepositry)]
pub struct ActivationRepositryImpl {
    pool: ConnectionPool,
}

impl ActivationRepositryImpl {
    async fn update_alert_impl(&self, a: AlertRow, db: &mut SqliteConnection) -> AppResult<()> {
        let program = a.program.as_i32();
        sqlx::query!(
            r#"
                INSERT INTO alerts (program, alert_id, user_id, reference, reference_detail, 
                                   "location", activator, activator_name, operator, start_time, end_time, frequencies,comment,poster)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                ON CONFLICT (program, alert_id ) DO UPDATE             
                SET program = EXCLUDED.program,
                    alert_id = EXCLUDED.alert_id,
                    user_id = EXCLUDED.user_id,
                    reference = EXCLUDED.reference,
                    reference_detail = EXCLUDED.reference_detail,
                    "location" = EXCLUDED."location",
                    activator = EXCLUDED. activator,
                    activator_name = EXCLUDED.activator_name,
                    operator = EXCLUDED.operator,
                    start_time = EXCLUDED.start_time,
                    end_time = EXCLUDED.end_time,
                    frequencies = EXCLUDED.frequencies,
                    comment = EXCLUDED.comment,
                    poster = EXCLUDED.poster
            "#,
            program,
            a.alert_id,
            a.user_id,
            a.reference,
            a.reference_detail,
            a.location,
            a.activator,
            a.activator_name,
            a.operator,
            a.start_time,
            a.end_time,
            a.frequencies,
            a.comment,
            a.poster
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn update_spot_impl(&self, s: SpotRow, db: &mut SqliteConnection) -> AppResult<()> {
        let program = s.program.as_i32();
        sqlx::query!(
            r#"
                INSERT INTO spots (program, spot_id, reference, reference_detail, activator, activator_name, operator, spot_time, frequency, mode, spotter,comment) 
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (program, spot_id ) DO UPDATE             
                SET program = EXCLUDED.program,
                    spot_id = EXCLUDED.spot_id,
                    reference = EXCLUDED.reference,
                    reference_detail = EXCLUDED.reference_detail,
                    activator = EXCLUDED.activator,
                    activator_name = EXCLUDED.activator_name,
                    operator = EXCLUDED.operator,
                    spot_time = EXCLUDED.spot_time,
                    frequency = EXCLUDED.frequency,
                    mode = EXCLUDED.mode,
                    spotter = EXCLUDED.spotter,
                    comment = EXCLUDED.comment
            "#,
            program,
            s.spot_id,
            s.reference,
            s.reference_detail,
            s.activator,
            s.activator_name,
            s.operator,
            s.spot_time,
            s.frequency,
            s.mode,
            s.spotter,
            s.comment,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete_alerts_impl(&self, d: DeleteAct, db: &mut SqliteConnection) -> AppResult<()> {
        let before = d.before;
        sqlx::query!(
            r#"
                DELETE FROM alerts
               WHERE start_time < $1
            "#,
            before,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn delete_spots_impl(&self, d: DeleteAct, db: &mut SqliteConnection) -> AppResult<()> {
        let before = d.before;
        sqlx::query!(
            r#"
                DELETE FROM spots
               WHERE spot_time < $1
            "#,
            before,
        )
        .execute(db)
        .await
        .map_err(AppError::SpecificOperationError)?;
        Ok(())
    }

    async fn select_alerts_by_condition(&self, query: &FindAct) -> AppResult<Vec<Alert>> {
        let select = r#"
            SELECT
                program,
                alert_id,
                user_id,
                reference,
                reference_detail,
                location,
                activator,
                activator_name,
                operator,
                start_time,
                end_time,
                frequencies,
                comment,
                poster
            FROM alerts WHERE "#;

        let mut builder = findact_query_builder(true, select, query);
        let sql_query = builder.build_query_as::<AlertRow>();

        let rows: Vec<AlertRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;

        Ok(rows.into_iter().map(Alert::from).collect())
    }

    async fn select_spots_by_condition(&self, query: &FindAct) -> AppResult<Vec<Spot>> {
        let select = r#"
            SELECT
                program,
                spot_id,
                reference,
                reference_detail,
                activator,
                activator_name,
                operator,
                spot_time,
                frequency,
                mode,
                spotter,
                comment
            FROM spots WHERE "#;

        let mut builder = findact_query_builder(false, select, query);
        let sql_query = builder.build_query_as::<SpotRow>();

        let rows: Vec<SpotRow> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(|e| AppError::RowNotFound {
                source: e,
                location: format!("{}:{}", file!(), line!()),
            })?;

        Ok(rows.into_iter().map(Spot::from).collect())
    }
}

#[async_trait]
impl ActivationRepositry for ActivationRepositryImpl {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        for r in alerts.into_iter().enumerate() {
            self.update_alert_impl(AlertRow::from(r.1), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        for r in spots.into_iter().enumerate() {
            self.update_spot_impl(SpotRow::from(r.1), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }
    async fn delete_alerts(&self, query: DeleteAct) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.delete_alerts_impl(query, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_spots(&self, query: DeleteAct) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.delete_spots_impl(query, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn find_alerts(&self, event: &FindAct) -> AppResult<Vec<Alert>> {
        let results = self.select_alerts_by_condition(event).await?;
        Ok(results)
    }

    async fn find_spots(&self, event: &FindAct) -> AppResult<Vec<Spot>> {
        let results = self.select_spots_by_condition(event).await?;
        Ok(results)
    }
}
