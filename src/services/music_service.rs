use std::sync::Arc;

use crate::asset_storage::AssetStorage;
use crate::audio_processing_queue::AudioProcessingQueue;
use crate::config::RustfsConfig;
use crate::database;
use crate::database::CatalogDb;
use crate::database::music::MusicDb;
use music::music_server::Music;
use music::*;
use tonic::{Request, Response, Status};

pub use music::music_server::MusicServer;

pub mod music {
    tonic::include_proto!("music");
}

pub struct MusicService {
    catalog_db: Arc<CatalogDb>,
    thumbnail_storage: Arc<AssetStorage>,
    raw_audio_storage: Arc<AssetStorage>,
    hls_audio_storage: Arc<AssetStorage>,
    audio_processing_queue: Arc<AudioProcessingQueue>,
}

impl MusicService {
    pub async fn build(
        storage_conf: &RustfsConfig,
        music_db: Arc<CatalogDb>,
        audio_processing_queue: Arc<AudioProcessingQueue>,
    ) -> anyhow::Result<MusicService> {
        let (is, ras, sas) = tokio::try_join!(
            AssetStorage::build(storage_conf.thumbnails_bucket.clone(), storage_conf),
            AssetStorage::build(storage_conf.raw_audio_bucket.clone(), storage_conf),
            AssetStorage::build(storage_conf.hls_audio_bucket.clone(), storage_conf),
        )?;

        Ok(MusicService {
            catalog_db: music_db,
            thumbnail_storage: Arc::new(is),
            raw_audio_storage: Arc::new(ras),
            hls_audio_storage: Arc::new(sas),
            audio_processing_queue,
        })
    }
}

#[tonic::async_trait]
impl Music for MusicService {
    async fn get_full_music(&self, req: Request<MusicId>) -> Result<Response<FullMusic>, Status> {
        let req = req.into_inner();

        let query_res = self.catalog_db.get_full_music(req.id).await;

        let music = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("music not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        let res = FullMusic {
            id: music.id,
            title: music.title,
            description: music.description,
            thumbnail_id: music
                .thumbnail_id
                .map(|v| self.thumbnail_storage.prepare_asset_id(&v)),
            duration_seconds: music.duration_seconds,
            play_count: music.play_count,
            like_count: music.like_count,
            uploader_id: music.uploader_id,
            categories: music
                .categories
                .into_iter()
                .map(|c| Category {
                    id: c.id,
                    name: c.name,
                })
                .collect(),
            upload_status: music.upload_status,
            created_at: music.created_at.unix_timestamp(),
        };

        Ok(Response::new(res))
    }

