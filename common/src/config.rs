use regex::Regex;
use std::time::Duration;

#[derive(Default, Clone)]
pub struct AppConfig {
    pub database: String,

    pub sota_alert_endpoint: String,
    pub sota_spot_endpoint: String,
    pub pota_alert_endpoint: String,
    pub pota_spot_endpoint: String,

    pub alert_update_schedule: String,
    pub alert_expire: Duration,
    pub spot_expire: Duration,
    pub spot_update_schedule: String,
    pub sota_import_association: Option<Regex>,
    pub log_expire: Duration,
}

pub struct AppConfigBuilder {
    config: AppConfig,
}

impl Default for AppConfigBuilder {
    fn default() -> Self {
        Self {
            config: AppConfig {
                ..Default::default()
            },
        }
    }
}

impl AppConfigBuilder {
    pub fn database(mut self, name: Option<&str>) -> Self {
        if let Some(name) = name {
            self.config.database = name.to_string();
        } else {
            let name = std::env::var("DATABASE_URL").unwrap_or_default();
            self.config.database = name;
        }
        self
    }

    pub fn sota_alert_endpoint(mut self, endpoint: &str) -> Self {
        self.config.sota_alert_endpoint = endpoint.to_string();
        self
    }

    pub fn sota_spot_endpoint(mut self, endpoint: &str) -> Self {
        self.config.sota_spot_endpoint = endpoint.to_string();
        self
    }

    pub fn sota_import_association(mut self, regex: &str) -> Self {
        let re = Regex::new(regex).unwrap();
        self.config.sota_import_association = Some(re);
        self
    }

    pub fn pota_alert_endpoint(mut self, endpoint: &str) -> Self {
        self.config.pota_alert_endpoint = endpoint.to_string();
        self
    }

    pub fn pota_spot_endpoint(mut self, endpoint: &str) -> Self {
        self.config.pota_spot_endpoint = endpoint.to_string();
        self
    }

    pub fn alert_expire(mut self, expire: Duration) -> Self {
        self.config.alert_expire = expire;
        self
    }

    pub fn alert_update_schedule(mut self, schedule: &str) -> Self {
        self.config.alert_update_schedule = schedule.to_string();
        self
    }

    pub fn spot_expire(mut self, expire: Duration) -> Self {
        self.config.spot_expire = expire;
        self
    }

    pub fn spot_update_schedule(mut self, schedule: &str) -> Self {
        self.config.spot_update_schedule = schedule.to_string();
        self
    }

    pub fn log_expire(mut self, expire: Duration) -> Self {
        self.config.log_expire = expire;
        self
    }

    pub fn build(self) -> AppConfig {
        self.config
    }
}
