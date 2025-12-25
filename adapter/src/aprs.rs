use anyhow::{anyhow, Result};
use aprs_message::{AprsCallsign, AprsData, AprsIS};
use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::{AppError, AppResult};
use common::http::{with_retry, RetryConfig};
use domain::repository::aprs::AprsRepositry;

pub async fn connect_aprsis_with(cfg: &AppConfig) -> Result<AprsIS> {
    let host = cfg.aprs_host.clone();
    let user = cfg.aprs_user.clone();
    let password = cfg.aprs_password.clone();

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
    .ok_or_else(|| {
        anyhow!(
            "Failed to connect to APRS-IS {} after retries",
            cfg.aprs_host
        )
    })
}

#[derive(Component)]
#[shaku(interface = AprsRepositry)]
pub struct AprsRepositryImpl {
    aprs: AprsIS,
}

#[async_trait]
impl AprsRepositry for AprsRepositryImpl {
    async fn write_message(&self, addressee: &AprsCallsign, message: &str) -> AppResult<()> {
        self.aprs
            .write_message(addressee, message)
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(())
    }

    async fn set_filter(&self, filter: String) -> AppResult<()> {
        self.aprs
            .set_filter(filter)
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(())
    }

    async fn set_buddy_list(&self, buddy: Vec<String>) -> AppResult<()> {
        self.aprs
            .set_budlist_filter(buddy)
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(())
    }

    async fn get_aprs_packet(&self) -> AppResult<AprsData> {
        let packet = self
            .aprs
            .read_packet()
            .await
            .map_err(|_| AppError::APRSError)?;
        Ok(packet)
    }
}
