use std::error::Error;
use music_catalog::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let conf = Config::build("config/config.local.toml")?;
    music_catalog::run(&conf).await?;
    Ok(())
}
