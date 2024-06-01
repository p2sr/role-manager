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
use serde::{Deserialize};
use tokio::sync::Mutex;
use tower::limit::RateLimit;
use tower::Service;
use tower::ServiceExt;
use crate::analyzer::role_definition::PartnerRestriction;
use crate::boards::srcom::category::{Category, CategoryId, CategoryOrId};
use crate::boards::srcom::game::{Game, GameId, GameOrId};
use crate::boards::srcom::leaderboard::{Leaderboard, LeaderboardPlace, UserOrGuest};
use crate::boards::srcom::level::LevelId;
use crate::boards::srcom::user::{User, UserId};
use crate::boards::srcom::variable::{Variable, VariableId, VariableValueId};
use crate::error::RoleManagerError;

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum TimingMethod {
    #[serde(rename = "realtime")]
    RealTime,
    #[serde(rename = "realtime_noloads")]
    RealTimeNoLoads,
    #[serde(rename = "ingame")]
    InGame
}

#[derive(Clone, Debug)]
pub struct SrComBoardsState {
    rate_limited_client: Arc<Mutex<RateLimit<Client>>>,

    cache_persist_time: ChronoDuration,
    cached_boards: Arc<Mutex<HashMap<BoardDefinition, CachedBoard>>>,
    cached_games: Arc<Mutex<HashMap<GameId, CachedGame>>>,
    cached_categories: Arc<Mutex<HashMap<CategoryId, CachedCategory>>>,
    cached_users: Arc<Mutex<HashMap<UserId, CachedUser>>>,
    cached_variables: Arc<Mutex<HashMap<VariableId, CachedVariable>>>,
}

impl SrComBoardsState {
    pub fn new(cache_persist_time: ChronoDuration) -> SrComBoardsState {
        let svc = tower::ServiceBuilder::new()
            .rate_limit(100, Duration::from_secs(60))
            .service(Client::new());

        Self {
            rate_limited_client: Arc::new(Mutex::new(svc)),
            cache_persist_time,
            cached_boards: Arc::new(Mutex::new(HashMap::new())),
            cached_games: Arc::new(Mutex::new(HashMap::new())),
            cached_categories: Arc::new(Mutex::new(HashMap::new())),
            cached_users: Arc::new(Mutex::new(HashMap::new())),
            cached_variables: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub async fn fetch_user_highest_run(
        &self,
        user_id: &UserId,
        partner_restriction: &Option<PartnerRestriction>,
        game: GameId,
        category: CategoryId,
        variable_map: BTreeMap<VariableId, VariableValueId>
    ) -> Result<Option<Arc<LeaderboardPlace>>, RoleManagerError> {
        let board_definition = BoardDefinition {
            game,
            category,
            level: None,
            variables: variable_map
        };

        let leaderboard = self.fetch_leaderboard_by_definition(board_definition)
            .await?;

        Ok(leaderboard.get_highest_run(user_id, partner_restriction))
    }

    pub async fn fetch_leaderboard(
        &self,
        game: GameId,
        category: CategoryId
    ) -> Result<Arc<Leaderboard>, RoleManagerError> {
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
    ) -> Result<Arc<Leaderboard>, RoleManagerError> {
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
    ) -> Result<Arc<Leaderboard>, RoleManagerError> {
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
    ) -> Result<Arc<Leaderboard>, RoleManagerError> {
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
    ) -> Result<Arc<Leaderboard>, RoleManagerError> {
        let mut cached_boards = self.cached_boards.lock().await;
        let mut cached_games = self.cached_games.lock().await;
        let mut cached_categories = self.cached_categories.lock().await;
        let mut cached_users = self.cached_users.lock().await;
        let mut cached_variables = self.cached_variables.lock().await;

        match cached_boards.get(&def).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_board) => {
                Ok(Arc::clone(&cached_board.leaderboard))
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

                let mut client = self.rate_limited_client.lock().await;
                let mut request_builder = client.ready().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to obtain ticket for sending requests to speedrun.com: {}", err)))?
                    .get_ref().get(endpoint_url)
                    .query(&[("embed", "game,category,players,variables")]);

                for var_pair in &def.variables {
                    request_builder = request_builder.query(&[(format!("var-{}", var_pair.0.0.clone()).as_str(), var_pair.1.0.clone().as_str())]);
                }

                let response: Response = request_builder.send().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to send request to speedrun.com: {}", err)))?;

                let leaderboard = Arc::new(response.json::<SingleItemRequest<Leaderboard>>()
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to parse leaderboard provided by speedrun.com: {}", err)))?
                    .data);

                cached_boards.insert(def, CachedBoard {
                    leaderboard: Arc::clone(&leaderboard),
                    fetched_at: Utc::now().naive_utc()
                });

                // Cache any embedded information
                if let GameOrId::Game { data } = &leaderboard.game {
                    cached_games.insert(data.id.clone(), CachedGame {
                        game: Arc::new(data.clone()),
                        fetched_at: Utc::now().naive_utc()
                    });
                }
                if let CategoryOrId::Category { data } = &leaderboard.category {
                    cached_categories.insert(data.id.clone(), CachedCategory {
                        category: Arc::new(data.clone()),
                        fetched_at: Utc::now().naive_utc()
                    });
                }
                if let Some(MultipleItemRequest { data }) = &leaderboard.players {
                    for user in data {
                        if let UserOrGuest::User(user) = user {
                            cached_users.insert(user.id.clone(), CachedUser {
                                user: Arc::new(user.clone()),
                                fetched_at: Utc::now().naive_utc()
                            });
                        }
                    }
                }
                if let Some(MultipleItemRequest { data }) = &leaderboard.variables {
                    for var in data {
                        cached_variables.insert(var.id.clone(), CachedVariable {
                            variable: Arc::new(var.clone()),
                            fetched_at: Utc::now().naive_utc()
                        });
                    }
                }

                Ok(leaderboard)
            }
        }
    }

