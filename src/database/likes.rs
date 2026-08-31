use crate::database::MusicDb;

pub trait LikesDb {
    async fn like_track(&self, user_id: i64, track_id: i64) -> Result<(), sqlx::Error>;
    async fn get_liked_tracks_for_user(&self, user_id: i64, page_number: i32, page_size: i32) -> Result<Vec<LikedTrackInfo>, sqlx::Error>;
    async fn get_liked_tracks_count_for_user(&self, user_id: i64) -> Result<i64, sqlx::Error>;
}

impl LikesDb for MusicDb {
    async fn like_track(&self, user_id: i64, track_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO track_likes (user_id, track_id) VALUES ($1, $2);", user_id, track_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_liked_tracks_for_user(&self, user_id: i64, page_number: i32, page_size: i32) -> Result<Vec<LikedTrackInfo>, sqlx::Error> {
        let offset: i32 = (page_number - 1) * page_size;
        let liked_tracks: Vec<LikedTrackInfo> = sqlx::query_as!(
            LikedTrackInfo, r"
                SELECT T.id, T.name, T.thumbnail_url, T.duration_seconds, T.play_count
                FROM track_likes TL
                JOIN tracks T
                ON TL.track_id = T.id
                WHERE TL.user_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3;
            ",
            user_id, page_size as i64, offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(liked_tracks)
    }

    async fn get_liked_tracks_count_for_user(&self, user_id: i64) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(r"
            SELECT COUNT(*)
            FROM track_likes
            WHERE user_id = $1;
        ", user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count.unwrap_or_default())
    }
}

pub struct LikedTrackInfo {
    pub id: i64,
    pub name: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<i64>,
    pub play_count: i64,
}

#[cfg(test)]
mod tests {
use crate::database::{likes::LikesDb, tracks::*};
use super::MusicDb;

    #[sqlx::test]
    async fn test_like_track(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: None,
            uploader_id: 1,
            categories_ids: vec![],
        };
        let track_id = db.save_track_info(&track_data1).await?;
        db.like_track(1, track_id).await?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_liked_tracks(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let track_data2 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let track1_id = db.save_track_info(&track_data1).await?;
        let track2_id = db.save_track_info(&track_data2).await?;

        db.like_track(1, track1_id).await?;
        db.like_track(1, track2_id).await?;
        let tracks = db.get_liked_tracks_for_user(1, 1, 10).await?;
        let liked_ids: Vec<i64> = tracks.iter().clone().map(|t| t.id).collect();
        assert_eq!(liked_ids, vec![2, 1]);
        let names_ids: Vec<String> = tracks.iter().clone().map(|t| t.name.clone()).collect();
        assert_eq!(names_ids, vec![track_data2.name, track_data1.name]);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_liked_tracks_with_pagination(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 2,
            categories_ids: vec![],
        };
        let track_data2 = UploadTrackInfo {
            name: "test song 2".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 3,
            categories_ids: vec![],
        };
        let track1_id = db.save_track_info(&track_data1).await?;
        let track2_id = db.save_track_info(&track_data2).await?;

        print!("Like tracks for user 1: {track1_id}, {track2_id}");
        db.like_track(1, track1_id).await?;
        db.like_track(1, track2_id).await?;
        let liked_tracks = db.get_liked_tracks_for_user(1, 1, 1).await?;
        let track2 = liked_tracks.get(0).unwrap();
        assert_eq!(track2.id, track2_id);
        assert_eq!(track2.name, track_data2.name);

        let liked_tracks = db.get_liked_tracks_for_user(1, 2, 1).await?;
        let track1 = liked_tracks.get(0).unwrap();
        assert_eq!(track1.id, track1_id);
        assert_eq!(track1.name, track_data1.name);

        let liked_tracks = db.get_liked_tracks_for_user(1, 3, 1).await?;
        assert_eq!(liked_tracks.len(), 0);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_user_liked_count(pool: sqlx::PgPool) -> anyhow::Result<()> {
        let db = MusicDb { pool };
        let track_data1 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let track_data2 = UploadTrackInfo {
            name: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![],
        };
        let track1_id = db.save_track_info(&track_data1).await?;
        let track2_id = db.save_track_info(&track_data2).await?;

        db.like_track(1, track1_id).await?;
        db.like_track(1, track2_id).await?;
        let count = db.get_liked_tracks_count_for_user(1).await?;
        assert_eq!(count, 2);
        Ok(())
    }
}