use sqlx::types::time;

use crate::database::{
    CatalogDb,
    categories::{CategoriesDb, CategoryData},
};

pub trait MusicDb {
    async fn get_full_music(&self, music_id: i64) -> Result<FullMusic, sqlx::Error>;
    async fn save_music(&self, music: &UploadMusic) -> Result<i64, sqlx::Error>;
    async fn update_music(&self, music_id: i64, data: &UpdateMusic) -> Result<(), sqlx::Error>;
    async fn update_thumbnail(
        &self,
        music_id: i64,
        data: &UpdateThumbnail,
    ) -> Result<Option<String>, sqlx::Error>;
    async fn remove_music(
        &self,
        music_id: i64,
        uploader_id: i64,
    ) -> Result<MusicAssets, sqlx::Error>;
    async fn link_music_audio(&self, music_id: i64, link: &LinkAudio) -> Result<(), sqlx::Error>;
    async fn register_play(&self, music_id: i64) -> Result<String, sqlx::Error>;
    async fn update_upload_status(
        &self,
        music_id: i64,
        data: &UpdateUploadStatus,
    ) -> Result<(), sqlx::Error>;
}

impl MusicDb for CatalogDb {
    async fn get_full_music(&self, music_id: i64) -> Result<FullMusic, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let music = sqlx::query!(
            r"
            SELECT
                id,
                title,
                description,
                thumbnail_id,
                duration_seconds,
                play_count,
                COUNT(user_id) like_count,
                upload_status,
                uploader_id,
                created_at
            FROM music T
            LEFT JOIN music_likes TL
            ON T.id = TL.music_id
            WHERE T.id = $1
            GROUP BY id,
                title,
                description,
                thumbnail_id,
                duration_seconds,
                play_count,
                uploader_id,
                upload_status;",
            music_id,
        )
        .fetch_one(&mut *tx)
        .await?;

        let music_categories =
            CatalogDb::get_categories_for_music_with_transaction(&mut *tx, music_id).await?;

        tx.commit().await?;

