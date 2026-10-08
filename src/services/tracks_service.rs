use std::sync::Arc;

use crate::asset_storage::AssetStorage;
use crate::database::tracks::TracksDb;
use crate::database::{self, MusicDb};
use crate::token;
use tonic::{Request, Response, Status};
use tracks::tracks_server::Tracks;
use tracks::*;

pub use tracks::tracks_server::TracksServer;

pub mod tracks {
    tonic::include_proto!("tracks");
}

pub struct TracksService {
    music_db: Arc<MusicDb>,
    asset_storage: Arc<AssetStorage>,
    play_token_key: String,
}

impl TracksService {
    pub fn build(
        music_db: Arc<MusicDb>,
        asset_storage: Arc<AssetStorage>,
        play_token_key: String,
    ) -> TracksService {
        TracksService {
            music_db,
            asset_storage,
            play_token_key,
        }
    }
}

#[tonic::async_trait]
impl Tracks for TracksService {
    async fn get_full_track(
        &self,
        req: Request<TrackId>,
    ) -> Result<Response<FullTrack>, Status> {
        let req = req.into_inner();

        let query_res = self
            .music_db.get_full_track(req.id)
            .await;

        let track = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let res = FullTrack {
            id: track.id,
            title: track.title,
            description: track.description,
            thumbnail_id: track.thumbnail_id,
            duration_seconds: track.duration_seconds,
            play_count: track.play_count,
            like_count: track.like_count,
            uploader_id: track.uploader_id,
            categories: track
                .categories
                .into_iter()
                .map(|c| Category {
                    id: c.id,
                    name: c.name,
                })
                .collect(),
        };

        Ok(Response::new(res))
    }

    async fn upload_track(
        &self,
        req: Request<UploadTrackReq>,
    ) -> Result<Response<TrackId>, Status> {
        let req = req.into_inner();

        let track_data = database::tracks::UploadTrack {
            title: req.title,
            description: req.description,
            uploader_id: req.uploader_id,
            categories_ids: req.categories_ids,
        };
        let query_res = self.music_db.save_track(&track_data).await;

        match query_res {
            Ok(id) => Ok(Response::new(TrackId { id })),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn update_track(
        &self,
        req: Request<UpdateTrackReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let track_data = database::tracks::UpdateTrack {
            title: req.title,
            description: req.description,
            categories_ids: req.categories_ids,
            uploader_id: req.uploader_id,
        };
        let query_res = self.music_db.update_track(req.track_id, &track_data).await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("track not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn update_track_thumbnail(
        &self,
        req: Request<UpdateTrackThumbnailReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let mut thumbnail_id = None;
        if let Some(thumbnail) = req.thumbnail {
            let id = self.asset_storage
                .upload_asset(&thumbnail.image, &thumbnail.mime_type)
                .await
                .map_err(|_| Status::internal("failed to upload thumbnail"))?;
            thumbnail_id = Some(id);
        };

        let old_thumbnail_id = self
            .music_db
            .update_track_thumbnail(req.track_id, &database::tracks::UpdateTrackThumbnail {
                thumbnail_id,
                uploader_id: req.uploader_id,
            })
            .await;

       let old_thumbnail_id = match old_thumbnail_id {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        if let Some(old_id) = old_thumbnail_id {
            let asset_storage = Arc::clone(&self.asset_storage);
            tokio::spawn(async move {
                let old_id = old_id;
                let _ = asset_storage
                    .remove_asset(&old_id)
                    .await;
            });
        }

        Ok(Response::new(Empty {}))
    }

    async fn remove_track(
        &self,
        req: Request<RemoveTrackReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let res = self.music_db.remove_track(
            req.track_id,
            req.uploader_id,
        ).await;

        let assets = match res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        match assets.thumbnail_id {
            Some(thumbnail_id) => {
                let asset_storage = Arc::clone(&self.asset_storage);
                tokio::spawn(async move {
                    let thumbnail_id = thumbnail_id;
                    let _ = asset_storage.remove_asset(&thumbnail_id).await;
                });
            },
            None => (),
        };

        match assets.audio_uri {
            Some(audio_uri) => {
                // send remove request to music storage
            },
            None => (),
        };

        Ok(Response::new(Empty {}))
    }

    async fn link_track_audio(
        &self,
        req: Request<LinkTrackAudioReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let link = database::tracks::LinkAudio {
            audio_uri: req.audio_uri,
            duration_seconds: req.duration_seconds,
            uploader_id: req.uploader_id,
        };
        let query_res = self.music_db.link_track_audio(req.track_id, &link).await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("track not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn generate_play_token(
        &self,
        req: Request<TrackId>,
    ) -> Result<Response<PlayToken>, Status> {
        let req = req.into_inner();

        let query_res = self.music_db
            .register_play(req.id)
            .await;

        let audio_uri = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let claims = token::Claims::new(audio_uri);
        let play_token = match claims.to_token(&self.play_token_key) {
            Ok(v) => v,
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };
        Ok(Response::new(PlayToken { play_token }))
    }
}
