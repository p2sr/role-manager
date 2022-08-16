use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{Duration as ChronoDuration, NaiveDateTime, Utc};

use serde::Deserialize;
use tokio::sync::Mutex;
use crate::analyzer::role_definition::CmLeaderboard;
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
pub struct AggregatedUserData {
    #[serde(rename = "boardname")]
    pub board_name: Option<String>,
    pub avatar: Option<String>
}

#[derive(Deserialize, Debug, Clone)]
pub struct AggregatedPlace {
    #[serde(rename = "userData")]
    pub user_data: AggregatedUserData,
    #[serde(rename = "scoreData")]
    pub score_data: AggregatedScoreData
}

#[derive(Deserialize, Debug, Clone)]
pub struct AggregatedResponse {
    #[serde(rename = "Points")]
    pub points: HashMap<String, AggregatedPlace>
}

#[derive(Debug, Clone)]
pub struct CmBoardsState {
    cache_persist_time: ChronoDuration,

    cached_aggregates: Arc<Mutex<HashMap<CmLeaderboard, CachedAggregate>>>,
    cached_active_profiles: Arc<Mutex<HashMap<u64, CachedActiveProfiles>>>
}

#[derive(Debug)]
struct CachedAggregate {
    aggregate: AggregatedResponse,
    fetched_at: NaiveDateTime
}

#[derive(Debug)]
struct CachedActiveProfiles {
    active_profiles: Vec<String>,
    fetched_at: NaiveDateTime
}

impl CmBoardsState {
    pub fn new(cache_persist_time: ChronoDuration) -> Self {
        CmBoardsState {
            cache_persist_time,

            cached_aggregates: Arc::new(Mutex::new(HashMap::new())),
            cached_active_profiles: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub async fn fetch_aggregate(&self, leaderboard: &CmLeaderboard) -> Result<AggregatedResponse, RoleManagerError> {
        let mut cache = self.cached_aggregates.lock().await;

        match cache.get(leaderboard).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_aggregate) => {
                Ok(cached_aggregate.aggregate.clone())
            }
            None => {
                let page = match leaderboard {
                    CmLeaderboard::Overall => "aggregated/overall",
                    CmLeaderboard::SinglePlayer => "aggregated/sp",
                    CmLeaderboard::Coop => "aggregated/coop"
                };

                let aggregate = fetch_cm_leaderboard(page).await?;

                cache.insert(*leaderboard, CachedAggregate {
                    aggregate: aggregate.clone(),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(aggregate)
            }
        }
    }

    pub async fn fetch_active_profiles(&self, months: u64) -> Result<Vec<String>, RoleManagerError> {
        let mut cache = self.cached_active_profiles.lock().await;

        match cache.get(&months).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_profiles) => {
                Ok(cached_profiles.active_profiles.clone())
            }
            None => {
                let profiles = fetch_active_profiles(months).await?;

                cache.insert(months, CachedActiveProfiles {
                    active_profiles: profiles.clone(),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(profiles)
            }
        }
    }
}

async fn fetch_cm_leaderboard(page: &str) -> Result<AggregatedResponse, RoleManagerError> {
    Ok(reqwest::get(format!("https://board.portal2.sr/{}/json", page))
        .await.map_err(|err| format!("Failed to request {} page on board.portal2.sr: {}", page, err) )?
        .json::<AggregatedResponse>()
        .await.map_err(|err| format!("Failed to convert response from {} page on board.portal2.sr: {}", page, err) )?)
}

async fn fetch_active_profiles(months: u64) -> Result<Vec<String>, RoleManagerError> {
    let client = reqwest::Client::new();
    Ok(client.post("https://board.portal2.sr/api-v2/active-profiles")
        .form(&[("months", months)])
        .send()
        .await.map_err(|err| format!("Failed to request active profiles on board.portal2.sr: {}", err))?
        .json::<ActiveProfilesResponse>()
        .await.map_err(|err| format!("Failed to convert response from active profiles on board.portal2.sr: {}", err))?
        .profiles)
}

#[derive(Deserialize, Debug)]
struct ActiveProfilesResponse {
    profiles: Vec<String>
}
