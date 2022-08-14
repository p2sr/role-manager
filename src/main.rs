mod boards;
mod bot;
mod commands;
mod config;
mod error;
mod analyzer;
mod model;

use std::sync::Arc;
use std::time::Duration;

use sea_orm::{Database, DatabaseConnection};
use serenity::prelude::{Client, GatewayIntents};
use tokio::sync::Mutex;
use tokio::time;
use tokio_cron_scheduler::{Job, JobScheduler};
use crate::boards::cm::CmBoardsState;

use crate::bot::BotEventHandler;

#[tokio::main]
async fn main() {
    let mut scheduler = JobScheduler::new()
        .expect("Failed to create job scheduler environment");
    let config = config::load_config();

    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).with_test_writer().init();

    let db: DatabaseConnection = Database::connect(&config.database_url).await.expect(
        format!("Failed to open connection to database at {}", &config.database_url).as_str()
    );
    let cm_state = Arc::new(Mutex::new(CmBoardsState::new().await));

    CmBoardsState::schedule_refresh(Arc::clone(&cm_state));

    let mut client = Client::builder(config.discord_bot_token.as_str(), GatewayIntents::all())
        .application_id(config.discord_application_id)
        .event_handler(BotEventHandler { db: Arc::new(db) })
        .await
        .expect("Failed to create discord client");

    client.start().await.expect("Failed to start discord client");
}
