use std::sync::Arc;

use tonic::{Request, Response, Status, async_trait};
use crate::{database::{MusicDb, likes::LikesDb}, services::likes_service::likes::{Empty, GetUserLikedRequest, LikeTrackRequest, LikedPage, LikedTrackInfo, likes_server::Likes}};

pub mod likes {
    tonic::include_proto!("likes");
}

pub struct LikesService {
    music_db: Arc<MusicDb>,
}

impl LikesService {
    pub fn build(music_db: Arc<MusicDb>) -> LikesService {
        LikesService { music_db }
    }
}

#[async_trait]
impl Likes for LikesService {
    async fn like_track(&self, req: Request<LikeTrackRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        let res = self.music_db.like_track(req.user_id, req.track_id).await;
        match res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn get_user_liked(&self, req: Request<GetUserLikedRequest>) -> Result<Response<LikedPage>, Status> {
        let req = req.into_inner();
        let liked_tracks = self.music_db.get_liked_tracks_for_user(
            req.user_id,
            req.page_number,
            req.page_size
        );
        let total_count = self.music_db.get_liked_tracks_count_for_user(req.user_id);

        let res = tokio::try_join!(liked_tracks, total_count);
        let (liked_tracks, total_count) = match res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let liked_tracks = liked_tracks.into_iter()
            .map(|t| LikedTrackInfo {
                id: t.id,
                name: t.name,
                thumbnail_url: t.thumbnail_url,
                duration_seconds: t.duration_seconds,
                play_count: t.play_count,
            })
            .collect();

        Ok(Response::new(LikedPage {
            liked_tracks,
            total_count,
        }))
    }
}