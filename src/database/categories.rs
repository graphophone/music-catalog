pub trait CategoriesDb {
    async fn get_categories_for_track(&self, track_id: i64) -> Result<Vec<CategoryInfo>, sqlx::Error>;
    async fn create_category(&self, category_name: String) -> Result<(), sqlx::Error>;
    async fn update_category(&self, track_id: i64, new_category_name: String) -> Result<(), sqlx::Error>;
    async fn get_categories(&self, page_number: i32, page_size: i32) -> Result<Vec<CategoryInfo>, sqlx::Error>;
    async fn link_track_categories(&self, track_id: i64, categories_ids: Vec<i64>) -> Result<(), sqlx::Error>;
}

#[derive(sqlx::FromRow, Debug, PartialEq)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
}