        let res = FullMusic {
            id: music.id,
            title: music.title,
            description: music.description,
            thumbnail_id: music.thumbnail_id,
            duration_seconds: music.duration_seconds,
            play_count: music.play_count,
            like_count: music.like_count.unwrap_or_default(),
            uploader_id: music.uploader_id,
            categories: music_categories,
            upload_status: music.upload_status,
            created_at: music.created_at,
        };
        Ok(res)
    }

    async fn save_music(&self, music: &UploadMusic) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let id: i64 = sqlx::query_scalar!(
            r"
            INSERT INTO music (
                title, description, uploader_id
            ) VALUES ($1, $2, $3)
            RETURNING id;",
            &music.title,
            music.description,
            music.uploader_id,
        )
        .fetch_one(&mut *tx)
        .await?;

        Self::link_music_categories_with_transaction(&mut *tx, id, &music.categories_ids).await?;

        tx.commit().await?;
        Ok(id)
    }

    async fn update_music(&self, music_id: i64, data: &UpdateMusic) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let res = sqlx::query!(
            r"
            UPDATE music SET title = $1, description = $2
            WHERE id = $3 AND uploader_id = $4;",
            &data.title,
            data.description,
            music_id,
            data.uploader_id,
        )
        .execute(&mut *tx)
        .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Self::link_music_categories_with_transaction(&mut *tx, music_id, &data.categories_ids)
            .await?;

        Self::unlink_music_categories_with_transaction(&mut *tx, music_id, &data.categories_ids)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_thumbnail(
        &self,
        music_id: i64,
        data: &UpdateThumbnail,
    ) -> Result<Option<String>, sqlx::Error> {
        let old_thumbnail_id = sqlx::query_scalar!(
            r"
            UPDATE music SET thumbnail_id = $1
            WHERE id = $2 AND uploader_id = $3
            RETURNING OLD.thumbnail_id;",
            data.thumbnail_id,
            music_id,
            data.uploader_id,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(old_thumbnail_id)
    }

    async fn remove_music(
        &self,
        music_id: i64,
        uploader_id: i64,
    ) -> Result<MusicAssets, sqlx::Error> {
        let res = sqlx::query!(
            r"
            WITH ctx AS (
                DELETE FROM music
                WHERE id = $1 AND uploader_id = $2
                RETURNING id, thumbnail_id
            )
            SELECT thumbnail_id, audio_uri
            FROM music_audio
            RIGHT JOIN ctx
            ON music_id = id",
            music_id,
            uploader_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        if let Some(rec) = res {
            return Ok(MusicAssets {
                thumbnail_id: rec.thumbnail_id,
                audio_uri: rec.audio_uri,
            });
        } else {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    async fn link_music_audio(&self, music_id: i64, link: &LinkAudio) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let res1 = sqlx::query!(
            r"
            WITH ctx AS (
                SELECT id
                FROM music
                WHERE id = $1 AND uploader_id = $2
            )
            INSERT INTO music_audio (music_id, audio_uri, duration_seconds)
            SELECT id, $3, $4
            FROM ctx",
            music_id,
            link.uploader_id,
            &link.audio_uri,
            link.duration_seconds,
        )
        .execute(&mut *tx)
        .await?;

        if res1.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        sqlx::query!(
            r"
            UPDATE music SET upload_status = 'uploaded'
            WHERE id = $1;",
            music_id,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn register_play(&self, music_id: i64) -> Result<String, sqlx::Error> {
        let audio_uri = sqlx::query_scalar!(
            r"
            UPDATE music SET play_count = play_count + 1
            FROM music_audio
            WHERE id = $1 AND music_id = id
            RETURNING audio_uri;",
            music_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        match audio_uri {
            Some(uri) => Ok(uri),
            None => Err(sqlx::Error::RowNotFound),
        }
    }

    async fn update_upload_status(
        &self,
        music_id: i64,
        data: &UpdateUploadStatus,
    ) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(
            r"
            UPDATE music SET upload_status = $3
            WHERE id = $1 AND uploader_id = $2;",
            music_id,
            data.uploader_id,
            data.new_status,
        )
        .execute(&self.pool)
        .await?;

        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct FullMusic {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_id: Option<String>,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
    pub like_count: i64,
    pub uploader_id: i64,
    pub categories: Vec<CategoryData>,
    pub upload_status: String,
    pub created_at: time::OffsetDateTime,
}

pub struct MusicAssets {
    pub thumbnail_id: Option<String>,
    pub audio_uri: Option<String>,
}

pub struct UploadMusic {
    pub title: String,
    pub description: Option<String>,
    pub categories_ids: Vec<i64>,
    pub uploader_id: i64,
}

pub struct UpdateMusic {
    pub title: String,
    pub description: Option<String>,
    pub categories_ids: Vec<i64>,
    pub uploader_id: i64,
}

pub struct UpdateThumbnail {
    pub thumbnail_id: Option<String>,
    pub uploader_id: i64,
}

pub struct UpdateUploadStatus {
    pub new_status: String,
    pub uploader_id: i64,
}

pub struct LinkAudio {
    pub audio_uri: String,
    pub duration_seconds: i64,
    pub uploader_id: i64,
}

#[cfg(test)]
mod tests {
    use crate::database::CatalogDb;

    use super::*;
    use anyhow::Result;

    #[sqlx::test]
    async fn test_save_music(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let c1 = db.create_category("Category 1".to_string()).await?;
        let c2 = db.create_category("Category 2".to_string()).await?;
        let c3 = db.create_category("Category 3".to_string()).await?;
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![c1.id, c2.id, c3.id],
        };

        db.save_music(&music_data1).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_full_music(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_music(&music_data1).await?;
        let full_data = db.get_full_music(id1).await?;

        assert_eq!(
            full_data,
            FullMusic {
                id: id1,
                title: music_data1.title,
                thumbnail_id: None,
                duration_seconds: None,
                play_count: 0,
                like_count: 0,
                uploader_id: music_data1.uploader_id,
                description: music_data1.description,
                categories: vec![],
                upload_status: "audio not uploaded".to_string(),
                created_at: full_data.created_at,
            }
        );
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_music(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let c1 = db.create_category("Category 1".to_string()).await?;
        let c2 = db.create_category("Category 2".to_string()).await?;
        let c3 = db.create_category("Category 3".to_string()).await?;
        let c4 = db.create_category("Category 4".to_string()).await?;
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![c1.id, c2.id, c3.id],
        };
        let update_music1 = UpdateMusic {
            title: "new title 1".to_string(),
            description: Some("new description".to_string()),
            categories_ids: vec![c2.id, c4.id],
            uploader_id: 1,
        };
        let thumbnail_id = "test url".to_string();

        let id1 = db.save_music(&music_data1).await?;
        db.update_music(id1, &update_music1).await?;
        db.update_thumbnail(
            id1,
            &UpdateThumbnail {
                thumbnail_id: Some(thumbnail_id.clone()),
                uploader_id: 1,
            },
        )
        .await?;
        let full_data = db.get_full_music(id1).await?;

        assert_eq!(
            full_data,
            FullMusic {
                id: id1,
                title: update_music1.title,
                thumbnail_id: Some(thumbnail_id),
                duration_seconds: None,
                play_count: 0,
                like_count: 0,
                uploader_id: music_data1.uploader_id,
                description: update_music1.description,
                categories: vec![c2, c4],
                upload_status: "audio not uploaded".to_string(),
                created_at: full_data.created_at,
            }
        );
        Ok(())
    }

    #[sqlx::test]
    async fn test_remove_music(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_music(&music_data1).await?;
        db.remove_music(id1, 1).await?;
        let full_data = db.get_full_music(id1).await;

        match full_data {
            Ok(_) => panic!("music was not deleted"),
            Err(_) => Ok(()),
        }
    }

    #[sqlx::test]
    async fn test_link_audio(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let audio_ = LinkAudio {
            audio_uri: "some uri".to_string(),
            duration_seconds: 123,
            uploader_id: 1,
        };

        let id1 = db.save_music(&music_data1).await?;
        db.link_music_audio(id1, &audio_).await?;
        let full_data = db.get_full_music(id1).await?;

        assert_eq!(
            full_data,
            FullMusic {
                id: id1,
                title: music_data1.title,
                thumbnail_id: None,
                duration_seconds: Some(audio_.duration_seconds),
                play_count: 0,
                like_count: 0,
                uploader_id: music_data1.uploader_id,
                description: music_data1.description,
                categories: vec![],
                upload_status: "audio not uploaded".to_string(),
                created_at: full_data.created_at,
            }
        );
        Ok(())
    }

    #[sqlx::test]
    async fn test_register_play(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let audio_ = LinkAudio {
            audio_uri: "some uri".to_string(),
            duration_seconds: 123,
            uploader_id: 1,
        };

        let id1 = db.save_music(&music_data1).await?;
        db.link_music_audio(id1, &audio_).await?;
        let audio_uri = db.register_play(id1).await?;

        assert_eq!(audio_uri, audio_.audio_uri);
        Ok(())
    }

    #[sqlx::test]
    async fn test_register_play_without_audio(pool: sqlx::PgPool) -> Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };

        let id1 = db.save_music(&music_data1).await?;
        match db.register_play(id1).await {
            Ok(_) => panic!("play was registered when audio was not linked"),
            Err(_) => Ok(()),
        }
    }
}
