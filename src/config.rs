use config::{Config as _Config, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub postgres: PostgresConfig,
    pub rustfs: RustfsConfig,
    pub rabbitmq: RabbitmqConfig,
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
pub struct RustfsConfig {
    pub access_key: String,
    pub secret_key: String,
    pub endpoint_url: String,
    pub deployed_url: String,
    pub region: String,
    pub thumbnails_bucket: String,
    pub raw_audio_bucket: String,
    pub hls_audio_bucket: String,
}

#[derive(Debug, Deserialize)]
pub struct RabbitmqConfig {
    pub addr: String,
    pub queue: String,
}
