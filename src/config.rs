use std::fs;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub discord_application_id: u64,
    pub discord_bot_token: String,
    pub database_url: String
}

pub fn load_config() -> Config {
    if let Ok(config_contents) = fs::read_to_string("config.toml") {
        toml::from_str(config_contents.as_str()).expect("Failed to deserialize config.toml")
    } else {
        if let Ok(discord_application_id) = std::env::var("DISCORD_APPLICATION_ID")
            && let Ok(discord_bot_token) = std::env::var("DISCORD_BOT_TOKEN")
            && let Ok(database_url) = std::env::var("DATABASE_URL") {
            Config {
                discord_application_id: discord_application_id.parse().unwrap(),
                discord_bot_token,
                database_url
            }
        } else {
            panic!("Neither config.toml or environment variables (DISCORD_APPLICATION_ID, DISCORD_BOT_TOKEN, and DATABASE_URL) could be found.");
        }
    }
}