    pub async fn fetch_game(&self, id: GameId) -> Result<Arc<Game>, RoleManagerError> {
        let mut cached_games = self.cached_games.lock().await;

        match cached_games.get(&id).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_game) => {
                Ok(Arc::clone(&cached_game.game))
            }
            None => {
                let endpoint_url = Url::parse(
                    format!("https://www.speedrun.com/api/v1/games/{}",
                            urlencoding::encode(id.0.as_str())
                    ).as_str()
                ).map_err(|err| RoleManagerError::new(format!("Failed to build API request to speedrun.com: {}", err)))?;

                let mut client = self.rate_limited_client.lock().await;

                let response = client.ready().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to obtain ticket for sending requests to speedrun.com: {}", err)))?
                    .call(Request::new(Method::GET, endpoint_url))
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to send request to speedrun.com: {}", err)))?;

                let game = Arc::new(response.json::<SingleItemRequest<Game>>()
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to parse game provided by speedrun.com: {}", err)))?
                    .data);

                cached_games.insert(id.clone(), CachedGame {
                    game: Arc::clone(&game),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(game)
            }
        }
    }

    pub async fn fetch_category(&self, id: CategoryId) -> Result<Arc<Category>, RoleManagerError> {
        let mut cached_categories = self.cached_categories.lock().await;

        match cached_categories.get(&id).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_category) => {
                Ok(Arc::clone(&cached_category.category))
            }
            None => {
                let endpoint_url = Url::parse(
                    format!("https://www.speedrun.com/api/v1/categories/{}",
                        urlencoding::encode(id.0.as_str())
                    ).as_str()
                ).map_err(|err| RoleManagerError::new(format!("Failed to build API request to speedrun.com: {}", err)))?;

                let mut client = self.rate_limited_client.lock().await;

                let response = client.ready().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to obtain ticket for sending requests to speedrun.com: {}", err)))?
                    .call(Request::new(Method::GET, endpoint_url))
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to send request to speedrun.com: {}", err)))?;

                let category = Arc::new(response.json::<SingleItemRequest<Category>>()
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to parse category provided by speedrun.com: {}", err)))?
                    .data);

                cached_categories.insert(id.clone(), CachedCategory {
                    category: Arc::clone(&category),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(category)
            }
        }
    }

