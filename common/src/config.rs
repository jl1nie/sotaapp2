use std::time::Duration;

#[derive(Default, Clone)]
pub struct AppConfig {
    pub database: String,
    pub alert_expire: Duration,
    pub alert_schedule: String,
    pub spot_expire: Duration,
    pub spot_schedule: String,
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

    pub fn alert_expire(mut self, expire: Duration) -> Self {
        self.config.alert_expire = expire;
        self
    }

    pub fn alert_schedule(mut self, schedule: &str) -> Self {
        self.config.alert_schedule = schedule.to_string();
        self
    }

    pub fn spot_expire(mut self, expire: Duration) -> Self {
        self.config.spot_expire = expire;
        self
    }

    pub fn spot_schedule(mut self, schedule: &str) -> Self {
        self.config.spot_schedule = schedule.to_string();
        self
    }

    pub fn build(self) -> AppConfig {
        self.config
    }
}
