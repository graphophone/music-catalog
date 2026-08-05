use sqlx::{Row, prelude::FromRow};

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

    pub async fn get_full_track_info(&self, track_id: i64) -> Result<Option<FullTrackInfo>, sqlx::Error> {
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
        let mut tx = self.pool.begin().await?;
        let track_info = sqlx::query(query)
            .bind(track_id)
            .fetch_optional(&mut *tx)
            .await?;
        let track_info = match track_info {
            Some(track_info) => track_info,
            None => return Ok(None),
        };

        let query = r"
            SELECT C.id, C.name
            FROM categories C
            JOIN tracks_categories TC
            ON C.id = TC.category_id
            WHERE TC.track_id = $1
        ";
        let track_categories: Vec<CategoryInfo> = sqlx::query_as::<_, CategoryInfo>(query)
            .bind(track_id)
            .fetch_all(&mut *tx)
            .await?;
        let res = FullTrackInfo {
            id: track_info.try_get("id")?,
            name: track_info.try_get("name")?,
            description: track_info.try_get("description")?,
            thumbnail_url: track_info.try_get("thumbnail_url")?,
            duration_seconds: track_info.try_get("duration_seconds")?,
            play_count: track_info.try_get("play_count")?,
            like_count: track_info.try_get("like_count")?,
            user_id: track_info.try_get("user_id")?,
            categories: track_categories,
        };
        tx.commit().await?;
        Ok(Some(res))
    }
    pub async fn get_short_track_info(&self, track_id: i64) -> Result<Option<ShortTrackInfo>, sqlx::Error> {
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

    pub async fn save_track_info(&self, track_info: &UploadTrackInfo) -> Result<i64, sqlx::Error> {
        let query = r"
            INSERT INTO tracks (
                name, description, user_id
            ) VALUES ($1, $2, $3)
            RETURNING id;
        ";
        let id: i64 = sqlx::query_scalar(query)
            .bind(&track_info.name)
            .bind(&track_info.description)
            .bind(track_info.user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    pub async fn update_track_info(&self, track_id: i64, track_info: &UpdateTrackInfo) -> Result<(), sqlx::Error> {
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

    pub async fn update_track_thumbnail(&self, track_id: i64, thumbnail_url: &str) -> Result<(), sqlx::Error> {
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

    pub async fn remove_track_info(&self, track_id: i64) -> Result<(), sqlx::Error> {
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

    pub async fn link_track_audio(&self, track_id: i64, link_info: &LinkAudioInfo) -> Result<(), sqlx::Error> {
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

    pub async fn register_play(&self, track_id: i64) -> Result<String, sqlx::Error> {
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

#[derive(Debug)]
pub struct FullTrackInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
    pub like_count: i64,
    pub user_id: i64,
    pub categories: Vec<CategoryInfo>,
}

#[derive(FromRow, Debug)]
pub struct ShortTrackInfo {
    pub id: i64,
    pub name: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
    pub user_id: i64,
}

pub struct UploadTrackInfo {
    pub name: String,
    pub description: Option<String>,
    pub user_id: i64,
}

pub struct UpdateTrackInfo {
    pub name: String,
    pub description: Option<String>,
    pub category_ids: Vec<i64>,
}

#[derive(FromRow, Debug)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
}

pub struct LinkAudioInfo {
    pub audio_uri: String,
    pub duration_seconds: i64,
}