use std::sync::Arc;

use crate::{
    database::{MusicDb, categories::CategoriesDb},
    services::categories_service::categories::{
        CategoriesPage, CategoryData, CreateCategoryReq, GetCategoriesReq,
    },
};
use categories::categories_server::Categories;
pub use categories::categories_server::CategoriesServer;
use tonic::{Request, Response, Status, async_trait};

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
    async fn create_category(
        &self,
        req: Request<CreateCategoryReq>,
    ) -> Result<Response<CategoryData>, Status> {
        let req = req.into_inner();
        let res = self.music_db.create_category(req.name).await;
        let category = match res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };
        Ok(Response::new(CategoryData {
            id: category.id,
            name: category.name,
        }))
    }

    async fn get_categories(
        &self,
        req: Request<GetCategoriesReq>,
    ) -> Result<Response<CategoriesPage>, Status> {
        let req = req.into_inner();
        let categories_fut =
            self.music_db
                .get_categories(&req.search_token, req.page_number, req.page_size);
        let count_fut = self.music_db.get_categories_count();

        let futs_res = tokio::try_join!(categories_fut, count_fut,);

        let (categories, total_count) = match futs_res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };
        let categories = categories
            .into_iter()
            .map(|c| CategoryData {
                id: c.id,
                name: c.name,
            })
            .collect();

        Ok(Response::new(CategoriesPage {
            categories,
            total_count,
        }))
    }
}
