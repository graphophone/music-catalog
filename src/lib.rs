use std::{error::Error, net::SocketAddr, sync::Arc};

use tonic::transport::Server;

use crate::{database::MusicDb, services::{categories_service::{CategoriesServer, CategoriesService}, tracks_service::{TracksServer, TracksService}}};

pub mod config;
mod database;   
pub mod services;

pub async fn run(conf: &config::Config) -> Result<(), Box<dyn Error>> {
    let music_db = MusicDb::build(&conf).await?;
    let music_db = Arc::new(music_db);

    let addr: SocketAddr = format!("{}:{}", &conf.service.address, conf.service.port).parse()?;
    let tracks_service = TracksService::build(Arc::clone(&music_db));
    let categories_service = CategoriesService::build(Arc::clone(&music_db));

    println!("serving music catalog grpc api: {}", &addr);
    Server::builder()
        .add_service(TracksServer::new(tracks_service))
        .add_service(CategoriesServer::new(categories_service))
        .serve(addr)
        .await?;
    println!("music catalog server is down");
    Ok(())
}