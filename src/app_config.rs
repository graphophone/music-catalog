use serde::Deserialize;
use config::{Config, File};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub postgres: PostgresConfig,
}

impl AppConfig {
    pub fn build(filename: &str) -> Result<AppConfig, config::ConfigError> {
        let cfg = Config::builder()
            .add_source(File::with_name(filename))
            .build()?;
        cfg.try_deserialize()
    }
}

#[derive(Debug, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}