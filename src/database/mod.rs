use anyhow::Result;

use crate::config::Config;

pub mod tracks;
pub mod categories;
pub mod likes;

#[derive(Clone)]
pub struct MusicDb {
    pub pool: sqlx::PgPool,
}

impl MusicDb {
    pub async fn build(conf: &Config) -> Result<MusicDb> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            &conf.postgres.user,
            &conf.postgres.password,
            &conf.postgres.host,
            &conf.postgres.port,
            &conf.postgres.database,
        );
        let pool = sqlx::postgres::PgPool::connect(&url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(MusicDb { pool })
    }
}