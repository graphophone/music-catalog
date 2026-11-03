use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::{
    database::{
        CatalogDb,
        music::{LinkAudio, MusicDb},
    },
    services::audio_processing_service::audio_processing::{
        Empty, HslId, MusicId, audio_processing_server::AudioProcessing,
    },
};

pub mod audio_processing {
    tonic::include_proto!("audio_processing");
}

pub struct AudioProcessingService {
    catalog_db: Arc<CatalogDb>,
}

impl AudioProcessingService {
    pub fn new(catalog_db: Arc<CatalogDb>) -> Self {
        AudioProcessingService { catalog_db }
    }
}

#[tonic::async_trait]
impl AudioProcessing for AudioProcessingService {
    async fn notify_audio_processing_failed(
        &self,
        req: Request<MusicId>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        self.catalog_db
            .update_upload_status(req.music_id, "upload_failed")
            .await
            .map_err(|_| Status::internal("fail"))?;

        Ok(Response::new(Empty {}))
    }

    async fn notify_audio_processing_started(
        &self,
        req: Request<MusicId>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        self.catalog_db
            .update_upload_status(req.music_id, "processing_started")
            .await
            .map_err(|_| Status::internal("fail"))?;

        Ok(Response::new(Empty {}))
    }

    async fn notify_audio_processing_finished(
        &self,
        req: Request<HslId>,
    ) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();

        self.catalog_db
            .link_music_audio(
                req.music_id,
                &LinkAudio {
                    audio_uri: req.hls_id,
                    duration_seconds: req.duration_seconds,
                },
            )
            .await
            .map_err(|_| Status::internal("failed to link audio"))?;

        self.catalog_db
            .update_upload_status(req.music_id, "uploaded")
            .await
            .map_err(|_| Status::internal("fail"))?;

        Ok(Response::new(Empty {}))
    }
}
