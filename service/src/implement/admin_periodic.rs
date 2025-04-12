use aprs_message::AprsData;
use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use domain::repository::aprs::AprsLogRepository;
use shaku::Component;
use std::sync::Arc;

use common::{config::AppConfig, error::AppResult};
use domain::model::{activation::Alert, activation::Spot, event::DeleteAct};
use domain::repository::{
    activation::ActivationRepositry, aprs::AprsRepositry, sota::SotaRepository,
};

use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    pub act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    pub aprs_repo: Arc<dyn AprsRepositry>,
    #[shaku(inject)]
    pub aprs_log_repo: Arc<dyn AprsLogRepository>,
    #[shaku(inject)]
    pub sota_repo: Arc<dyn SotaRepository>,

    pub config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()> {
        let now: DateTime<Utc> = Utc::now();
        let alert_window_start = now - TimeDelta::hours(5);
        let alert_window_end = now + TimeDelta::hours(6);
        let mut buddy: Vec<_> = alerts
            .iter()
            .filter(|a| {
                a.program == domain::model::AwardProgram::SOTA
                    && a.start_time > alert_window_start
                    && a.start_time < alert_window_end
            })
            .map(|a| format!("{}-*", a.operator))
            .collect();

        if let Some(callsign) = self.config.aprs_user.split('-').next() {
            buddy.push(format!("{}-*", callsign));
        }

        self.aprs_repo.set_buddy_list(buddy).await?;

        self.act_repo.update_alerts(alerts).await?;

        let expire = now - self.config.alert_expire;
        self.act_repo
            .delete_alerts(DeleteAct { before: expire })
            .await?;

        let expire = now - self.config.aprs_log_expire;
        self.aprs_log_repo
            .delete_aprs_log(&expire.naive_utc())
            .await?;

        Ok(())
    }

    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()> {
        self.act_repo.update_spots(spots).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.act_repo
            .delete_spots(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn aprs_packet_received(&self, packet: AprsData) -> AppResult<()> {
        match packet {
            AprsData::AprsMessage {
                callsign,
                addressee,
                message,
            } => {
                tracing::info!(
                    "APRS message from = {:?} to = {:} message = {:}",
                    callsign,
                    addressee,
                    message
                );
                return self.process_message(&callsign, message).await;
            }
            AprsData::AprsPosition {
                callsign,
                latitude,
                longitude,
            } => {
                if let Some(ssid) = callsign.ssid {
                    if [5, 6, 7, 8, 9].contains(&ssid) {
                        return self.process_position(callsign, latitude, longitude).await;
                    }
                }
            }
        };
        Ok(())
    }
}
