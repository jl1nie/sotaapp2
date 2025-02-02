pub mod connect {
    use anyhow::Result;
    use aprs_message::AprsIS;
    use common::config::AppConfig;

    pub async fn connect_aprsis_with(cfg: &AppConfig) -> Result<AprsIS> {
        let aprs = AprsIS::connect(&cfg.aprs_host, &cfg.aprs_user, &cfg.aprs_password).await?;
        Ok(aprs)
    }
}
