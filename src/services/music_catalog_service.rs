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
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let res = FullTrackInfo {
            id: track_info.id,
            name: track_info.name,
            description: track_info.description,
            thumbnail_url: track_info.thumbnail_url,
            duration_seconds: track_info.duration_seconds,
            play_count: track_info.play_count,
            like_count: track_info.like_count,
            user_id: track_info.user_id,
            categories: track_info.categories.into_iter()
                .map(|c| CategoryInfo { id: c.id, name: c.name })
                .collect(),
        };
        Ok(Response::new(res))
    }

    async fn get_short_track_info(&self, req: Request<GetTrackInfoRequest>) -> Result<Response<ShortTrackInfo>, Status> {
        let req = req.into_inner();

        let query_res = self.track_db
            .get_short_track_info(req.track_id)
            .await;

        let track_info = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let res = ShortTrackInfo {
            id: track_info.id,
            name: track_info.name,
            thumbnail_url: track_info.thumbnail_url,
            duration_seconds: track_info.duration_seconds,
            play_count: track_info.play_count,
            user_id: track_info.user_id,
        };
        Ok(Response::new(res))
    }

    async fn upload_track_info(&self, req: Request<UploadTrackInfoRequest>) -> Result<Response<UploadTrackInfoResponse>, Status> {
        let req = req.into_inner();

        let track_info = database::tracks::UploadTrackInfo {
            name: req.name,
            description: req.description,
            user_id: req.user_id,
        };
        let query_res = self.track_db
            .save_track_info(&track_info)
            .await;

        match query_res {
            Ok(id) => Ok(Response::new(UploadTrackInfoResponse { id })),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn update_track_info(&self, req: Request<UpdateTrackInfoRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let track_info = database::tracks::UpdateTrackInfo {
            name: req.name,
            description: req.description,
            category_ids: req.category_ids,
        };
        let query_res = self.track_db
            .update_track_info(req.track_id, &track_info)
            .await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("track not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn update_track_thumbnail(&self, req: Request<UpdateTrackThumbnailRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let query_res = self.track_db
            .update_track_thumbnail(req.track_id, "placeholder url for now")
            .await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("track not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn remove_track_info(&self, req: Request<RemoveTrackInfoRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let query_res = self.track_db
            .remove_track_info(req.track_id)
            .await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("track not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn link_track_audio(&self, req: Request<LinkTrackAudioRequest>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let link_info = database::tracks::LinkAudioInfo {
            audio_uri: req.audio_uri,
            duration_seconds: req.duration_seconds,
        };
        let query_res = self.track_db
            .link_track_audio(req.track_id, &link_info)
            .await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("track not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn generate_play_token(&self, req: Request<GeneratePlayTokenRequest>) -> Result<Response<PlayToken>, Status> {
        let req = req.into_inner();

        let query_res = self.track_db
            .register_play(req.track_id)
            .await;

        let audio_uri = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        Ok(Response::new(PlayToken { play_token: audio_uri }))
    }
}