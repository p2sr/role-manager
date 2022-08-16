use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use chrono::{Date, DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use serenity::model::user::User;
use tokio::sync::Mutex;
use crate::analyzer::role_definition::{BadgeDefinition, CmLeaderboard, RankRequirement, RecentRequirement, RequirementDefinition, RoleDefinition, TimeRequirement};
use crate::analyzer::user::MetRequirementCause::CmActivity;
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

#[derive(Debug)]
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

#[derive(Debug)]
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
        achieved_on: speedate::Date,
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
    },
    CmActivity {
        steam_id: i64
    }
}

impl Display for MetRequirementCause {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual { assigned_on, note } => {
                match note {
                    Some(note_text) => write!(f, "Assigned {} - {}", assigned_on, note_text),
                    None => write!(f, "Assigned {}", assigned_on)
                }
            }
            Self::FullgameRun { srcom_id, link, rank, time, achieved_on } => {
                write!(f, "[#{} - {} ({})]({})", rank, time, achieved_on, link)
            }
            Self::CmAggregate { steam_id, board } => {
                write!(f, "CM Aggregate {}", board)
            }
            Self::CmRun { steam_id, chapter, chamber, rank, time, achieved_on } => {
                write!(f, "{}/{} - #{} - {} ({})", chapter, chamber, rank, time, achieved_on)
            }
            Self::CmActivity { steam_id } => {
                write!(f, "CM Activity")
            }
        }
    }
}

fn fullgame_cause(user: &UserId, place: LeaderboardPlace) -> Result<MetRequirementCause, RoleManagerError> {
    let date = match place.run.date {
        Some(d) => {
            speedate::Date::parse_str(d.as_str())
                .map_err(|err| RoleManagerError::new(format!("Speedrun.com returned an invalid date format: {} (caused by {})", d, err)))?
        }
        None => {
            speedate::Date::parse_str(Utc::now().date_naive().to_string().as_str())
                .map_err(|err| RoleManagerError::new(format!("Chrono returned an invalid datetime: {:?}", err)))?
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

#[derive(Debug)]
pub struct MetRequirement<'a> {
    pub definition: &'a RequirementDefinition,
    pub cause: MetRequirementCause
}

#[derive(Debug)]
pub struct AnalyzedUserBadge<'a> {
    pub definition: &'a BadgeDefinition,
    pub met_requirements: Vec<MetRequirement<'a>>
}

#[derive(Debug)]
pub struct AnalyzedUser<'a> {
    pub discord_user: User,
    pub external_accounts: Vec<ExternalAccount>,
    pub badges: Vec<AnalyzedUserBadge<'a>>
}

pub async fn analyze_user<'a>(
    discord_user: &User,
    role_definition: &'a RoleDefinition,
    db: &DatabaseConnection,
    srcom_boards: SrComBoardsState,
    cm_boards: CmBoardsState
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
                        RankRequirement::Srcom { game, category, variables, top } => {
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
                                .map_err(|err| RoleManagerError::new(format!("Invalid duration specified in badge {}, {} (caused by {:?})", badge_definition.name, time, err)))?
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
                                        if run.run.times.primary_t <= seconds as f64 {
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
                    for steam_id in &steam_ids {
                        let points_map = cm_boards.fetch_aggregate(leaderboard).await?.points;

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
                                        match &run.run.date {
                                            Some(date_text) => {
                                                if (Utc::now() - Duration::days(30 * 6)).timestamp() < speedate::Date::parse_str(date_text.as_str())
                                                    .map_err(|err| RoleManagerError::new(format!("Speedrun.com provided invalid date: {} (Caused by {:?})", date_text, err)))?
                                                    .timestamp() {
                                                    met_requirements.push(MetRequirement {
                                                        definition: requirement,
                                                        cause: fullgame_cause(srcom, run)?
                                                    });
                                                    break
                                                }
                                            }
                                            None => {}
                                        }
                                    }
                                    None => {}
                                }
                            }
                        }
                        RecentRequirement::Cm { months } => {
                            let active_users = cm_boards.fetch_active_profiles(*months)
                                .await?;

                            for steam_id in &steam_ids {
                                if active_users.contains(&steam_id.to_string()) {
                                    met_requirements.push(MetRequirement {
                                        definition: requirement,
                                        cause: CmActivity {
                                            steam_id: *steam_id
                                        }
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
                RequirementDefinition::Manual => {}
            }
        }

        if met_requirements.len() > 0 {
            analyzed_badges.push(AnalyzedUserBadge {
                definition: badge_definition,
                met_requirements
            });
        }
    }

    // Accumulate info about external accounts (Should be cached now that we've requested LBs)


    Ok(AnalyzedUser {
        discord_user: discord_user.clone(),
        external_accounts: vec![],
        badges: analyzed_badges
    })
}
