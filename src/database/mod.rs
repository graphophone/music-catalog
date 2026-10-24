use anyhow::Result;

use crate::config::Config;

pub mod categories;
pub mod likes;
pub mod music;

#[derive(Clone)]
pub struct CatalogDb {
    pub pool: sqlx::PgPool,
}

impl CatalogDb {
    pub async fn build(conf: &Config) -> Result<CatalogDb> {
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
        Ok(CatalogDb { pool })
    }
}
