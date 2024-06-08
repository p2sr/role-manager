use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use chrono::{Duration, NaiveDateTime, Utc};
use crate::analyzer::role_definition::{BadgeDefinition, CmLeaderboard, RankRequirement, RecentRequirement, RequirementDefinition, RoleDefinition, TimeRequirement};
use crate::analyzer::user::MetRequirementCause::CmActivity;
use crate::boards::srcom::leaderboard::LeaderboardPlace;
use crate::boards::srcom::SrComBoardsState;
use crate::boards::srcom::user::UserId;
use crate::boards::srcom::variable::{VariableId, VariableValueId};
use crate::boards::cm::CmBoardsState;
use crate::error::RoleManagerError;
use crate::model::lumadb::verified_connections;

#[derive(Debug)]
pub enum ExternalAccount {
    Srcom {
        username: String,
        id: UserId,
        link: String
    },
    Cm {
        username: String,
        id: i64
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
        board: CmLeaderboard,
        points: u32
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
            Self::FullgameRun { link, rank, time, .. } => {
                write!(f, "[#{} - {}]({})", rank, time, link)
            }
            Self::CmAggregate { steam_id, board, points } => {
                write!(f, "[{} - {}](https://board.portal2.sr/profile/{})", board, points, steam_id)
            }
            Self::CmRun { steam_id: _, chapter, chamber, rank, time, achieved_on } => {
                write!(f, "{}/{} - #{} - {} ({})", chapter, chamber, rank, time, achieved_on)
            }
            Self::CmActivity { steam_id } => {
                write!(f, "[CM Activity](https://board.portal2.sr/profile/{})", steam_id)
            }
        }
    }
}

