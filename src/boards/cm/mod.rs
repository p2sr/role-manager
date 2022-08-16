mod aggregate;
mod active_profiles;
mod profile;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{Duration as ChronoDuration, NaiveDateTime, Utc};

use serde::Deserialize;
use tokio::sync::Mutex;
use crate::analyzer::role_definition::CmLeaderboard;
use crate::boards::cm::active_profiles::CachedActiveProfiles;
use crate::boards::cm::aggregate::{AggregatedResponse, CachedAggregate};
use crate::boards::cm::profile::{CachedProfile, Profile};
use crate::error::RoleManagerError;

#[derive(Debug, Clone)]
pub struct CmBoardsState {
    cache_persist_time: ChronoDuration,

    cached_aggregates: Arc<Mutex<HashMap<CmLeaderboard, CachedAggregate>>>,
    cached_active_profiles: Arc<Mutex<HashMap<u64, CachedActiveProfiles>>>,
    cached_profiles: Arc<Mutex<HashMap<i64, CachedProfile>>>
}

impl CmBoardsState {
    pub fn new(cache_persist_time: ChronoDuration) -> Self {
        CmBoardsState {
            cache_persist_time,

            cached_aggregates: Arc::new(Mutex::new(HashMap::new())),
            cached_active_profiles: Arc::new(Mutex::new(HashMap::new())),
            cached_profiles: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub async fn fetch_aggregate(&self, leaderboard: &CmLeaderboard) -> Result<AggregatedResponse, RoleManagerError> {
        let mut cache = self.cached_aggregates.lock().await;
        let mut cached_profiles = self.cached_profiles.lock().await;

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

                let aggregate = aggregate::fetch_aggregate(page).await?;

                cache.insert(*leaderboard, CachedAggregate {
                    aggregate: aggregate.clone(),
                    fetched_at: Utc::now().naive_utc()
                });

                for pair in &aggregate.points {
                    cached_profiles.insert(pair.0.parse()
                                               .map_err(|err| format!("CM Boards provided invalid steam id: {}", err))?,
                                           CachedProfile {
                                               profile: pair.1.user_data.clone(),
                                               fetched_at: Utc::now().naive_utc()
                                           });
                }

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
                let profiles = active_profiles::fetch_active_profiles(months).await?;

                cache.insert(months, CachedActiveProfiles {
                    active_profiles: profiles.clone(),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(profiles)
            }
        }
    }

    pub async fn fetch_profile(&self, id: i64) -> Result<Profile, RoleManagerError> {
        let mut cache = self.cached_profiles.lock().await;

        match cache.get(&id).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_profile) => {
                Ok(cached_profile.profile.clone())
            }
            None => {
                let profile = profile::fetch_profile(id).await?;

                cache.insert(id, CachedProfile {
                    profile: profile.clone(),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(profile)
            }
        }
    }
}

