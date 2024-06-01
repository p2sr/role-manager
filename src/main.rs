use role_manager::boards as boards;
use role_manager::bot as bot;
use role_manager::config as config;
use role_manager::error as error;
use role_manager::analyzer as analyzer;
use role_manager::model as model;
use role_manager::multikey_hashmap as multikey_hashmap;

use std::sync::Arc;
use chrono::Duration;

use sea_orm::{Database, DatabaseConnection};
use role_manager::boards::cm::CmBoardsState;
use role_manager::boards::srcom::SrComBoardsState;

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
