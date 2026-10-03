use crate::database::{
    MusicDb,
    categories::{CategoriesDb, CategoryData},
};

pub trait TracksDb {
    async fn get_full_track(&self, track_id: i64) -> Result<FullTrack, sqlx::Error>;
    async fn save_track(&self, track_: &UploadTrack) -> Result<i64, sqlx::Error>;
    async fn update_track(
        &self,
        track_id: i64,
        track_: &UpdateTrack,
    ) -> Result<(), sqlx::Error>;
    async fn update_track_thumbnail(
        &self,
        track_id: i64,
        thumbnail_url: &str,
    ) -> Result<(), sqlx::Error>;
    async fn remove_track(&self, track_id: i64) -> Result<(), sqlx::Error>;
    async fn link_track_audio(
        &self,
        track_id: i64,
        link_: &LinkAudio,
    ) -> Result<(), sqlx::Error>;
    async fn register_play(&self, track_id: i64) -> Result<String, sqlx::Error>;
}

impl TracksDb for MusicDb {
    async fn get_full_track(&self, track_id: i64) -> Result<FullTrack, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let track = sqlx::query!(
            r"
            SELECT
                id,
                title,
                description,
                thumbnail_url,
                duration_seconds,
                play_count,
                COUNT(user_id) like_count,
                uploader_id
            FROM tracks T
            LEFT JOIN track_likes TL
            ON T.id = TL.track_id
            WHERE T.id = $1
            GROUP BY id,
                title,
                description,
                thumbnail_url,
                duration_seconds,
                play_count,
                uploader_id;",
            track_id,
        )
        .fetch_one(&mut *tx);
        let track_categories = self.get_categories_for_track(track_id);
        let (track, track_categories) = tokio::try_join!(track, track_categories)?;

        tx.commit().await?;

