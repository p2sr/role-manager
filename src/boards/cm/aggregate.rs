use std::collections::HashMap;
use std::sync::Arc;
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
    pub aggregate: Arc<AggregatedResponse>,
    pub fetched_at: NaiveDateTime
}

pub async fn fetch_aggregate(page: &str) -> Result<AggregatedResponse, RoleManagerError> {
    let url = format!("https://board.portal2.sr/{}/json", page);
    let resp = reqwest::get(&url).await.map_err(|err| format!("Failed to request {} page on board.portal2.sr: {}", page, err) )?;
    let body = resp.text().await.map_err(|err| format!("Failed to read response body from {} page on board.portal2.sr: {}", page, err) )?;

    let parsed = match json5::from_str::<AggregatedResponse>(&body) {
        Ok(v) => v,
        Err(err) => {
            // Log a snippet from the start and end of the body to help debugging
            let head: String = body.chars().take(200).collect();
            let tail: String = body.chars().rev().take(200).collect::<String>().chars().rev().collect();
            println!("Failed to convert response from {} page on board.portal2.sr: {}", page, err);
            println!("Response head (first 200 chars):\n{}", head);
            println!("Response tail (last 200 chars):\n{}", tail);

            return Err(format!("Failed to convert response from {} page on board.portal2.sr: {}", page, err).into());
        }
    };

    Ok(parsed)
}