    async fn upload_music(
        &self,
        req: Request<UploadMusicReq>,
    ) -> Result<Response<MusicId>, Status> {
        let req = req.into_inner();

        let music_data = database::music::UploadMusic {
            title: req.title,
            description: req.description,
            uploader_id: req.uploader_id,
            categories_ids: req.categories_ids,
        };
        let query_res = self.catalog_db.save_music(&music_data).await;

        match query_res {
            Ok(id) => Ok(Response::new(MusicId { id })),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn update_music(&self, req: Request<UpdateMusicReq>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let music_data = database::music::UpdateMusic {
            title: req.title,
            description: req.description,
            categories_ids: req.categories_ids,
        };
        let query_res = self
            .catalog_db
            .update_music(req.music_id, &music_data)
            .await;

        match query_res {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(sqlx::Error::RowNotFound) => Err(Status::not_found("music not found")),
            Err(e) => Err(Status::from_error(Box::new(e))),
        }
    }

    async fn upload_thumbnail(
        &self,
        req: Request<UploadAssetReq>,
    ) -> Result<Response<UploadAssetRes>, Status> {
        let req = req.into_inner();

        let (upload_url, id) = self
            .thumbnail_storage
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
            .catalog_db
            .update_thumbnail(req.music_id, Some(req.asset_id))
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        if let Some(old_thumbnail_id) = old_thumbnail_id {
            let thumbnail_storage = Arc::clone(&self.thumbnail_storage);
            tokio::spawn(async move {
                let old_id = old_thumbnail_id;
                let _ = thumbnail_storage.remove_asset(&old_id).await;
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
            .catalog_db
            .update_thumbnail(req.music_id, None)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        if let Some(old_thumbnail_id) = old_thumbnail_id {
            let thumbnail_storage = Arc::clone(&self.thumbnail_storage);
            tokio::spawn(async move {
                let old_id = old_thumbnail_id;
                let _ = thumbnail_storage.remove_asset(&old_id).await;
            });
        }

        Ok(Response::new(Empty {}))
    }

    async fn remove_music(&self, req: Request<RemoveMusicReq>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let res = self.catalog_db.remove_music(req.music_id).await;

        let assets = match res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("music not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        if let Some(thumbnail_id) = assets.thumbnail_id {
            let thumbnail_storage = Arc::clone(&self.thumbnail_storage);
            tokio::spawn(async move {
                let thumbnail_id = thumbnail_id;
                let _ = thumbnail_storage.remove_asset(&thumbnail_id).await;
            });
        }

        if let Some(raw_audio_id) = assets.raw_audio_id {
            let raw_audio_storage = Arc::clone(&self.raw_audio_storage);
            tokio::spawn(async move {
                let raw_audio_id = raw_audio_id;
                let _ = raw_audio_storage.remove_asset(&raw_audio_id).await;
            });
        }

        if let Some(_) = assets.audio_id {
            let hls_storage = Arc::clone(&self.hls_audio_storage);
            tokio::spawn(async move {
                let _ = hls_storage
                    .remove_objects_with_prefix(format!("{}/", req.music_id))
                    .await;
            });
        };

        Ok(Response::new(Empty {}))
    }

    async fn link_music_audio(
        &self,
        req: Request<LinkMusicAudioReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        let link = database::music::LinkAudio {
            audio_id: req.audio_id,
            duration_seconds: req.duration_seconds,
        };
        let query_res = self.catalog_db.link_music_audio(req.music_id, &link).await;

        let raw_audio_id = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("music not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        if let Some(raw_audio_id) = raw_audio_id {
            let raw_audio_storage = Arc::clone(&self.raw_audio_storage);
            tokio::spawn(async move {
                let raw_audio_id = raw_audio_id;
                let _ = raw_audio_storage.remove_asset(&raw_audio_id).await;
            });
        }

        Ok(Response::new(Empty {}))
    }

    async fn request_music_play(
        &self,
        req: Request<MusicId>,
    ) -> Result<Response<MusicPlayResponse>, Status> {
        let req = req.into_inner();

        let query_res = self.catalog_db.register_play(req.id).await;

        let hls_id = match query_res {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Err(Status::not_found("music not found")),
            Err(e) => return Err(Status::from_error(Box::new(e))),
        };

        Ok(Response::new(MusicPlayResponse {
            hls_master_id: hls_id,
        }))
    }

    async fn upload_music_audio(
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

    async fn notify_upload_music_audio(
        &self,
        req: Request<NotifyUploadAssetReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        self.catalog_db
            .update_upload_status(req.music_id, "waiting_for_processing")
            .await
            .map_err(|_| Status::internal("failed to update upload status"))?;

        self.audio_processing_queue
            .send_audio_processing_request(req.music_id, &req.asset_id)
            .await
            .map_err(|e| {
                Status::internal(format!(
                    "failed to send audio processing request: {}",
                    e.to_string()
                ))
            })?;

        Ok(Response::new(Empty {}))
    }

    async fn check_uploader(
        &self,
        req: Request<CheckUploaderReq>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        match self
            .catalog_db
            .check_uploader(req.music_id, req.uploader_id)
            .await
        {
            Ok(_) => Ok(Response::new(Empty {})),
            Err(_) => Err(Status::unauthenticated("not authenticated")),
        }
    }
}
