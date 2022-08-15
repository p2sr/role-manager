pub mod game;
pub mod leaderboard;
pub mod category;
pub mod level;
pub mod platform;
pub mod region;
pub mod variable;
pub mod run;
pub mod user;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use chrono::{NaiveDateTime, Utc};
use chrono::Duration as ChronoDuration;
use reqwest::{Client, Method, Request, Response, Url};
use serde::Deserialize;
use tokio::sync::Mutex;
use tower::limit::RateLimit;
use tower::Service;
use tower::ServiceExt;
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::game::GameId;
use crate::boards::srcom::leaderboard::Leaderboard;
use crate::boards::srcom::level::LevelId;
use crate::boards::srcom::variable::{VariableId, VariableValueId};
use crate::error::RoleManagerError;

#[derive(Deserialize, Debug)]
pub struct Asset {
    pub uri: String,
    pub width: u32,
    pub height: u32
}

#[derive(Deserialize, Debug, Clone)]
pub struct Link {
    pub rel: String,
    pub uri: String
}

#[derive(Deserialize, Debug, Clone)]
pub enum TimingMethod {
    #[serde(rename = "realtime")]
    RealTime,
    #[serde(rename = "realtime_noloads")]
    RealTimeNoLoads,
    #[serde(rename = "ingame")]
    InGame
}

#[derive(Clone)]
pub struct SrComBoardsState {
    rate_limited_client: Arc<Mutex<RateLimit<Client>>>,

    cache_persist_time: ChronoDuration,
    cached_boards: Arc<Mutex<HashMap<BoardDefinition, CachedBoard>>>
}

pub fn new_boards_state(cache_persist_time: ChronoDuration) -> SrComBoardsState {
    let mut svc = tower::ServiceBuilder::new()
        .rate_limit(100, Duration::from_secs(60))
        .service(Client::new());

    SrComBoardsState {
        rate_limited_client: Arc::new(Mutex::new(svc)),
        cache_persist_time,
        cached_boards: Arc::new(Mutex::new(HashMap::new()))
    }
}

impl SrComBoardsState {
    pub async fn fetch_leaderboard(
        &self,
        game: GameId,
        category: CategoryId
    ) -> Result<Leaderboard, RoleManagerError> {
        self.fetch_leaderboard_by_definition(BoardDefinition {
            game,
            category,
            level: None,
            variables: BTreeMap::new()
        }).await
    }

    pub async fn fetch_leaderboard_with_level(
        &self,
        game: GameId,
        category: CategoryId,
        level: LevelId
    ) -> Result<Leaderboard, RoleManagerError> {
        self.fetch_leaderboard_by_definition(BoardDefinition {
            game,
            category,
            level: Some(level),
            variables: BTreeMap::new()
        }).await
    }

    pub async fn fetch_leaderboard_with_variables(
        &self,
        game: GameId,
        category: CategoryId,
        variables: BTreeMap<VariableId, VariableValueId>
    ) -> Result<Leaderboard, RoleManagerError> {
        self.fetch_leaderboard_by_definition(BoardDefinition {
            game,
            category,
            level: None,
            variables
        }).await
    }

    pub async fn fetch_leaderboard_with_level_and_variables(
        &self,
        game: GameId,
        category: CategoryId,
        level: LevelId,
        variables: BTreeMap<VariableId, VariableValueId>
    ) -> Result<Leaderboard, RoleManagerError> {
        self.fetch_leaderboard_by_definition(BoardDefinition {
            game,
            category,
            level: Some(level),
            variables
        }).await
    }

    async fn fetch_leaderboard_by_definition(
        &self,
        def: BoardDefinition
    ) -> Result<Leaderboard, RoleManagerError> {
        let mut cache = self.cached_boards.lock().await;

        match cache.get(&def).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_board) => {
                Ok(cached_board.leaderboard.clone())
            }
            None => {
                let endpoint_url = match &def.level {
                    Some(level) => Url::parse(
                        format!("https://www.speedrun.com/api/v1/leaderboards/{}/level/{}/{}",
                                urlencoding::encode(&def.game.0.as_str()),
                                urlencoding::encode(level.0.as_str()),
                                urlencoding::encode(&def.category.0.as_str())
                        ).as_str()
                    ),
                    None => Url::parse(
                        format!("https://www.speedrun.com/api/v1/leaderboards/{}/category/{}",
                                urlencoding::encode(&def.game.0.as_str()),
                                urlencoding::encode(&def.category.0.as_str())
                        ).as_str()
                    )
                }.map_err(|err| RoleManagerError::new(format!("Failed to build API request to speedrun.com: {}", err)))?;

                let request = Request::new(Method::GET, endpoint_url);
                let mut client = self.rate_limited_client.lock().await;
                let response: Response = client.ready().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to obtain ticket for sending requests to speedrun.com: {}", err)))?
                    .call(request).await
                    .map_err(|err| RoleManagerError::new(format!("Failed to send request to speedrun.com: {}", err)))?;

                let leaderboard = response.json::<SingleItemRequest<Leaderboard>>()
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to parse leaderboard provided by speedrun.com: {}", err)))?
                    .data;

                cache.insert(def, CachedBoard {
                    leaderboard: leaderboard.clone(),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(leaderboard)
            }
        }
    }
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq)]
struct BoardDefinition {
    game: GameId,
    category: CategoryId,
    level: Option<LevelId>,
    variables: BTreeMap<VariableId, VariableValueId>
}

struct CachedBoard {
    leaderboard: Leaderboard,
    fetched_at: NaiveDateTime
}

#[derive(Deserialize, Debug)]
struct SingleItemRequest<T> {
    data: T
}
