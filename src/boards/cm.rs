use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

#[derive(Deserialize)]
struct AggregatedScoreData {
    score: u32,
    #[serde(rename = "playerRank")]
    player_rank: u32,
    #[serde(rename = "scoreRank")]
    score_rank: u32
}

#[derive(Deserialize)]
struct AggregatedUserData {
    #[serde(rename = "boardname")]
    board_name: String,
    avatar: String
}

#[derive(Deserialize)]
struct AggregatedPlace {
    #[serde(rename = "userData")]
    user_data: AggregatedUserData,
    #[serde(rename = "scoreData")]
    score_data: AggregatedScoreData
}

#[derive(Deserialize)]
struct AggregatedResponse {
    #[serde(rename = "Points")]
    points: HashMap<String, AggregatedPlace>
}

struct CmBoardsState {
    overall: Arc<Mutex<AggregatedResponse>>,
    sp: Arc<Mutex<AggregatedResponse>>,
    coop: Arc<Mutex<AggregatedResponse>>
}

impl CmBoardsState {
    async fn refresh_loop(&self) {
        loop {

        }
    }
}
