use std::{error::Error, net::SocketAddr};

use tonic::transport::Server;

use crate::{database::tracks, services::music_catalog_service::{MusicCatalogService, music_catalog::music_catalog_server::MusicCatalogServer}};

pub mod config;
pub mod database;
pub mod services;

pub async fn run(conf: &config::Config) -> Result<(), Box<dyn Error>> {
    let tracks_db = tracks::TracksDb::build(&conf).await?;
    let music_catalog_service = MusicCatalogService::build(tracks_db);
    let addr: SocketAddr = format!("{}:{}", &conf.service.address, conf.service.port).parse()?;

    println!("serving music catalog grpc api: {}", &addr);
    Server::builder()
        .add_service(MusicCatalogServer::new(music_catalog_service))
        .serve(addr)
        .await?;
    println!("music catalog server is down");
    Ok(())
}