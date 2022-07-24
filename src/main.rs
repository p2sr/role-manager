mod config;
mod bot;
mod commands;

use std::sync::Arc;
use sea_orm::{Database, DatabaseConnection};
use serenity::prelude::Client;
use serenity::prelude::GatewayIntents;
use crate::bot::BotEventHandler;

#[tokio::main]
async fn main() {
    let config = config::load_config();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .init();

    let db: DatabaseConnection = Database::connect(&config.database_url).await
        .expect(format!("Failed to open connection to database at {}", &config.database_url).as_str());

    let mut client = Client::builder(config.discord_bot_token.as_str(), GatewayIntents::all())
        .application_id(config.discord_application_id)
        .event_handler(BotEventHandler { db: Arc::new(db) })
        .await.expect("Failed to create discord client");

    client.start()
        .await.expect("Failed to start discord client");
}
