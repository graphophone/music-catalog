use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    music_catalog::run("config/config.local.toml").await?;
    Ok(())
}
