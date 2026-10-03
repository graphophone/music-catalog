use serde::Deserialize;
use config::{Config as _Config, File};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub postgres: PostgresConfig,
    pub streaming: StreamingConfig,
    pub rustfs: RustfsConfig,
}

impl Config {
    pub fn build(filename: &str) -> Result<Config, config::ConfigError> {
        let cfg = _Config::builder()
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

#[derive(Debug, Deserialize)]
pub struct StreamingConfig {
    pub play_token_key: String,
}

#[derive(Debug, Deserialize)]
pub struct RustfsConfig {
    pub access_key: String,
    pub secret_key: String,
    pub endpoint_url: String,
    pub region: String,
    pub assets_bucket: String,
}