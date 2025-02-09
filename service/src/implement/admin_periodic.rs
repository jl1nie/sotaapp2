use aprs_message::AprsData;
use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use domain::repository::aprs::AprsLogRepository;
use shaku::Component;
use std::sync::Arc;

use common::{config::AppConfig, error::AppResult};
use domain::model::{activation::Alert, activation::Spot, event::DeleteAct};
use domain::repository::{activation::ActivationRepositry, aprs::AprsRepositry};

use crate::services::AdminPeriodicService;

#[derive(Component)]
#[shaku(interface = AdminPeriodicService)]
pub struct AdminPeriodicServiceImpl {
    #[shaku(inject)]
    act_repo: Arc<dyn ActivationRepositry>,
    #[shaku(inject)]
    aprs_repo: Arc<dyn AprsRepositry>,
    #[shaku(inject)]
    aprs_log_repo: Arc<dyn AprsLogRepository>,

    config: AppConfig,
}

#[async_trait]
impl AdminPeriodicService for AdminPeriodicServiceImpl {
    async fn update_alerts(&self, alerts: Vec<Alert>) -> AppResult<()> {
        tracing::info!("Update {} alerts", alerts.len());

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

        buddy.push("JL1NIE-*".to_string());

        if !buddy.is_empty() {
            self.aprs_repo.set_buddy_list(buddy).await?;
            //self.aprs_repo
            //    .set_filter("r/35.684074/139.75296/100 b/JL1NIE".to_string())
            //    .await?;
        }

        self.act_repo.update_alerts(alerts).await?;

        let expire = now - self.config.alert_expire;
        self.act_repo
            .delete_alerts(DeleteAct { before: expire })
            .await?;

        let expire = now - self.config.aprslog_expire;
        self.aprs_log_repo
            .delete_aprs_log(&expire.naive_utc())
            .await?;

        Ok(())
    }

    async fn update_spots(&self, spots: Vec<Spot>) -> AppResult<()> {
        tracing::info!("Update {} spots", spots.len());

        self.act_repo.update_spots(spots).await?;

        let expire: DateTime<Utc> = Utc::now() - self.config.alert_expire;
        self.act_repo
            .delete_spots(DeleteAct { before: expire })
            .await?;
        Ok(())
    }

    async fn aprs_packet_received(&self, packet: AprsData) -> AppResult<()> {
        match packet {
            AprsData::AprsMesasge {
                ref callsign,
                addressee,
                message,
            } => {
                tracing::info!(
                    "APRS message from = {:?} to = {:} message = {:}",
                    callsign,
                    addressee,
                    message
                );

                let msgcall: String = callsign.into();
                let message = format!("{}:{}", msgcall, message);

                self.aprs_repo.write_message(callsign, &message).await?;
            }
            AprsData::AprsPosition {
                ref callsign,
                latitude,
                longitude,
            } => {
                if let Some(ssid) = callsign.ssid {
                    if [5, 6, 7, 8, 9].contains(&ssid) {
                        tracing::info!(
                            "APRS position from = {:?} lon={} lat={}",
                            callsign,
                            longitude,
                            latitude
                        );
                        let time = Utc::now().naive_utc();
                        let log = domain::model::aprslog::AprsLog {
                            callsign: callsign.into(),
                            ssid,
                            destination: "".to_string(),
                            state: domain::model::aprslog::AprsState::Approaching {
                                time,
                                distance: 0.0,
                            },
                            longitude,
                            latitude,
                        };
                        self.aprs_log_repo.insert_aprs_log(log).await?;
                    }
                }
            }
        };
        Ok(())
    }
}
