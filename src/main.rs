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
use serenity::model::id::GuildId;
use serenity::prelude::{Client, GatewayIntents};
use tokio::sync::Mutex;
use tokio::time;
use crate::boards::cm::CmBoardsState;

use crate::bot::{BotState};

#[tokio::main]
async fn main() {
    let config = config::load_config();

    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).with_test_writer().init();

    let db: DatabaseConnection = Database::connect(&config.database_url).await.expect(
        format!("Failed to open connection to database at {}", &config.database_url).as_str()
    );
    let cm_state = Arc::new(Mutex::new(CmBoardsState::new().await));
    CmBoardsState::schedule_refresh(Arc::clone(&cm_state));

    bot::create_bot(config, Arc::new(db), cm_state).await;
}
