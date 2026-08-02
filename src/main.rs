use music_catalog::app_config;

fn main() {
    let conf = app_config::AppConfig::build("config/config.local.toml")
        .expect("failed to read config");
    dbg!(conf);
}
