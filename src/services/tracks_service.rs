use std::sync::Arc;

use crate::asset_storage::AssetStorage;
use crate::config::RustfsConfig;
use crate::database::tracks::{TracksDb, UpdateThumbnail, UpdateUploadStatus};
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
    image_storage: Arc<AssetStorage>,
    raw_audio_storage: Arc<AssetStorage>,
    streaming_audio_storage: Arc<AssetStorage>,
    play_token_key: String,
}

impl TracksService {
    pub async fn build(
        storage_conf: &RustfsConfig,
        music_db: Arc<MusicDb>,
        play_token_key: String,
    ) -> anyhow::Result<TracksService> {
        let (is, ras, sas) = tokio::try_join!(
            AssetStorage::build(storage_conf.images_bucket.clone(), storage_conf,),
            AssetStorage::build(storage_conf.raw_audio_bucket.clone(), storage_conf,),
            AssetStorage::build(storage_conf.streaming_audio_bucket.clone(), storage_conf,),
        )?;

        Ok(TracksService {
            music_db,
            image_storage: Arc::new(is),
            raw_audio_storage: Arc::new(ras),
            streaming_audio_storage: Arc::new(sas),
            play_token_key,
        })
    }
}

#[tonic::async_trait]
impl Tracks for TracksService {
    async fn get_full_track(&self, req: Request<TrackId>) -> Result<Response<FullTrack>, Status> {
        let req = req.into_inner();

        let query_res = self.music_db.get_full_track(req.id).await;

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
            upload_status: track.upload_status,
            created_at: track.created_at.unix_timestamp(),
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

    async fn update_track(&self, req: Request<UpdateTrackReq>) -> Result<Response<Empty>, Status> {
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

    async fn upload_thumbnail(
        &self,
        req: Request<UploadAssetReq>,
    ) -> Result<Response<UploadAssetRes>, Status> {
        let req = req.into_inner();

        let (upload_url, id) = self
            .image_storage
            .get_upload_url(&req.mime_type)
            .await
            .map_err(|_| Status::internal("failed to upload thumbnail"))?;

        Ok(Response::new(UploadAssetRes {
            upload_url,
            asset_id: id,
        }))
    }

    async fn notify_upload_thumbnail(
        &self,
        req: Request<NotifyUploadAssetReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let old_thumbnail_id = self
            .music_db
            .update_thumbnail(
                req.track_id,
                &UpdateThumbnail {
                    thumbnail_id: Some(req.asset_id),
                    uploader_id: req.uploader_id,
                },
            )
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        if let Some(old_thumbnail_id) = old_thumbnail_id {
            let image_storage = Arc::clone(&self.image_storage);
            tokio::spawn(async move {
                let old_id = old_thumbnail_id;
                let _ = image_storage.remove_asset(&old_id).await;
            });
        }

        Ok(Response::new(Empty {}))
    }

    async fn remove_thumbnail(
        &self,
        req: Request<RemoveThumbnailReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let old_thumbnail_id = self
            .music_db
            .update_thumbnail(
                req.track_id,
                &UpdateThumbnail {
                    thumbnail_id: None,
                    uploader_id: req.uploader_id,
                },
            )
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        if let Some(old_thumbnail_id) = old_thumbnail_id {
            let image_storage = Arc::clone(&self.image_storage);
            tokio::spawn(async move {
                let old_id = old_thumbnail_id;
                let _ = image_storage.remove_asset(&old_id).await;
            });
        }

        Ok(Response::new(Empty {}))
    }

    async fn remove_track(&self, req: Request<RemoveTrackReq>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let res = self
            .music_db
            .remove_track(req.track_id, req.uploader_id)
            .await;

        let assets = match res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("track not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        match assets.thumbnail_id {
            Some(thumbnail_id) => {
                let image_storage = Arc::clone(&self.image_storage);
                tokio::spawn(async move {
                    let thumbnail_id = thumbnail_id;
                    let _ = image_storage.remove_asset(&thumbnail_id).await;
                });
            }
            None => (),
        };

        match assets.audio_uri {
            Some(audio_uri) => {
                // send remove request to music storage
            }
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

        let query_res = self.music_db.register_play(req.id).await;

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

    async fn upload_track_audio(
        &self,
        req: Request<UploadAssetReq>,
    ) -> Result<Response<UploadAssetRes>, Status> {
        let req = req.into_inner();

        let (upload_url, id) = self
            .raw_audio_storage
            .get_upload_url(&req.mime_type)
            .await
            .map_err(|_| Status::internal("failed to upload thumbnail"))?;

        Ok(Response::new(UploadAssetRes {
            upload_url,
            asset_id: id,
        }))
    }

    async fn notify_upload_track_audio(
        &self,
        req: Request<NotifyUploadAssetReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        // Request audio fragmentation from audio processing service

        self.music_db
            .update_upload_status(
                req.track_id,
                &UpdateUploadStatus {
                    uploader_id: req.uploader_id,
                    new_status: "waiting_for_processing".to_string(),
                },
            )
            .await
            .map_err(|_| Status::internal("failed to update upload status"))?;

        Ok(Response::new(Empty {}))
    }
}
