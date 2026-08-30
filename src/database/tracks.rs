use crate::database::{MusicDb, categories::{CategoriesDb, CategoryInfo}};

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
        
        let track_categories = self.get_categories_for_track(track_id)
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
        let mut tx = self.pool.begin().await?;
        
        let id: i64 = sqlx::query_scalar!(r"
            INSERT INTO tracks (
                name, description, user_id
            ) VALUES ($1, $2, $3)
            RETURNING id;",
            &track_info.name,
            track_info.description,
            track_info.user_id,
        )
            .fetch_one(&mut *tx)
            .await?;

        Self::link_track_categories_with_transaction(
            &mut *tx, id, &track_info.categories_ids,
        ).await?;

        tx.commit().await?;
        Ok(id)
    }

    async fn update_track_info(&self, track_id: i64, track_info: &UpdateTrackInfo) -> Result<(), sqlx::Error> {
    let mut tx = self.pool.begin().await?;
        
        let res = sqlx::query!(r"
            UPDATE tracks SET name = $1, description = $2
            WHERE id = $3;",
            &track_info.name,
            track_info.description,
            track_id,
        )
            .execute(&mut *tx)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Self::link_track_categories_with_transaction(
            &mut *tx, track_id, &track_info.categories_ids,
        ).await?;

        Self::unlink_track_categories_with_transaction(
            &mut *tx, track_id, &track_info.categories_ids,
        ).await?;

        tx.commit().await?;
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
        let res = sqlx::query!(r"
            UPDATE tracks SET audio_uri = $1, duration_seconds = $2
            WHERE id = $3;",
            &link_info.audio_uri,
            link_info.duration_seconds,
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

#[derive(Debug, PartialEq)]
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

#[derive(sqlx::FromRow, Debug, PartialEq)]
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
    pub categories_ids: Vec<i64>,
    pub user_id: i64,
}

pub struct UpdateTrackInfo {
    pub name: String,
    pub description: Option<String>,
    pub categories_ids: Vec<i64>,
}

pub struct LinkAudioInfo {
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
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: None,
            user_id: 1,
            categories_ids: vec![c1.id, c2.id, c3.id],
        };

        db.save_track_info(&track_data1).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_short_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            user_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track_info(&track_data1).await?;
        let short_data = db.get_short_track_info(id1).await?;

        assert_eq!(short_data, ShortTrackInfo {
            id: id1,
            name: track_data1.name,
            thumbnail_url: None,
            duration_seconds: None,
            play_count: 0,
            user_id: track_data1.user_id,
        });
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_full_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            user_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track_info(&track_data1).await?;
        let full_data = db.get_full_track_info(id1).await?;

        assert_eq!(full_data, FullTrackInfo {
            id: id1,
            name: track_data1.name,
            thumbnail_url: None,
            duration_seconds: None,
            play_count: 0,
            user_id: track_data1.user_id,
            description: track_data1.description,
            categories: vec![],
        });
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let c1 = db.create_category("Category 1".to_string()).await?;
        let c2 = db.create_category("Category 2".to_string()).await?;
        let c3 = db.create_category("Category 3".to_string()).await?;
        let c4 = db.create_category("Category 4".to_string()).await?;
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: None,
            user_id: 1,
            categories_ids: vec![c1.id, c2.id, c3.id],
        };
        let update_track1 = UpdateTrackInfo {
            name: "new name 1".to_string(),
            description: Some("new description".to_string()),
            categories_ids: vec![c2.id, c4.id],
        };
        let thumbnail_url = "test url";

        let id1 = db.save_track_info(&track_data1).await?;
        db.update_track_info(id1, &update_track1).await?;
        db.update_track_thumbnail(id1, thumbnail_url).await?;
        let full_data = db.get_full_track_info(id1).await?;

        assert_eq!(full_data, FullTrackInfo {
            id: id1,
            name: update_track1.name,
            thumbnail_url: Some(thumbnail_url.to_string()),
            duration_seconds: None,
            play_count: 0,
            user_id: track_data1.user_id,
            description: update_track1.description,
            categories: vec![c2, c4],
        });
        Ok(())
    }

    #[sqlx::test]
    async fn test_remove_track(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            user_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track_info(&track_data1).await?;
        db.remove_track_info(id1).await?;
        let full_data = db.get_full_track_info(id1).await;
        
        match full_data {
            Ok(_) => panic!("track was not deleted"),
            Err(_) => Ok(()),
        }
    }

    #[sqlx::test]
    async fn test_link_audio(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            user_id: 1,
            categories_ids: vec![],
        };
        let audio_info = LinkAudioInfo {
            audio_uri: "some uri".to_string(),
            duration_seconds: 123,
        };

        let id1 = db.save_track_info(&track_data1).await?;
        db.link_track_audio(id1, &audio_info).await?;
        let full_data = db.get_full_track_info(id1).await?;

        assert_eq!(full_data, FullTrackInfo {
            id: id1,
            name: track_data1.name,
            thumbnail_url: None,
            duration_seconds: Some(audio_info.duration_seconds),
            play_count: 0,
            user_id: track_data1.user_id,
            description: track_data1.description,
            categories: vec![],
        });
        Ok(())
    }

    #[sqlx::test]
    async fn test_register_play(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            user_id: 1,
            categories_ids: vec![],
        };
        let audio_info = LinkAudioInfo {
            audio_uri: "some uri".to_string(),
            duration_seconds: 123,
        };

        let id1 = db.save_track_info(&track_data1).await?;
        db.link_track_audio(id1, &audio_info).await?;
        let audio_uri = db.register_play(id1).await?;

        assert_eq!(audio_uri, audio_info.audio_uri);
        Ok(())
    }

    #[sqlx::test]
    async fn test_register_play_without_audio(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            user_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_track_info(&track_data1).await?;
        match db.register_play(id1).await {
            Ok(_) => panic!("play was registered when audio was not linked"),
            Err(_) => Ok(()),
        }
    }
}