use sqlx::prelude::FromRow;

pub struct TracksDb {
    pool: sqlx::postgres::PgPool,
}

impl TracksDb {
    pub async fn build(conf: &crate::config::Config) -> Result<TracksDb, sqlx::Error> {
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
        Ok(TracksDb { pool })
    }

    pub async fn get_full_track_info(&self, track_id: i32) -> Result<Option<FullTrackInfo>, sqlx::Error> {
        let query = r"
            SELECT
                T.id,
                name,
                description,
                thumbnail_url,
                duration_seconds,
                play_count,
                T.user_id,
                COUNT(L.user_id) like_count
            FROM tracks T
            LEFT JOIN likes L
            ON T.id = L.track_id
            WHERE T.id = $1
            GROUP BY
                T.id,
                name,
                description,
                thumbnail_url,
                duration_seconds,
                play_count,
                T.user_id;
        ";
        let query = sqlx::query_as::<_, FullTrackInfo>(query)
            .bind(track_id);
        let res = query.fetch_optional(&self.pool).await?;
        Ok(res)
    }

    pub async fn get_short_track_info(&self, track_id: i32) -> Result<Option<ShortTrackInfo>, sqlx::Error> {
        let query = r"
            SELECT
                id, name, thumbnail_url, duration_seconds, play_count, user_id
            FROM tracks
            WHERE id = $1;
        ";
        let query = sqlx::query_as::<_, ShortTrackInfo>(query)
            .bind(track_id);
        let res = query.fetch_optional(&self.pool).await?;
        Ok(res)
    }

    pub async fn save_track_info(&self, track_info: &UploadTrackInfo) -> Result<i32, sqlx::Error> {
        let query = r"
            INSERT INTO tracks (
                name, description, user_id
            ) VALUES ($1, $2, $3)
            RETURNING id;
        ";
        let id: i32 = sqlx::query_scalar(query)
            .bind(&track_info.name)
            .bind(&track_info.description)
            .bind(track_info.user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn update_track_info(&self, track_id: i32, track_info: &UpdateTrackInfo) -> Result<(), sqlx::Error> {
        let query = r"
            UPDATE tracks SET name = $1, description = $2
            WHERE id = $3;
        ";
        let res = sqlx::query(query)
            .bind(&track_info.name)
            .bind(&track_info.description)
            .bind(track_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    pub async fn update_track_thumbnail(&self, track_id: i32, thumbnail_url: &str) -> Result<(), sqlx::Error> {
        let query = r"
            UPDATE tracks SET thumbnail_url = $1
            WHERE id = $2;
        ";
        let res = sqlx::query(query)
            .bind(thumbnail_url)
            .bind(track_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    pub async fn remove_track(&self, track_id: i32) -> Result<(), sqlx::Error> {
        let query = "DELETE FROM tracks WHERE id = $1;";
        let res = sqlx::query(query)
            .bind(track_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    pub async fn link_track_to_audio(&self, track_id: i32, link_info: &LinkAudioInfo) -> Result<(), sqlx::Error> {
        let query = r"
            UPDATE tracks SET audio_uri = $1, duration_seconds = $2
            WHERE id = $3'
        ";
        sqlx::query(query)
            .bind(&link_info.audio_uri)
            .bind(link_info.duration_seconds)
            .bind(track_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn register_play(&self, track_id: i32) -> Result<String, sqlx::Error> {
        let query = r"
            UPDATE tracks SET play_count = play_count + 1
            WHERE id = $1
            RETURNING audio_uri;
        ";
        let audio_uri: String = sqlx::query_scalar(query)
            .bind(track_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(audio_uri)
    }
}

#[derive(FromRow, Debug)]
pub struct FullTrackInfo {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i32>,
    pub play_count: i64,
    pub like_count: i64,
    pub user_id: i32,
    // pub categories: Vec<String>,
}

#[derive(FromRow, Debug)]
pub struct ShortTrackInfo {
    pub id: i32,
    pub name: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i32>,
    pub play_count: i64,
    pub user_id: i32,
}

pub struct UploadTrackInfo {
    pub name: String,
    pub description: Option<String>,
    pub user_id: i32,
}

pub struct UpdateTrackInfo {
    pub name: String,
    pub description: Option<String>,
    // pub category_ids: Vec<i32>,
}

pub struct LinkAudioInfo {
    pub audio_uri: String,
    pub duration_seconds: i32,
}