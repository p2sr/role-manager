use std::fmt::{Display, Formatter};
use serde::{Deserialize};
use crate::{CmBoardsState, SrComBoardsState};
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::game::GameId;
use crate::boards::srcom::variable::{VariableId, VariableValueId};
use crate::error::RoleManagerError;

#[derive(Deserialize, Debug)]
pub struct RoleDefinition {
    pub badges: Vec<BadgeDefinition>
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct BadgeDefinition {
    pub name: String,
    pub requirements: Vec<RequirementDefinition>
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[serde(tag = "type")]
pub enum RequirementDefinition {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "rank")]
    Rank(RankRequirement),
    #[serde(rename = "time")]
    Time(TimeRequirement),
    #[serde(rename = "points")]
    Points {
        leaderboard: CmLeaderboard,
        points: u64
    },
    #[serde(rename = "recent")]
    Recent(RecentRequirement)
}

impl RequirementDefinition {
    pub async fn format(&self, srcom_state: SrComBoardsState, cm_state: CmBoardsState) -> Result<String, RoleManagerError> {
        Ok(match self {
            Self::Manual => format!("Manual"),
            Self::Rank(RankRequirement::Srcom { game, category, variables, top }) => {
                let game = srcom_state.fetch_game(game.clone()).await?;
                let category = srcom_state.fetch_category(category.clone()).await?;

                let mut variable_descs = vec![];
                for id_pair in variables.as_ref().unwrap_or(&vec![]) {
                    let variable = srcom_state.fetch_variable(id_pair.variable.clone()).await?;
                    let value = match variable.values.values.get(&id_pair.choice) {
                        Some(value) => value.clone(),
                        None => return Err(RoleManagerError::new(format!("Variable value {} is not a choice for variable {}", id_pair.choice.0, id_pair.variable.0)))
                    };

                    variable_descs.push(format!("{}={}", variable.name, value.label));
                }

                if variable_descs.is_empty() {
                    format!("Speedrun.com - {} - {} - Top {}", game.names.international, category.name, top)
                } else {
                    format!("Speedrun.com - {} - {} ({}) - Top {}", game.names.international, category.name, variable_descs.join(","), top)
                }
            },
            Self::Time(TimeRequirement::Srcom { game, category, variables, time}) => {
                let game = srcom_state.fetch_game(game.clone()).await?;
                let category = srcom_state.fetch_category(category.clone()).await?;

                let mut variable_descs = vec![];
                for id_pair in variables.as_ref().unwrap_or(&vec![]) {
                    let variable = srcom_state.fetch_variable(id_pair.variable.clone()).await?;
                    let value = match variable.values.values.get(&id_pair.choice) {
                        Some(value) => value.clone(),
                        None => return Err(RoleManagerError::new(format!("Variable value {} is not a choice for variable {}", id_pair.choice.0, id_pair.variable.0)))
                    };

                    variable_descs.push(format!("{}={}", variable.name, value.label));
                }

                if variable_descs.is_empty() {
                    format!("Speedrun.com - {} - {} - Sub {}", game.names.international, category.name, time)
                } else {
                    format!("Speedrun.com - {} - {} ({}) - Sub {}", game.names.international, category.name, variable_descs.join(","), time)
                }
            },
            Self::Points { leaderboard, points } => {
                format!("CM - {} - {} Points", leaderboard, points)
            },
            Self::Recent(RecentRequirement::Cm { months}) => {
                format!("CM - Activity in last {} months", months)
            }
            Self::Recent(RecentRequirement::Srcom { game, category, variables, months}) => {
                let game = srcom_state.fetch_game(game.clone()).await?;
                let category = srcom_state.fetch_category(category.clone()).await?;

                let mut variable_descs = vec![];
                for id_pair in variables.as_ref().unwrap_or(&vec![]) {
                    let variable = srcom_state.fetch_variable(id_pair.variable.clone()).await?;
                    let value = match variable.values.values.get(&id_pair.choice) {
                        Some(value) => value.clone(),
                        None => return Err(RoleManagerError::new(format!("Variable value {} is not a choice for variable {}", id_pair.choice.0, id_pair.variable.0)))
                    };

                    variable_descs.push(format!("{}={}", variable.name, value.label));
                }

                if variable_descs.is_empty() {
                    format!("Speedrun.com - {} - {} - Activity in last {} months", game.names.international, category.name, months)
                } else {
                    format!("Speedrun.com - {} - {} ({}) - Activity in last {} months", game.names.international, category.name, variable_descs.join(","), months)
                }
            }
        })
    }
}

#[derive(Deserialize, Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum CmLeaderboard {
    #[serde(rename = "aggregated/overall")]
    Overall,
    #[serde(rename = "aggregated/sp")]
    SinglePlayer,
    #[serde(rename = "aggregated/coop")]
    Coop
}

impl Display for CmLeaderboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overall => write!(f, "aggregated/overall"),
            Self::SinglePlayer => write!(f, "aggregated/sp"),
            Self::Coop => write!(f, "aggregated/coop")
        }
    }
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct VariableDefinition {
    pub variable: VariableId,
    pub choice: VariableValueId
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum Platform {
    #[serde(rename = "cm")]
    Cm,
    #[serde(rename = "srcom")]
    Srcom
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[serde(tag = "platform")]
pub enum RecentRequirement {
    #[serde(rename = "srcom")]
    Srcom {
        game: GameId,
        category: CategoryId,
        variables: Option<Vec<VariableDefinition>>,
        months: u64
    },
    #[serde(rename = "cm")]
    Cm {
        months: u64
    }
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[serde(tag = "platform")]
pub enum RankRequirement {
    #[serde(rename = "srcom")]
    Srcom {
        game: GameId,
        category: CategoryId,
        variables: Option<Vec<VariableDefinition>>,
        top: u64
    }
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[serde(tag = "platform")]
pub enum TimeRequirement {
    #[serde(rename = "srcom")]
    Srcom {
        game: GameId,
        category: CategoryId,
        variables: Option<Vec<VariableDefinition>>,
        time: String
    }
}
