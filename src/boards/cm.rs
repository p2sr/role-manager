use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Mutex;
use crate::error::RoleManagerError;

#[derive(Deserialize, Debug)]
pub struct AggregatedScoreData {
    pub score: u32,
    #[serde(rename = "playerRank")]
    pub player_rank: u32,
    #[serde(rename = "scoreRank")]
    pub score_rank: u32
}

#[derive(Deserialize, Debug)]
pub struct AggregatedUserData {
    #[serde(rename = "boardname")]
    pub board_name: Option<String>,
    pub avatar: Option<String>
}

#[derive(Deserialize, Debug)]
pub struct AggregatedPlace {
    #[serde(rename = "userData")]
    pub user_data: AggregatedUserData,
    #[serde(rename = "scoreData")]
    pub score_data: AggregatedScoreData
}

#[derive(Deserialize, Debug)]
pub struct AggregatedResponse {
    #[serde(rename = "Points")]
    pub points: HashMap<String, AggregatedPlace>
}

#[derive(Debug)]
pub struct CmBoardsState {
    pub overall: AggregatedResponse,
    pub sp: AggregatedResponse,
    pub coop: AggregatedResponse
}

impl CmBoardsState {
    pub async fn new() -> Self {
        CmBoardsState {
            overall: fetch_cm_leaderboard("aggregated/overall").await
                .expect("Failed to fetch overall LB"),
            sp: fetch_cm_leaderboard("aggregated/sp").await
                .expect("Failed to fetch sp LB"),
            coop: fetch_cm_leaderboard("aggregated/coop").await
                .expect("Failed to fetch coop LB")
        }
    }

    pub fn schedule_refresh(this: Arc<Mutex<Self>>) {
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60 * 15));
            loop {
                interval.tick().await;
                Self::refresh(Arc::clone(&this)).await;
            }
        });
    }

    pub async fn refresh(this: Arc<Mutex<Self>>) {
        let mut state = this.lock().await;

        state.overall = fetch_cm_leaderboard("aggregated/overall").await
            .expect("Failed to fetch overall LB");
        state.sp = fetch_cm_leaderboard("aggregated/sp").await
            .expect("Failed to fetch sp LB");
        state.coop = fetch_cm_leaderboard("aggregated/coop").await
            .expect("Failed to fetch coop LB");
    }
}

async fn fetch_cm_leaderboard(page: &str) -> Result<AggregatedResponse, RoleManagerError> {
    Ok(reqwest::get(format!("https://board.portal2.sr/{}/json", page))
        .await.map_err(|err| format!("Failed to request {} page on board.portal2.sr: {}", page, err) )?
        .json::<AggregatedResponse>()
        .await.map_err(|err| format!("Failed to convert response from {} page on board.portal2.sr: {}", page, err) )?)
}