    pub async fn fetch_user(&self, id: UserId) -> Result<Arc<User>, RoleManagerError> {
        let mut cached_users = self.cached_users.lock().await;

        match cached_users.get(&id).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_user) => {
                Ok(Arc::clone(&cached_user.user))
            }
            None => {
                let endpoint_url = Url::parse(
                    format!("https://www.speedrun.com/api/v1/users/{}",
                        urlencoding::encode(id.0.as_str())
                    ).as_str()
                ).map_err(|err| RoleManagerError::new(format!("Failed to build API request to speedrun.com: {}", err)))?;

                let mut client = self.rate_limited_client.lock().await;

                let response = client.ready().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to obtain ticket for sending requests to speedrun.com: {}", err)))?
                    .call(Request::new(Method::GET, endpoint_url))
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to send request to speedrun.com: {}", err)))?;

                let user = Arc::new(response.json::<SingleItemRequest<User>>()
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to parse user provided by speedrun.com: {}", err)))?
                    .data);

                cached_users.insert(id.clone(), CachedUser {
                    user: Arc::clone(&user),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(user)
            }
        }
    }

    pub async fn fetch_variable(&self, id: VariableId) -> Result<Arc<Variable>, RoleManagerError> {
        let mut cached_variables = self.cached_variables.lock().await;

        match cached_variables.get(&id).filter(|c| {
            c.fetched_at.checked_add_signed(self.cache_persist_time).map(|t| t > Utc::now().naive_utc()).unwrap_or(false)
        }) {
            Some(cached_variable) => {
                Ok(Arc::clone(&cached_variable.variable))
            }
            None => {
                let endpoint_url = Url::parse(
                    format!("https://www.speedrun.com/api/v1/variables/{}",
                        urlencoding::encode(id.0.as_str())
                    ).as_str()
                ).map_err(|err| RoleManagerError::new(format!("Failed to build API request to speedrun.com: {}", err)))?;

                let mut client = self.rate_limited_client.lock().await;

                let response = client.ready().await
                    .map_err(|err| RoleManagerError::new(format!("Failed to obtain ticket for sending requests to speedrun.com: {}", err)))?
                    .call(Request::new(Method::GET, endpoint_url))
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to send request to speedrun.com: {}", err)))?;

                let variable = Arc::new(response.json::<SingleItemRequest<Variable>>()
                    .await.map_err(|err| RoleManagerError::new(format!("Failed to parse variable provided by speedrun.com: {}", err)))?
                    .data);

                cached_variables.insert(id.clone(), CachedVariable {
                    variable: Arc::clone(&variable),
                    fetched_at: Utc::now().naive_utc()
                });

                Ok(variable)
            }
        }
    }
}

#[derive(Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
struct BoardDefinition {
    game: GameId,
    category: CategoryId,
    level: Option<LevelId>,
    variables: BTreeMap<VariableId, VariableValueId>
}

#[derive(Debug)]
struct CachedBoard {
    leaderboard: Arc<Leaderboard>,
    fetched_at: NaiveDateTime
}

#[derive(Debug)]
struct CachedGame {
    game: Arc<Game>,
    fetched_at: NaiveDateTime
}

#[derive(Debug)]
struct CachedCategory {
    category: Arc<Category>,
    fetched_at: NaiveDateTime
}

#[derive(Debug)]
struct CachedUser {
    user: Arc<User>,
    fetched_at: NaiveDateTime
}

#[derive(Debug)]
struct CachedVariable {
    variable: Arc<Variable>,
    fetched_at: NaiveDateTime
}

#[derive(Deserialize, Debug, Clone)]
pub struct SingleItemRequest<T> {
    data: T
}

#[derive(Deserialize, Debug, Clone)]
pub struct MultipleItemRequest<T> {
    data: Vec<T>
}
