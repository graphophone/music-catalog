use std::error::Error;

use crate::database::tracks;

pub mod config;
pub mod database;

pub async fn run(config_filename: &str) -> Result<(), Box<dyn Error>> {
    let conf = crate::config::Config::build(config_filename)?;
    dbg!(&conf);
    let tracks_db = tracks::TracksDb::build(&conf).await?;

    let track_id = tracks_db.save_track_info(&tracks::UploadTrackInfo{
        name: String::from("White dove"),
        description: Some(String::from("Description")),
        user_id: 1,
    }).await?;

    let track_info = tracks_db.get_full_track_info(track_id).await?;
    match track_info.as_ref() {
        Some(info) => { dbg!(info); },
        None => eprintln!("no track was found with id = {}", track_id),
    }
    Ok(())
}