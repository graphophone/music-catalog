use sqlx::PgConnection;

use crate::database::MusicDb;

pub trait CategoriesDb {
    async fn get_categories_for_track(&self, track_id: i64) -> Result<Vec<CategoryInfo>, sqlx::Error>;
    async fn create_category(&self, category_name: String) -> Result<i64, sqlx::Error>;
    async fn update_category(&self, category_id: i64, new_category_name: String) -> Result<(), sqlx::Error>;
    async fn get_categories(&self, page_number: i32, page_size: i32) -> Result<Vec<CategoryInfo>, sqlx::Error>;
    async fn link_track_categories_with_transaction(tx: &mut PgConnection, track_id: i64, categories_ids: &[i64]) -> Result<(), sqlx::Error>;
}

impl CategoriesDb for MusicDb {
    async fn get_categories_for_track(&self, track_id: i64) -> Result<Vec<CategoryInfo>, sqlx::Error> {
        let categories = sqlx::query_as!(CategoryInfo, r"
            SELECT c.*
            FROM categories c
            JOIN tracks_categories tc
            ON tc.category_id = c.id
            WHERE tc.track_id = $1;
        ", track_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(categories)
    }

    async fn create_category(&self, category_name: String) -> Result<i64, sqlx::Error> {
        let id = sqlx::query_scalar!(r"
            INSERT INTO categories (name)
            VALUES ($1)
            RETURNING id;
        ", category_name)
            .fetch_one(&self.pool)
            .await?;
        Ok(id)
    }

    async fn update_category(&self, category_id: i64, new_category_name: String) -> Result<(), sqlx::Error> {
        let res = sqlx::query!(r"
            UPDATE categories SET name = $1
            WHERE id = $2;
        ", new_category_name, category_id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound)
        }
        Ok(())
    }

    async fn get_categories(&self, page_number: i32, page_size: i32) -> Result<Vec<CategoryInfo>, sqlx::Error> {
        let offset = (page_number - 1) * page_size;
        let categories = sqlx::query_as!(CategoryInfo, r"
            SELECT *
            FROM categories
            ORDER BY id
            LIMIT $1 OFFSET $2;
        ", page_size as i64, offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(categories)
    }

    async fn link_track_categories_with_transaction(tx: &mut PgConnection, track_id: i64, categories_ids: &[i64]) -> Result<(), sqlx::Error> {
        sqlx::query!(r"
            INSERT INTO tracks_categories (track_id, category_id)
            SELECT $1, *
            FROM (
                SELECT * FROM UNNEST($2::bigint[])
            )
            ON CONFLICT DO NOTHING;
        ", track_id, &categories_ids)
            .execute(tx)
            .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow, Debug, PartialEq)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
}
