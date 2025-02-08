use anyhow::{bail, Result};
use aprs_message::{AprsData, AprsIS};
use async_trait::async_trait;
use shaku::Component;

use common::config::AppConfig;
use common::error::{AppError, AppResult};
use domain::repository::aprs::AprsRepositry;

pub async fn connect_aprsis_with(cfg: &AppConfig) -> Result<AprsIS> {
    if let Ok(aprs) = AprsIS::connect(&cfg.aprs_host, &cfg.aprs_user, &cfg.aprs_password).await {
        return Ok(aprs);
    };
    bail!("Failed to connect to APRS-IS {}", cfg.aprs_host)
}

#[derive(Component)]
#[shaku(interface = AprsRepositry)]
pub struct AprsRepositryImpl {
    aprs: AprsIS,
}

#[async_trait]
impl AprsRepositry for AprsRepositryImpl {
    async fn write_message(&self, addressee: &str, message: &str) -> AppResult<()> {
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