fn fullgame_cause(user: &UserId, place: &LeaderboardPlace) -> Result<MetRequirementCause, RoleManagerError> {
    let date = match &place.run.date {
        Some(d) => {
            speedate::Date::parse_str(d.as_str())
                .map_err(|err| RoleManagerError::new(format!("Speedrun.com returned an invalid date format: {} (caused by {})", d, err)))?
        }
        None => {
            speedate::Date::parse_str(Utc::now().date_naive().to_string().as_str())
                .map_err(|err| RoleManagerError::new(format!("Chrono returned an invalid datetime: {:?}", err)))?
        }
    };

    let total_seconds = place.run.times.primary_t as u64;
    let hours = total_seconds / (60 * 60);
    let minutes = (total_seconds % (60 * 60)) / 60;
    let seconds = total_seconds % 60;
    let milliseconds = ((place.run.times.primary_t - (total_seconds as f64)) * 1_000.0) as u64;

    let duration = if hours == 0 {
        format!("{}:{:02}.{:03}", minutes, seconds, milliseconds)
    } else {
        format!("{}:{:02}:{:02}.{:03}", hours, minutes, seconds, milliseconds)
    };

    Ok(MetRequirementCause::FullgameRun {
        srcom_id: user.clone(),
        link: (&place.run.weblink).clone(),
        rank: place.place as u32,
        time: duration,
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
    pub discord_id: u64,
    pub external_accounts: Vec<ExternalAccount>,
    pub badges: Vec<AnalyzedUserBadge<'a>>
}

pub async fn analyze_user<'a>(
    discord_id: u64,
    role_definition: &'a RoleDefinition,
    connections: &Vec<verified_connections::Model>,
    srcom_boards: SrComBoardsState,
    cm_boards: CmBoardsState,
    requires_external_details: bool
) -> Result<AnalyzedUser<'a>, RoleManagerError> {
    let mut steam_ids: Vec<i64> = Vec::new();
    let mut srcom_ids = Vec::new();
    for connection in connections {
        if connection.user_id != (discord_id as i64) {
            continue;
        }

        match connection.connection_type.as_str() {
            "steam" => {
                steam_ids.push(connection.id.parse().map_err(|err| RoleManagerError::new(format!("Database contains steam account with invalid ID: {}", err)))?)
            }
            "srcom" => {
                srcom_ids.push(UserId::try_from(connection.id.as_str())?);
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
                        RankRequirement::Srcom { game, category, variables, top, partner } => {
                            let mut variable_map = BTreeMap::new();
                            match variables {
                                Some(v) => {
                                    for var in v {
                                        variable_map.insert(var.variable.clone(), var.choice.clone());
                                    }
                                }
                                None => {}
                            }

                            for srcom in &srcom_ids {
                                match srcom_boards.fetch_user_highest_run(
                                    *srcom,
                                    *partner,
                                    game.clone(),
                                    category.clone(),
                                    variable_map.clone()
                                ).await? {
                                    Some(run) => {
                                        if run.place <= *top {
                                            met_requirements.push(MetRequirement {
                                                definition: requirement,
                                                cause: fullgame_cause(srcom, &run)?
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
                        TimeRequirement::Srcom { game, category, variables, time, partner } => {
                            let seconds = speedate::Duration::parse_str(time.as_str())
                                .map_err(|err| RoleManagerError::new(format!("Invalid duration specified in badge {}, {} (caused by {:?})", badge_definition.name, time, err)))?
                                .signed_total_seconds();

                            let mut variable_map = BTreeMap::new();
                            match variables {
                                Some(v) => {
                                    for var in v {
                                        variable_map.insert(var.variable.clone(), var.choice.clone());
                                    }
                                }
                                None => {}
                            }

                            for srcom in &srcom_ids {
                                match srcom_boards.fetch_user_highest_run(
                                    *srcom,
                                    *partner,
                                    game.clone(),
                                    category.clone(),
                                    variable_map.clone()
                                ).await? {
                                    Some(run) => {
                                        if run.run.times.primary_t <= seconds as f64 {
                                            met_requirements.push(MetRequirement {
                                                definition: requirement,
                                                cause: fullgame_cause(srcom, &run)?
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
                        let aggregate = cm_boards.fetch_aggregate(leaderboard).await?;
                        let points_map = &aggregate.points;

                        match points_map.get(&(steam_id.to_string())) {
                            Some(place) => {
                                if place.score_data.score >= *points as u32 {
                                    met_requirements.push(MetRequirement {
                                        definition: requirement,
                                        cause: MetRequirementCause::CmAggregate {
                                            steam_id: *steam_id,
                                            board: *leaderboard,
                                            points: place.score_data.score
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
                        RecentRequirement::Srcom { game, category, variables, .. } => {
                            let mut variable_map = BTreeMap::new();
                            match variables {
                                Some(v) => {
                                    for var in v {
                                        variable_map.insert(VariableId(var.variable.0.clone()), VariableValueId(var.choice.0.clone()));
                                    }
                                }
                                None => {}
                            }
                            for srcom in &srcom_ids {
                                match srcom_boards.fetch_user_highest_run(
                                    *srcom,
                                    None,
                                    game.clone(),
                                    category.clone(),
                                    variable_map.clone()
                                ).await? {
                                    Some(run) => {
                                        match &run.run.date {
                                            Some(date_text) => {
                                                if (Utc::now() - Duration::days(30 * 6)).timestamp() < speedate::Date::parse_str(date_text.as_str())
                                                    .map_err(|err| RoleManagerError::new(format!("Speedrun.com provided invalid date: {} (Caused by {:?})", date_text, err)))?
                                                    .timestamp() {
                                                    met_requirements.push(MetRequirement {
                                                        definition: requirement,
                                                        cause: fullgame_cause(srcom, &run)?
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
    let mut external_accounts = Vec::new();

    for steam_id in &steam_ids {
        let username = if requires_external_details {
            cm_boards.fetch_profile(*steam_id).await?.board_name.clone().unwrap_or(steam_id.to_string())
        } else {
            steam_id.to_string()
        };

        external_accounts.push(ExternalAccount::Cm {
            id: *steam_id,
            username,
        });
    }
    for srcom_id in &srcom_ids {
        let (username, link) = if requires_external_details {
            let user = srcom_boards.fetch_user(srcom_id.clone()).await?;
            ((&user.names.international).clone(), (&user.weblink).clone())
        } else {
            (format!("{}", srcom_id), format!("{}", srcom_id))
        };

        external_accounts.push(ExternalAccount::Srcom {
            id: *srcom_id,
            username,
            link,
        })
    }


    Ok(AnalyzedUser {
        discord_id,
        external_accounts,
        badges: analyzed_badges
    })
}
