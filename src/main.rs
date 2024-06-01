mod boards;
mod bot;
mod config;
mod error;
mod analyzer;
mod model;
mod multikey_hashmap;

use std::sync::Arc;
use chrono::Duration;

use sea_orm::{Database, DatabaseConnection};
use crate::boards::cm::CmBoardsState;
use crate::boards::srcom::SrComBoardsState;

#[tokio::main]
async fn main() {
    let config = config::load_config();

    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).with_test_writer().init();

    let db: DatabaseConnection = Database::connect(&config.database_url).await.expect(
        format!("Failed to open connection to database at {}", &config.database_url).as_str()
    );

    let srcom_state = SrComBoardsState::new(Duration::minutes(15));
    let cm_state = CmBoardsState::new(Duration::minutes(15));

    bot::create_bot(config, Arc::new(db), srcom_state, cm_state).await;
}
