use std::collections::BTreeMap;
use std::sync::Arc;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use serenity::model::user::User;
use tokio::sync::Mutex;
use crate::analyzer::role_definition::{BadgeDefinition, CmLeaderboard, RankRequirement, RecentRequirement, RequirementDefinition, RoleDefinition, TimeRequirement};
use crate::boards::srcom;
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::game::GameId;
use crate::boards::srcom::leaderboard::LeaderboardPlace;
use crate::boards::srcom::SrComBoardsState;
use crate::boards::srcom::user::UserId;
use crate::boards::srcom::variable::{VariableId, VariableValueId};
use crate::CmBoardsState;
use crate::error::RoleManagerError;
use crate::model::lumadb::verified_connections;

pub enum ExternalAccount {
    Srcom {
        username: String,
        id: String
    },
    Cm {
        username: String,
        id: String
    }
}

pub enum MetRequirementCause {
    Manual {
        assigned_on: NaiveDateTime,
        note: Option<String>
    },
    FullgameRun {
        srcom_id: UserId,
        link: String,
        rank: u32,
        time: String,
        achieved_on: NaiveDateTime,
    },
    CmAggregate {
        steam_id: i64,
        board: CmLeaderboard
    },
    CmRun {
        steam_id: i64,
        chapter: String,
        chamber: String,
        rank: u32,
        time: String,
        achieved_on: NaiveDateTime
    }
}

fn fullgame_cause(user: &UserId, place: LeaderboardPlace) -> Result<MetRequirementCause, RoleManagerError> {
    let date = match place.run.date {
        Some(d) => {
            DateTime::parse_from_rfc3339(d.as_str())
                .map_err(|err| RoleManagerError::new(format!("Speedrun.com returned an invalid date format: {}", err)))?
                .naive_utc()
        }
        None => {
            Utc::now().naive_utc()
        }
    };

    Ok(MetRequirementCause::FullgameRun {
        srcom_id: user.clone(),
        link: place.run.weblink,
        rank: place.place as u32,
        time: place.run.times.primary,
        achieved_on: date
    })
}

pub struct MetRequirement<'a> {
    pub definition: &'a RequirementDefinition,
    pub cause: MetRequirementCause
}

pub struct AnalyzedUserBadge<'a> {
    pub definition: &'a BadgeDefinition,
    pub met_requirements: Vec<MetRequirement<'a>>
}

pub struct AnalyzedUser<'a> {
    pub discord_user: User,
    pub external_accounts: Vec<ExternalAccount>,
    pub badges: Vec<AnalyzedUserBadge<'a>>
}

pub async fn analyze_user<'a>(
    discord_user: User,
    role_definition: &'a RoleDefinition,
    db: &DatabaseConnection,
    srcom_boards: SrComBoardsState,
    cm_boards: Arc<Mutex<CmBoardsState>>
) -> Result<AnalyzedUser<'a>, RoleManagerError> {
    // Request relevant (steam,srcom) accounts from database
    let connections: Vec<verified_connections::Model> = verified_connections::Entity::find()
        .filter(verified_connections::Column::UserId.eq(discord_user.id.0 as i64))
        .filter(verified_connections::Column::Removed.eq(0))
        .all(db)
        .await?;

    let mut steam_ids: Vec<i64> = Vec::new();
    let mut srcom_ids = Vec::new();
    for connection in connections {
        match connection.connection_type.as_str() {
            "steam" => {
                steam_ids.push(connection.id.parse().map_err(|err| RoleManagerError::new(format!("Database contains steam account with invalid ID: {}", err)))?)
            }
            "srcom" => {
                srcom_ids.push(UserId(connection.id));
            }
            _ => {}
        }
    }

    // Process each badge
    let mut analyzed_badges: Vec<AnalyzedUserBadge> = Vec::new();

    for badge_definition in &(role_definition.badges) {
        let mut met_requirements: Vec<MetRequirement> = Vec::new();

        for requirement in &badge_definition.requirements {
            match requirement {
                RequirementDefinition::Rank(req) => {
                    match req {
                        RankRequirement::Srcom { game, category, variables, top} => {
                            let mut variable_map = BTreeMap::new();
                            match variables {
                                Some(v) => {
                                    for var in v {
                                        variable_map.insert(VariableId(var.variable.clone()), VariableValueId(var.choice.clone()));
                                    }
                                }
                                None => {}
                            }
                            let leaderboard = srcom_boards.fetch_leaderboard_with_variables(GameId(game.clone()), CategoryId(category.clone()), variable_map).await?;

                            for srcom in &srcom_ids {
                                match leaderboard.get_highest_run(&srcom) {
                                    Some(run) => {
                                        if run.place <= *top {
                                            met_requirements.push(MetRequirement {
                                                definition: requirement,
                                                cause: fullgame_cause(srcom, run)?
                                            });
                                            break;
                                        }
                                    }
                                    None => {}
                                }
                            }
                        }
                    }
                }
                RequirementDefinition::Time(req) => {
                    match req {
                        TimeRequirement::Srcom { game, category, variables, time } => {
                            let seconds = speedate::Duration::parse_str(time.as_str())
                                .map_err(|err| RoleManagerError::new(format!("Invalid date specified in badge {}, {} (caused by {:?})", badge_definition.name, time, err)))?
                                .signed_total_seconds();

                            let mut variable_map = BTreeMap::new();
                            match variables {
                                Some(v) => {
                                    for var in v {
                                        variable_map.insert(VariableId(var.variable.clone()), VariableValueId(var.choice.clone()));
                                    }
                                }
                                None => {}
                            }
                            let leaderboard = srcom_boards.fetch_leaderboard_with_variables(GameId(game.clone()), CategoryId(category.clone()), variable_map).await?;

                            for srcom in &srcom_ids {
                                match leaderboard.get_highest_run(&srcom) {
                                    Some(run) => {
                                        if run.run.times.primary_t <= seconds as u64 {
                                            met_requirements.push(MetRequirement {
                                                definition: requirement,
                                                cause: fullgame_cause(srcom, run)?
                                            });
                                            break;
                                        }
                                    }
                                    None => {}
                                }
                            }
                        }
                    }
                }
                RequirementDefinition::Points { leaderboard, points } => {
                    let boards = cm_boards.lock().await;

                    for steam_id in &steam_ids {
                        let points_map = match leaderboard {
                            CmLeaderboard::Overall => {
                                &boards.overall.points
                            }
                            CmLeaderboard::SinglePlayer => {
                                &boards.sp.points
                            }
                            CmLeaderboard::Coop => {
                                &boards.coop.points
                            }
                        };

                        match points_map.get(&(steam_id.to_string())) {
                            Some(place) => {
                                if place.score_data.score >= *points as u32 {
                                    met_requirements.push(MetRequirement {
                                        definition: requirement,
                                        cause: MetRequirementCause::CmAggregate {
                                            steam_id: *steam_id,
                                            board: *leaderboard
                                        }
                                    });
                                    break;
                                }
                            }
                            None => {}
                        }
                    }
                }
                RequirementDefinition::Recent(recent) => {
                    match recent {
                        RecentRequirement::Srcom { game, category, variables, months } => {

                        }
                        RecentRequirement::Cm { months } => {

                        }
                    }
                }
                RequirementDefinition::Manual => {}
            }
        }

        analyzed_badges.push(AnalyzedUserBadge {
            definition: badge_definition,
            met_requirements
        });
    }

    // Accumulate info about external accounts (Should be cached now that we've requested LBs)


    Ok(AnalyzedUser {
        discord_user,
        external_accounts: vec![],
        badges: analyzed_badges
    })
}
