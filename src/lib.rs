use std::{error::Error, net::SocketAddr, sync::Arc};

use tonic::transport::Server;
use tower_http::trace::TraceLayer;

use crate::{
    audio_processing_manager::AudioProcessingManager,
    database::CatalogDb,
    services::{
        categories_service::{CategoriesServer, CategoriesService},
        likes_service::{LikesService, likes::likes_server::LikesServer},
        music_service::{MusicServer, MusicService},
    },
};

mod asset_storage;
pub mod audio_processing_manager;
pub mod config;
mod database;
pub mod services;

pub async fn run(conf: &config::Config) -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let catalog_db = Arc::new(CatalogDb::build(&conf).await?);

    let audio_processing_manager = Arc::new(AudioProcessingManager::build(&conf.rabbitmq).await?);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let music_service = MusicService::build(
        &conf.rustfs,
        Arc::clone(&catalog_db),
        audio_processing_manager,
    )
    .await?;
    let categories_service = CategoriesService::build(Arc::clone(&catalog_db));
    let likes_service = LikesService::build(Arc::clone(&catalog_db));

    println!("serving music catalog grpc api: {}", &addr);
    Server::builder()
        .layer(TraceLayer::new_for_grpc())
        .add_service(MusicServer::new(music_service))
        .add_service(CategoriesServer::new(categories_service))
        .add_service(LikesServer::new(likes_service))
        .serve(addr)
        .await?;
    println!("music catalog server is down");
    Ok(())
}
