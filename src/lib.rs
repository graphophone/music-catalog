use std::{error::Error, net::SocketAddr};

use tonic::transport::Server;

use crate::{database::{MusicDb}, services::tracks_service::{TracksService, tracks::tracks_server::TracksServer}};

pub mod config;
mod database;
pub mod services;

pub async fn run(conf: &config::Config) -> Result<(), Box<dyn Error>> {
    let tracks_db = MusicDb::build(&conf).await?;
    let tracks_service = TracksService::build(tracks_db);
    let addr: SocketAddr = format!("{}:{}", &conf.service.address, conf.service.port).parse()?;

    println!("serving music catalog grpc api: {}", &addr);
    Server::builder()
        .add_service(TracksServer::new(tracks_service))
        .serve(addr)
        .await?;
    println!("music catalog server is down");
    Ok(())
}