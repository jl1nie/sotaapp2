use async_trait::async_trait;
use shaku::Component;
use sqlx::PgConnection;

use common::config::AppConfig;
use common::error::{AppError, AppResult};

use domain::model::common::activation::{Alert, Spot};
use domain::model::common::event::{DeleteAct, FindAct, FindResult, UpdateAct};
use domain::repository::activation::ActivationRepositry;

use crate::database::model::activation::{AlertImpl, SpotImpl};
use crate::database::ConnectionPool;
use crate::implement::querybuilder::findact_query_builder;

#[derive(Component)]
#[shaku(interface = ActivationRepositry)]
pub struct ActivationRepositryImpl {
    config: AppConfig,
    pool: ConnectionPool,
}

impl ActivationRepositryImpl {
    async fn update_alert_impl(&self, a: AlertImpl, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO alerts (program, alert_id, user_id, reference, reference_detail, 
                                   "location", activator, activator_name, start_time, end_time, frequencies,comment,poster)
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT (program, alert_id ) DO UPDATE             
                SET program = EXCLUDED.program,
                    alert_id = EXCLUDED.alert_id,
                    user_id = EXCLUDED.user_id,
                    reference = EXCLUDED.reference,
                    reference_detail = EXCLUDED.reference_detail,
                    "location" = EXCLUDED."location",
                    activator = EXCLUDED. activator,
                    activator_name = EXCLUDED.activator_name,
                    start_time = EXCLUDED.start_time,
                    end_time = EXCLUDED.end_time,
                    frequencies = EXCLUDED.frequencies,
                    comment = EXCLUDED.comment,
                    poster = EXCLUDED.poster
            "#,
            a.program,
            a.alert_id,
            a.user_id,
            a.reference,
            a.reference_detail,
            a.location,
            a.activator,
            a.activator_name,
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

    async fn update_spot_impl(&self, s: SpotImpl, db: &mut PgConnection) -> AppResult<()> {
        sqlx::query!(
            r#"
                INSERT INTO spots (program, spot_id, reference, reference_detail, activator, activator_name, spot_time, frequency, mode, spotter,comment) 
                VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (program, spot_id ) DO UPDATE             
                SET program = EXCLUDED.program,
                    spot_id = EXCLUDED.spot_id,
                    reference = EXCLUDED.reference,
                    reference_detail = EXCLUDED.reference_detail,
                    activator = EXCLUDED.activator,
                    activator_name = EXCLUDED.activator_name,
                    spot_time = EXCLUDED.spot_time,
                    frequency = EXCLUDED.frequency,
                    mode = EXCLUDED.mode,
                    spotter = EXCLUDED.spotter,
                    comment = EXCLUDED.comment
            "#,
            s.program,
            s.spot_id,
            s.reference,
            s.reference_detail,
            s.activator,
            s.activator_name,
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

    async fn delete_alerts_impl(&self, d: DeleteAct, db: &mut PgConnection) -> AppResult<()> {
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

    async fn delete_spots_impl(&self, d: DeleteAct, db: &mut PgConnection) -> AppResult<()> {
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

    async fn select_alerts_by_condition(&self, query: &str) -> AppResult<Vec<Alert>> {
        let mut select = r#"
            SELECT
                program,
                alert_id,
                user_id,
                reference,
                reference_detail,
                location,
                activator,
                activator_name,
                start_time,
                end_time,
                frequencies,
                comment,
                poster,
            FROM alerts WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, AlertImpl>(&select);

        let rows: Vec<AlertImpl> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;

        Ok(rows.into_iter().map(Alert::from).collect())
    }

    async fn select_spots_by_condition(&self, query: &str) -> AppResult<Vec<Spot>> {
        let mut select = r#"
            SELECT
                program,
                spot_id,
                reference,
                reference_detail,
                activator,
                activator_name,
                spot_time,
                frequency,
                mode,
                spotter,
                comment
            FROM spots WHERE "#
            .to_string();

        select.push_str(query);

        let sql_query = sqlx::query_as::<_, SpotImpl>(&select);

        let rows: Vec<SpotImpl> = sql_query
            .fetch_all(self.pool.inner_ref())
            .await
            .map_err(AppError::SpecificOperationError)?;

        Ok(rows.into_iter().map(Spot::from).collect())
    }
}

#[async_trait]
impl ActivationRepositry for ActivationRepositryImpl {
    async fn update_alerts(&self, event: UpdateAct<Alert>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        for r in event.requests.into_iter().enumerate() {
            self.update_alert_impl(AlertImpl::from(r.1), &mut tx)
                .await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn update_spots(&self, event: UpdateAct<Spot>) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        for r in event.requests.into_iter().enumerate() {
            self.update_spot_impl(SpotImpl::from(r.1), &mut tx).await?;
        }
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }
    async fn delete_alerts(&self, event: DeleteAct) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.delete_alerts_impl(event, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn delete_spots(&self, event: DeleteAct) -> AppResult<()> {
        let mut tx = self
            .pool
            .inner_ref()
            .begin()
            .await
            .map_err(AppError::TransactionError)?;

        self.delete_spots_impl(event, &mut tx).await?;
        tx.commit().await.map_err(AppError::TransactionError)?;
        Ok(())
    }

    async fn find_alerts(&self, event: &FindAct) -> AppResult<FindResult<Alert>> {
        let query = findact_query_builder(true, event);
        let results = self.select_alerts_by_condition(&query).await?;
        Ok(FindResult::new(results))
    }

    async fn find_spots(&self, event: &FindAct) -> AppResult<FindResult<Spot>> {
        let query = findact_query_builder(false, event);
        let results = self.select_spots_by_condition(&query).await?;
        Ok(FindResult::new(results))
    }
}
