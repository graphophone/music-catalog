use std::{error::Error, net::SocketAddr, sync::Arc};

use tonic::transport::Server;

use crate::{database::MusicDb, services::{categories_service::{CategoriesServer, CategoriesService}, likes_service::{LikesService, likes::likes_server::LikesServer}, tracks_service::{TracksServer, TracksService}}};

pub mod config;
mod database;   
pub mod services;
pub mod token;

pub mod proto { 
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = 
        tonic::include_file_descriptor_set!("service_reflection"); 
} 

pub async fn run(conf: &config::Config) -> Result<(), Box<dyn Error>> {
    let music_db = MusicDb::build(&conf).await?;
    let music_db = Arc::new(music_db);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let tracks_service = TracksService::build(
        Arc::clone(&music_db),
        conf.streaming.play_token_key.clone(),
    );
    let categories_service = CategoriesService::build(Arc::clone(&music_db));
    let likes_service = LikesService::build(Arc::clone(&music_db));
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    println!("serving music catalog grpc api: {}", &addr);
    Server::builder()
        .add_service(reflection_service)
        .add_service(TracksServer::new(tracks_service))
        .add_service(CategoriesServer::new(categories_service))
        .add_service(LikesServer::new(likes_service))
        .serve(addr)
        .await?;
    println!("music catalog server is down");
    Ok(())
}
