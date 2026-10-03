use std::{error::Error, net::SocketAddr, sync::Arc};

use tonic::transport::Server;
use tower_http::trace::TraceLayer;

use crate::{
    asset_storage::AssetStorage, database::MusicDb, services::{
        categories_service::{CategoriesServer, CategoriesService},
        likes_service::{LikesService, likes::likes_server::LikesServer},
        tracks_service::{TracksServer, TracksService},
    },
};

pub mod config;
mod database;
pub mod services;
mod asset_storage;
pub mod token;

pub async fn run(conf: &config::Config) -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let music_db = MusicDb::build(&conf).await?;
    let music_db = Arc::new(music_db);
    let asset_storage = AssetStorage::build(&conf.rustfs).await?;
    let asset_storage = Arc::new(asset_storage);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let tracks_service =
        TracksService::build(
            Arc::clone(&music_db),
            asset_storage,
            conf.streaming.play_token_key.clone(),
        );
    let categories_service = CategoriesService::build(
        Arc::clone(&music_db),
    );
    let likes_service = LikesService::build(
        Arc::clone(&music_db),
    );

    println!("serving music catalog grpc api: {}", &addr);
    Server::builder()
        .layer(TraceLayer::new_for_grpc())
        .add_service(TracksServer::new(tracks_service))
        .add_service(CategoriesServer::new(categories_service))
        .add_service(LikesServer::new(likes_service))
        .serve(addr)
        .await?;
    println!("music catalog server is down");
    Ok(())
}
