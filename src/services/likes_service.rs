use std::sync::Arc;

use crate::{
    database::{CatalogDb, likes::LikesDb},
    services::likes_service::likes::{
        Empty, GetUserLikedRequest, LikeMusicRequest, LikedMusic, LikedPage, UnlikeMusicRequest,
        likes_server::Likes,
    },
};
use tonic::{Request, Response, Status, async_trait};

pub mod likes {
    tonic::include_proto!("likes");
}

pub struct LikesService {
    catalog_db: Arc<CatalogDb>,
}

impl LikesService {
    pub fn build(catalog_db: Arc<CatalogDb>) -> LikesService {
        LikesService { catalog_db }
    }
}

#[async_trait]
impl Likes for LikesService {
    async fn like_music(&self, req: Request<LikeMusicRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        let res = self.catalog_db.like_music(req.user_id, req.music_id).await;
        match res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn unlike_music(
        &self,
        req: Request<UnlikeMusicRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        let res = self
            .catalog_db
            .unlike_music(req.user_id, req.music_id)
            .await;
        match res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn get_user_liked(
        &self,
        req: Request<GetUserLikedRequest>,
    ) -> Result<Response<LikedPage>, Status> {
        let req = req.into_inner();
        let liked_music =
            self.catalog_db
                .get_liked_music_for_user(req.user_id, req.page_number, req.page_size);
        let total_count = self.catalog_db.get_liked_music_count_for_user(req.user_id);

        let res = tokio::try_join!(liked_music, total_count);
        let (liked_music, total_count) = match res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let liked_music = liked_music
            .into_iter()
            .map(|t| LikedMusic {
                id: t.id,
                title: t.title,
                thumbnail_id: t.thumbnail_id,
                duration_seconds: t.duration_seconds,
                play_count: t.play_count,
            })
            .collect();

        Ok(Response::new(LikedPage {
            liked_music,
            total_count,
        }))
    }
}
