use sqlx::PgConnection;

use crate::database::MusicDb;

pub trait CategoriesDb {
    async fn get_categories_for_track_with_transaction(
        tx: &mut PgConnection,
        track_id: i64,
    ) -> Result<Vec<CategoryData>, sqlx::Error>;
    async fn create_category(&self, category_name: String) -> Result<CategoryData, sqlx::Error>;
    async fn get_categories(
        &self,
        search_token: &str,
        page_number: i32,
        page_size: i32,
    ) -> Result<Vec<CategoryData>, sqlx::Error>;
    async fn get_categories_count(&self) -> Result<i64, sqlx::Error>;
    async fn link_track_categories_with_transaction(
        tx: &mut PgConnection,
        track_id: i64,
        categories_ids: &[i64],
    ) -> Result<(), sqlx::Error>;
    async fn unlink_track_categories_with_transaction(
        tx: &mut PgConnection,
        track_id: i64,
        categories_ids: &[i64],
    ) -> Result<(), sqlx::Error>;
}

impl CategoriesDb for MusicDb {
    async fn get_categories_for_track_with_transaction(
        tx: &mut PgConnection,
        track_id: i64,
    ) -> Result<Vec<CategoryData>, sqlx::Error> {
        let categories = sqlx::query_as!(
            CategoryData,
            r"SELECT c.*
            FROM categories c
            JOIN tracks_categories tc
            ON tc.category_id = c.id
            WHERE tc.track_id = $1;",
            track_id
        ).fetch_all(&mut *tx)
            .await?;
        Ok(categories)
    }

    async fn create_category(&self, category_name: String) -> Result<CategoryData, sqlx::Error> {
        let category = sqlx::query_as!(
            CategoryData,
            r"INSERT INTO categories (name)
            VALUES ($1)
            RETURNING *;",
            category_name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(category)
    }

    async fn get_categories(
        &self,
        search_token: &str,
        page_number: i32,
        page_size: i32,
    ) -> Result<Vec<CategoryData>, sqlx::Error> {
        let offset = (page_number - 1) * page_size;
        let categories = sqlx::query_as!(
            CategoryData,
            r"SELECT *
            FROM categories
            WHERE name ILIKE CONCAT('%', $1::text, '%')
            ORDER BY id
            LIMIT $2 OFFSET $3;",
            search_token,
            page_size as i64,
            offset as i64
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(categories)
    }

    async fn link_track_categories_with_transaction(
        tx: &mut PgConnection,
        track_id: i64,
        categories_ids: &[i64],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(r"
            INSERT INTO tracks_categories (track_id, category_id)
            SELECT $1, *
            FROM (
                SELECT * FROM UNNEST($2::bigint[])
            )
            ON CONFLICT DO NOTHING;",
            track_id,
            &categories_ids
        )
        .execute(tx)
        .await?;
        Ok(())
    }

    async fn unlink_track_categories_with_transaction(
        tx: &mut PgConnection,
        track_id: i64,
        categories_ids: &[i64],
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(r"
            DELETE FROM tracks_categories 
            WHERE
                track_id = $1 AND
                category_id NOT IN (SELECT * FROM UNNEST($2::bigint[]));",
            track_id,
            &categories_ids
        )
        .execute(tx)
        .await?;
        Ok(())
    }

    async fn get_categories_count(&self) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar!(r"
            SELECT COUNT(*)
            FROM categories;"
        )
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(0);
        Ok(count)
    }
}

#[derive(sqlx::FromRow, Debug, PartialEq, Clone)]
pub struct CategoryData {
    pub id: i64,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::database::tracks::{TracksDb, UploadTrack};
    use anyhow::Result;
    use tokio::task::JoinSet;

