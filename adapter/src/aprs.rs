use anyhow::{anyhow, Result};
use aprs_message::{AprsCallsign, AprsData, AprsIS};
use async_trait::async_trait;
use shaku::Component;
use std::sync::Arc;
use tokio::sync::Mutex;

use common::config::AppConfig;
use common::error::{AppError, AppResult};
use common::http::{with_retry, RetryConfig};
use domain::repository::aprs::AprsRepositry;

pub async fn connect_aprsis_with(cfg: &AppConfig) -> Result<AprsIS> {
    connect_aprsis(&cfg.aprs_host, &cfg.aprs_user, &cfg.aprs_password).await
}

async fn connect_aprsis(host: &str, user: &str, password: &str) -> Result<AprsIS> {
    let host = host.to_string();
    let user = user.to_string();
    let password = password.to_string();

    with_retry("APRS-IS connect", &RetryConfig::default(), || {
        let h = host.clone();
        let u = user.clone();
        let p = password.clone();
        async move {
            AprsIS::connect(&h, &u, &p)
                .await
                .map_err(|e| anyhow!("{}", e))
        }
    })
    .await
    .ok_or_else(|| anyhow!("Failed to connect to APRS-IS {} after retries", host))
}

#[derive(Component)]
#[shaku(interface = AprsRepositry)]
pub struct AprsRepositryImpl {
    aprs: Arc<Mutex<AprsIS>>,
    aprs_host: String,
    aprs_user: String,
    aprs_password: String,
}

#[async_trait]
impl AprsRepositry for AprsRepositryImpl {
    async fn write_message(&self, addressee: &AprsCallsign, message: &str) -> AppResult<()> {
        self.aprs
            .lock()
            .await
            .write_message(addressee, message)
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(())
    }

    async fn set_filter(&self, filter: String) -> AppResult<()> {
        self.aprs
            .lock()
            .await
            .set_filter(filter)
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(())
    }

    async fn set_buddy_list(&self, buddy: Vec<String>) -> AppResult<()> {
        self.aprs
            .lock()
            .await
            .set_budlist_filter(buddy)
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(())
    }

    async fn get_aprs_packet(&self) -> AppResult<AprsData> {
        let mut aprs = self.aprs.lock().await;
        match aprs.read_packet().await {
            Ok(packet) => Ok(packet),
            Err(e) => {
                tracing::warn!("APRS-IS connection lost ({e}), reconnecting...");
                match connect_aprsis(&self.aprs_host, &self.aprs_user, &self.aprs_password).await {
                    Ok(new_conn) => {
                        *aprs = new_conn;
                        tracing::info!("APRS-IS reconnected successfully");
                        aprs.read_packet().await.map_err(|_| AppError::APRSError)
                    }
                    Err(e) => {
                        tracing::error!("APRS-IS reconnect failed: {e}");
                        Err(AppError::APRSError)
                    }
                }
            }
        }
    }
}
