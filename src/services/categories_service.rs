use std::sync::Arc;

use tonic::{Request, Response, Status, async_trait};
use categories::categories_server::Categories;
use crate::{database::{MusicDb, categories::CategoriesDb}, services::categories_service::categories::{CategoriesPage, CategoryInfo, CreateCategoryRequest, Empty, GetCategoriesRequest, UpdateCategoryRequest}};
pub use categories::categories_server::CategoriesServer;

pub mod categories {
    tonic::include_proto!("categories");
}

pub struct CategoriesService {
    music_db: Arc<MusicDb>,
}

impl CategoriesService {
    pub fn build(music_db: Arc<MusicDb>) -> CategoriesService {
        CategoriesService { music_db }
    }
}

#[async_trait]
impl Categories for CategoriesService {
    async fn create_category(&self, req: Request<CreateCategoryRequest>) -> Result<Response<CategoryInfo>, Status> {
        let req = req.into_inner();
        let res = self.music_db
            .create_category(req.name)
            .await;
        let info = match res {
            Ok(info) => info,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };
        Ok(Response::new(CategoryInfo {
            id: info.id,
            name: info.name,
        }))
    }

    async fn update_category(&self, req: Request<UpdateCategoryRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        let res = self.music_db
            .update_category(req.category_id, req.new_name)
            .await;
        if let Err(e) = res {
            return Err(Status::from_error(Box::new(e)));
        }
        Ok(Response::new(Empty {}))
    }

    async fn get_categories(&self, req: Request<GetCategoriesRequest>) -> Result<Response<CategoriesPage>, Status> {
        let req = req.into_inner();
        let categories_fut = self.music_db
            .get_categories(req.page_number, req.page_size);
        let count_fut = self.music_db
            .get_categories_count();

        let futs_res = tokio::try_join!(
            categories_fut,
            count_fut,
        );

        let (categories, total_count) = match futs_res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };
        let categories = categories.into_iter()
            .map(|c| CategoryInfo { id: c.id, name: c.name })
            .collect();

        Ok(Response::new(CategoriesPage { categories, total_count }))
    }
}