    #[sqlx::test]
    async fn text_create_categories(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let c1_name = "Category 1".to_string();
        let c2_name = "Category 2".to_string();
        let c3_name = "Category 3".to_string();
        let c4_name = "Category 4".to_string();
        let (c1, c2, c3, c4) = tokio::try_join!(
            db.create_category(c1_name.clone()),
            db.create_category(c2_name.clone()),
            db.create_category(c3_name.clone()),
            db.create_category(c4_name.clone()),
        )?;

        assert_eq!(c1.name, c1_name);
        assert_eq!(c2.name, c2_name);
        assert_eq!(c3.name, c3_name);
        assert_eq!(c4.name, c4_name);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_categories(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let db_arc = Arc::new(db.clone());
        let mut futures = JoinSet::new();
        for i in 0..20 {
            let db_arc = Arc::clone(&db_arc);
            futures.spawn(async move { db_arc.create_category(format!("Category {i}")).await });
        }
        let mut categories = vec![];
        while let Some(res) = futures.join_next().await {
            let cat = res??;
            categories.push(cat);
        }

        categories.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        let page_size = 5;
        for page_number in 1..=((categories.len() as f64 / page_size as f64).ceil() as i32) {
            let page = db.get_categories("", page_number, page_size).await?;
            let offset = (page_number - 1) * page_size;
            let cats: Vec<CategoryData> = categories
                .iter()
                .skip(offset as usize)
                .take(page_size as usize)
                .map(|c| c.clone())
                .collect();
            assert_eq!(page, cats);
        }
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_categories_partial_page(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let db_arc = Arc::new(db.clone());
        let mut futures = JoinSet::new();
        for i in 0..2 {
            let db_arc = Arc::clone(&db_arc);
            futures.spawn(async move { db_arc.create_category(format!("Category {i}")).await });
        }
        let mut categories = vec![];
        while let Some(res) = futures.join_next().await {
            let cat = res??;
            categories.push(cat);
        }

        categories.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        let page_size = 5;
        for page_number in 1..=((categories.len() as f64 / page_size as f64).ceil() as i32) {
            let page = db.get_categories("", page_number, page_size).await?;
            let offset = (page_number - 1) * page_size;
            let cats: Vec<CategoryData> = categories
                .iter()
                .skip(offset as usize)
                .take(page_size as usize)
                .map(|c| c.clone())
                .collect();
            assert_eq!(page, cats);
        }
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_categories_empty_page(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let db_arc = Arc::new(db.clone());
        let mut futures = JoinSet::new();
        for i in 0..4 {
            let db_arc = Arc::clone(&db_arc);
            futures.spawn(async move { db_arc.create_category(format!("Category {i}")).await });
        }
        let mut categories = vec![];
        while let Some(res) = futures.join_next().await {
            let cat = res??;
            categories.push(cat);
        }

        categories.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        let page_size = 5;
        let page_outside_range = (categories.len() as f64 / page_size as f64).ceil() as i32 + 1;
        let page = db.get_categories("", page_outside_range, page_size).await?;
        assert_eq!(page, vec![]);
        Ok(())
    }

    #[sqlx::test]
    async fn test_link_track_categories_with_transaction(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let (c1, c2, c3) = tokio::try_join!(
            db.create_category("Category 1".to_string()),
            db.create_category("Category 2".to_string()),
            db.create_category("Category 3".to_string()),
        )?;

        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![c1.id, c2.id],
        };
        let t_id = db.save_track(&track_data1).await?;

        let mut tx = db.pool.begin().await?;
        let new_cats_ids: Vec<i64> = vec![c1.id, c3.id];
        MusicDb::link_track_categories_with_transaction(&mut tx, t_id, &new_cats_ids).await?;
        tx.commit().await?;

        let track_data = db.get_full_track(t_id).await?;
        let mut categories = track_data.categories;
        categories.sort_by_key(|c| c.id);
        let mut expected_cats = vec![c1, c2, c3];
        expected_cats.sort_by_key(|c| c.id);
        assert_eq!(categories, expected_cats);
        Ok(())
    }

    #[sqlx::test]
    async fn test_unlink_track_categories_with_transaction(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let (c1, c2, c3) = tokio::try_join!(
            db.create_category("Category 1".to_string()),
            db.create_category("Category 2".to_string()),
            db.create_category("Category 3".to_string()),
        )?;

        let track_data1 = UploadTrack {
            title: "test song 1".to_string(),
            description: Some("test description 1".to_string()),
            uploader_id: 1,
            categories_ids: vec![c1.id, c2.id],
        };
        let t_id = db.save_track(&track_data1).await?;

        let mut tx = db.pool.begin().await?;
        let new_cats_ids: Vec<i64> = vec![c1.id, c3.id];
        MusicDb::link_track_categories_with_transaction(&mut tx, t_id, &new_cats_ids).await?;

        MusicDb::unlink_track_categories_with_transaction(&mut tx, t_id, &new_cats_ids).await?;

        tx.commit().await?;

        let track_data = db.get_full_track(t_id).await?;
        let mut categories = track_data.categories;
        categories.sort_by_key(|c| c.id);
        let mut expected_cats = vec![c1, c3];
        expected_cats.sort_by_key(|c| c.id);
        assert_eq!(categories, expected_cats);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_categories_count(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        tokio::try_join!(
            db.create_category("Category 1".to_string()),
            db.create_category("Category 2".to_string()),
            db.create_category("Category 3".to_string()),
        )?;

        let count = db.get_categories_count().await?;
        assert_eq!(count, 3);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_categories_count_with_duplicate(pool: sqlx::PgPool) -> Result<()> {
        let db = MusicDb { pool };
        let _ = tokio::try_join!(
            db.create_category("Category 1".to_string()),
            db.create_category("Category 2".to_string()),
            db.create_category("Category 2".to_string()),
        );

        let count = db.get_categories_count().await?;
        assert_eq!(count, 2);
        Ok(())
    }
}
