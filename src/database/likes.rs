use crate::database::CatalogDb;

pub trait LikesDb {
    async fn like_music(&self, user_id: i64, music_id: i64) -> Result<(), sqlx::Error>;
    async fn unlike_music(&self, user_id: i64, music_id: i64) -> Result<(), sqlx::Error>;
    async fn get_liked_music_for_user(
        &self,
        user_id: i64,
        page_number: i32,
        page_size: i32,
    ) -> Result<Vec<LikedMusic>, sqlx::Error>;
    async fn get_liked_music_count_for_user(&self, user_id: i64) -> Result<i64, sqlx::Error>;
}

impl LikesDb for CatalogDb {
    async fn like_music(&self, user_id: i64, music_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO music_likes (user_id, music_id) VALUES ($1, $2);",
            user_id,
            music_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn unlike_music(&self, user_id: i64, music_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM music_likes WHERE user_id = $1 AND music_id = $2;",
            user_id,
            music_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_liked_music_for_user(
        &self,
        user_id: i64,
        page_number: i32,
        page_size: i32,
    ) -> Result<Vec<LikedMusic>, sqlx::Error> {
        let offset: i32 = (page_number - 1) * page_size;
        let liked_music: Vec<LikedMusic> = sqlx::query_as!(
            LikedMusic,
            r"
                SELECT T.id, T.title, T.thumbnail_id, TA.duration_seconds, T.play_count
                FROM music_likes TL
                JOIN music T
                ON TL.music_id = T.id
                JOIN music_audio TA
                ON T.id = TA.music_id
                WHERE TL.user_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3;
            ",
            user_id,
            page_size as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(liked_music)
    }

    async fn get_liked_music_count_for_user(&self, user_id: i64) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(
            r"
            SELECT COUNT(*)
            FROM music_likes
            WHERE user_id = $1;
        ",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count.unwrap_or_default())
    }
}

pub struct LikedMusic {
    pub id: i64,
    pub title: String,
    pub thumbnail_id: Option<String>,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
}

#[cfg(test)]
mod tests {
    use super::CatalogDb;
    use crate::database::{likes::LikesDb, music::*};

    #[sqlx::test]
    async fn test_like_music(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music_id = db.save_music(&music_data1).await?;
        db.like_music(1, music_id).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_unlike_music(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music_id = db.save_music(&music_data1).await?;
        db.like_music(1, music_id).await?;
        db.unlike_music(1, music_id).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_liked_music(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music_data2 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music1_id = db.save_music(&music_data1).await?;
        let music2_id = db.save_music(&music_data2).await?;

        db.like_music(1, music1_id).await?;
        db.like_music(1, music2_id).await?;
        let music = db.get_liked_music_for_user(1, 1, 10).await?;
        let liked_ids: Vec<i64> = music.iter().clone().map(|t| t.id).collect();
        assert_eq!(liked_ids, vec![2, 1]);
        let titles_ids: Vec<String> = music.iter().clone().map(|t| t.title.clone()).collect();
        assert_eq!(titles_ids, vec![music_data2.title, music_data1.title]);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_liked_music_with_pagination(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 2,
            categories_ids: vec![],
        };
        let music_data2 = UploadMusic {
            title: "test song 2".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 3,
            categories_ids: vec![],
        };
        let music1_id = db.save_music(&music_data1).await?;
        let music2_id = db.save_music(&music_data2).await?;

        print!("Like music for user 1: {music1_id}, {music2_id}");
        db.like_music(1, music1_id).await?;
        db.like_music(1, music2_id).await?;
        let liked_music = db.get_liked_music_for_user(1, 1, 1).await?;
        let music2 = liked_music.get(0).unwrap();
        assert_eq!(music2.id, music2_id);
        assert_eq!(music2.title, music_data2.title);

        let liked_music = db.get_liked_music_for_user(1, 2, 1).await?;
        let music1 = liked_music.get(0).unwrap();
        assert_eq!(music1.id, music1_id);
        assert_eq!(music1.title, music_data1.title);

        let liked_music = db.get_liked_music_for_user(1, 3, 1).await?;
        assert_eq!(liked_music.len(), 0);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_user_liked_count(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music_data2 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music1_id = db.save_music(&music_data1).await?;
        let music2_id = db.save_music(&music_data2).await?;

        db.like_music(1, music1_id).await?;
        db.like_music(1, music2_id).await?;
        let count = db.get_liked_music_count_for_user(1).await?;
        assert_eq!(count, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_user_liked_count_after_unlike(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = CatalogDb { pool };
        let music_data1 = UploadMusic {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let music1_id = db.save_music(&music_data1).await?;

        db.like_music(1, music1_id).await?;
        let count = db.get_liked_music_count_for_user(1).await?;
        assert_eq!(count, 1);
        db.unlike_music(1, music1_id).await?;
        let count = db.get_liked_music_count_for_user(1).await?;
        assert_eq!(count, 0);
        Ok(())
    }
}
