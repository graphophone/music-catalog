use tonic::{Request, Response, Status};
use music_catalog::music_catalog_server::{MusicCatalog};
use music_catalog::*;
use crate::database;

pub mod music_catalog {
    tonic::include_proto!("music_catalog");
}

pub struct MusicCatalogService {
    track_db: database::tracks::TracksDb,
}

impl MusicCatalogService {
    pub fn build(track_db: database::tracks::TracksDb) -> MusicCatalogService {
        MusicCatalogService {
            track_db,
        }
    }
}

#[tonic::async_trait]
impl MusicCatalog for MusicCatalogService {
    async fn get_full_track_info(&self, req: Request<GetTrackInfoRequest>) -> Result<Response<FullTrackInfo>, Status> {
        let req = req.into_inner();

        let query_res = self.track_db
            .get_full_track_info(req.track_id)
            .await;

        let track_info = match query_res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let res = match track_info {
            Some(info) => FullTrackInfo {
                id: info.id,
                name: info.name,
                description: info.description,
                thumbnail_url: info.thumbnail_url,
                duration_seconds: info.duration_seconds,
                play_count: info.play_count,
                like_count: info.like_count,
                user_id: info.user_id,
                categories: info.categories.into_iter()
                    .map(|c| CategoryInfo { id: c.id, name: c.name })
                    .collect(),
            },
            None => return Err(Status::not_found("track info not found")),
        };

        Ok(Response::from(res))
    }

    async fn get_short_track_info(&self, req: Request<GetTrackInfoRequest>) -> Result<Response<ShortTrackInfo>, Status> {
        let req = req.into_inner();

        let query_res = self.track_db
            .get_short_track_info(req.track_id)
            .await;

        let track_info = match query_res {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let res = match track_info {
            Some(info) => ShortTrackInfo {
                id: info.id,
                name: info.name,
                thumbnail_url: info.thumbnail_url,
                duration_seconds: info.duration_seconds,
                play_count: info.play_count,
                user_id: info.user_id,
            },
            None => return Err(Status::not_found("track info not found")),
        };

        Ok(Response::from(res))
    }
}