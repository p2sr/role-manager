use std::collections::HashMap;
use chrono::NaiveDateTime;
use serde::Deserialize;
use crate::boards::cm::profile::Profile;
use crate::error::RoleManagerError;

#[derive(Deserialize, Debug, Clone)]
pub struct AggregatedScoreData {
    pub score: u32,
    #[serde(rename = "playerRank")]
    pub player_rank: u32,
    #[serde(rename = "scoreRank")]
    pub score_rank: u32
}

#[derive(Deserialize, Debug, Clone)]
pub struct AggregatedPlace {
    #[serde(rename = "userData")]
    pub user_data: Profile,
    #[serde(rename = "scoreData")]
    pub score_data: AggregatedScoreData
}

#[derive(Deserialize, Debug, Clone)]
pub struct AggregatedResponse {
    #[serde(rename = "Points")]
    pub points: HashMap<String, AggregatedPlace>
}

#[derive(Debug)]
pub struct CachedAggregate {
    pub aggregate: AggregatedResponse,
    pub fetched_at: NaiveDateTime
}

pub async fn fetch_aggregate(page: &str) -> Result<AggregatedResponse, RoleManagerError> {
    Ok(reqwest::get(format!("https://board.portal2.sr/{}/json", page))
        .await.map_err(|err| format!("Failed to request {} page on board.portal2.sr: {}", page, err) )?
        .json::<AggregatedResponse>()
        .await.map_err(|err| format!("Failed to convert response from {} page on board.portal2.sr: {}", page, err) )?)
}
