use crate::database::MusicDb;

pub trait TracksDb {
    async fn get_full_track_info(&self, track_id: i64) -> Result<FullTrackInfo, sqlx::Error>;
    async fn get_short_track_info(&self, track_id: i64) -> Result<ShortTrackInfo, sqlx::Error>;
    async fn save_track_info(&self, track_info: &UploadTrackInfo) -> Result<i64, sqlx::Error>;
    async fn update_track_info(&self, track_id: i64, track_info: &UpdateTrackInfo) -> Result<(), sqlx::Error>;
    async fn update_track_thumbnail(&self, track_id: i64, thumbnail_url: &str) -> Result<(), sqlx::Error>;
    async fn remove_track_info(&self, track_id: i64) -> Result<(), sqlx::Error>;
    async fn link_track_audio(&self, track_id: i64, link_info: &LinkAudioInfo) -> Result<(), sqlx::Error>;
    async fn register_play(&self, track_id: i64) -> Result<String, sqlx::Error>;
}

impl TracksDb for MusicDb {
    async fn get_full_track_info(&self, track_id: i64) -> Result<FullTrackInfo, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let track_info = sqlx::query!(r"
            SELECT
                T.id,
                name,
                description,
                thumbnail_url,
                duration_seconds,
                play_count,
                T.user_id
            FROM tracks T
            WHERE T.id = $1;",
            track_id,
        )
            .fetch_one(&mut *tx)
            .await?;
        
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
        
        tx.commit().await?;

        let res = FullTrackInfo {
            id: track_info.id,
            name: track_info.name,
            description: track_info.description,
            thumbnail_url: track_info.thumbnail_url,
            duration_seconds: track_info.duration_seconds,
            play_count: track_info.play_count,
            user_id: track_info.user_id,
            categories: track_categories,
        };
        Ok(res)
    }

    async fn get_short_track_info(&self, track_id: i64) -> Result<ShortTrackInfo, sqlx::Error> {
        let res = sqlx::query_as!(ShortTrackInfo, r"
            SELECT
                id, name, thumbnail_url, duration_seconds, play_count, user_id
            FROM tracks
            WHERE id = $1;",
            track_id
        )
            .fetch_one(&self.pool)
            .await?;
        Ok(res)
    }

    async fn save_track_info(&self, track_info: &UploadTrackInfo) -> Result<i64, sqlx::Error> {
        let id: i64 = sqlx::query_scalar!(r"
            INSERT INTO tracks (
                name, description, user_id
            ) VALUES ($1, $2, $3)
            RETURNING id;",
            &track_info.name,
            track_info.description,
            track_info.user_id,
        )
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    async fn update_track_info(&self, track_id: i64, track_info: &UpdateTrackInfo) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(r"
            UPDATE tracks SET name = $1, description = $2
            WHERE id = $3;",
            &track_info.name,
            track_info.description,
            track_id,
        )
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn update_track_thumbnail(&self, track_id: i64, thumbnail_url: &str) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(r"
            UPDATE tracks SET thumbnail_url = $1
            WHERE id = $2;",
            thumbnail_url,
            track_id,
        )
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn remove_track_info(&self, track_id: i64) -> Result<(), sqlx::Error> {
        let res = sqlx::query!("DELETE FROM tracks WHERE id = $1;", track_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn link_track_audio(&self, track_id: i64, link_info: &LinkAudioInfo) -> Result<(), sqlx::Error> {
        sqlx::query!(r"
            UPDATE tracks SET audio_uri = $1, duration_seconds = $2
            WHERE id = $3;",
            &link_info.audio_uri,
            link_info.duration_seconds,
            track_id,
        )
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn register_play(&self, track_id: i64) -> Result<String, sqlx::Error> {
        let audio_uri = sqlx::query_scalar!(r"
            UPDATE tracks SET play_count = play_count + 1
            WHERE id = $1
            RETURNING audio_uri;",
            track_id,
        )
            .fetch_one(&self.pool)
            .await?;
        match audio_uri {
            Some(uri) => Ok(uri),
            None => Err(sqlx::Error::RowNotFound),
        }
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
    pub user_id: i64,
    pub categories: Vec<CategoryInfo>,
}

#[derive(sqlx::FromRow, Debug)]
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

#[derive(sqlx::FromRow, Debug)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
}

pub struct LinkAudioInfo {
    pub audio_uri: String,
    pub duration_seconds: i64,
}