        let res = FullTrack {
            id: track.id,
            title: track.title,
            description: track.description,
            thumbnail_url: track.thumbnail_url,
            duration_seconds: track.duration_seconds,
            play_count: track.play_count,
            like_count: track.like_count.unwrap_or_default(),
            uploader_id: track.uploader_id,
            categories: track_categories,
        };
        Ok(res)
    }

    async fn save_track(&self, track: &UploadTrack) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let id: i64 = sqlx::query_scalar!(
            r"
            INSERT INTO tracks (
                title, description, uploader_id
            ) VALUES ($1, $2, $3)
            RETURNING id;",
            &track.title,
            track.description,
            track.uploader_id,
        )
        .fetch_one(&mut *tx)
        .await?;

        Self::link_track_categories_with_transaction(&mut *tx, id, &track.categories_ids)
            .await?;

        tx.commit().await?;
        Ok(id)
    }

    async fn update_track(
        &self,
        track_id: i64,
        track: &UpdateTrack,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let res = sqlx::query!(
            r"
            UPDATE tracks SET title = $1, description = $2
            WHERE id = $3;",
            &track.title,
            track.description,
            track_id,
        )
        .execute(&mut *tx)
        .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Self::link_track_categories_with_transaction(
            &mut *tx,
            track_id,
            &track.categories_ids,
        )
        .await?;

        Self::unlink_track_categories_with_transaction(
            &mut *tx,
            track_id,
            &track.categories_ids,
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_track_thumbnail(
        &self,
        track_id: i64,
        thumbnail_url: &str,
    ) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(
            r"
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

    async fn remove_track(&self, track_id: i64) -> Result<(), sqlx::Error> {
        let res = sqlx::query!("DELETE FROM tracks WHERE id = $1;", track_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn link_track_audio(
        &self,
        track_id: i64,
        link_: &LinkAudio,
    ) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(
            r"
            UPDATE tracks SET audio_uri = $1, duration_seconds = $2
            WHERE id = $3;",
            &link_.audio_uri,
            link_.duration_seconds,
            track_id,
        )
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    async fn register_play(&self, track_id: i64) -> Result<String, sqlx::Error> {
        let audio_uri = sqlx::query_scalar!(
            r"
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

#[derive(Debug, PartialEq)]
pub struct FullTrack {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
    pub like_count: i64,
    pub uploader_id: i64,
    pub categories: Vec<CategoryData>,
}

pub struct UploadTrack {
    pub title: String,
    pub description: Option<String>,
    pub categories_ids: Vec<i64>,
    pub uploader_id: i64,
}

pub struct UpdateTrack {
    pub title: String,
    pub description: Option<String>,
    pub categories_ids: Vec<i64>,
}

pub struct LinkAudio {
    pub audio_uri: String,
    pub duration_seconds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[sqlx::test]
    async fn test_save_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let c1 = db.create_category("Category 1".to_string()).await?;
        let c2 = db.create_category("Category 2".to_string()).await?;
        let c3 = db.create_category("Category 3".to_string()).await?;
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![c1.id, c2.id, c3.id],
        };

        db.save_track(&track_data1).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_full_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track(&track_data1).await?;
        let full_data = db.get_full_track(id1).await?;

        assert_eq!(
            full_data,
            FullTrack {
                id: id1,
                title: track_data1.title,
                thumbnail_url: None,
                duration_seconds: None,
                play_count: 0,
                like_count: 0,
                uploader_id: track_data1.uploader_id,
                description: track_data1.description,
                categories: vec![],
            }
        );
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let c1 = db.create_category("Category 1".to_string()).await?;
        let c2 = db.create_category("Category 2".to_string()).await?;
        let c3 = db.create_category("Category 3".to_string()).await?;
        let c4 = db.create_category("Category 4".to_string()).await?;
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![c1.id, c2.id, c3.id],
        };
        let update_track1 = UpdateTrack {
            title: "new title 1".to_string(),
            description: Some("new description".to_string()),
            categories_ids: vec![c2.id, c4.id],
        };
        let thumbnail_url = "test url";

        let id1 = db.save_track(&track_data1).await?;
        db.update_track(id1, &update_track1).await?;
        db.update_track_thumbnail(id1, thumbnail_url).await?;
        let full_data = db.get_full_track(id1).await?;

        assert_eq!(
            full_data,
            FullTrack {
                id: id1,
                title: update_track1.title,
                thumbnail_url: Some(thumbnail_url.to_string()),
                duration_seconds: None,
                play_count: 0,
                like_count: 0,
                uploader_id: track_data1.uploader_id,
                description: update_track1.description,
                categories: vec![c2, c4],
            }
        );
        Ok(())
    }

    #[sqlx::test]
    async fn test_remove_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track(&track_data1).await?;
        db.remove_track(id1).await?;
        let full_data = db.get_full_track(id1).await;

        match full_data {
            Ok(_) => panic!("track was not deleted"),
            Err(_) => Ok(()),
        }
    }

    #[sqlx::test]
    async fn test_link_audio(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let audio_ = LinkAudio {
            audio_uri: "some uri".to_string(),
            duration_seconds: 123,
        };

        let id1 = db.save_track(&track_data1).await?;
        db.link_track_audio(id1, &audio_).await?;
        let full_data = db.get_full_track(id1).await?;

        assert_eq!(
            full_data,
            FullTrack {
                id: id1,
                title: track_data1.title,
                thumbnail_url: None,
                duration_seconds: Some(audio_.duration_seconds),
                play_count: 0,
                like_count: 0,
                uploader_id: track_data1.uploader_id,
                description: track_data1.description,
                categories: vec![],
            }
        );
        Ok(())
    }

    #[sqlx::test]
    async fn test_register_play(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let audio_ = LinkAudio {
            audio_uri: "some uri".to_string(),
            duration_seconds: 123,
        };

        let id1 = db.save_track(&track_data1).await?;
        db.link_track_audio(id1, &audio_).await?;
        let audio_uri = db.register_play(id1).await?;

        assert_eq!(audio_uri, audio_.audio_uri);
        Ok(())
    }

    #[sqlx::test]
    async fn test_register_play_without_audio(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track(&track_data1).await?;
        match db.register_play(id1).await {
            Ok(_) => panic!("play was registered when audio was not linked"),
            Err(_) => Ok(()),
        }
    }
}
