use anyhow::Result;

pub struct AppConfig {
    pub database: DatabaseConfig,
}

impl AppConfig {
    pub fn new() -> Result<Self> {
        let database = DatabaseConfig {
            url: std::env::var("DATABASE_URL")?,
        };
        Ok(Self { database })
    }
}

pub struct DatabaseConfig {
    pub url: String,
}
