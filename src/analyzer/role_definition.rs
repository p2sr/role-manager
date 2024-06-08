use std::fmt::{Display, Formatter};
use serde::{Deserialize};
use crate::boards::srcom::SrComBoardsState;
use crate::boards::srcom::category::CategoryId;
use crate::boards::srcom::game::GameId;
use crate::boards::srcom::variable::{VariableId, VariableValueId};
use crate::error::RoleManagerError;

#[derive(Deserialize, Debug, Clone)]
pub struct RoleDefinition {
    pub badges: Vec<BadgeDefinition>
}

#[derive(Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct BadgeDefinition {
    pub name: String,
    pub requirements: Vec<RequirementDefinition>
}

impl BadgeDefinition {
    pub fn can_autoremove(&self) -> bool {
        for req in &self.requirements {
            if let RequirementDefinition::Manual = req {
                return false;
            }
        }
        return true;
    }
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
    pub fn short_description(&self) -> String {
        match self {
            Self::Manual => format!("Manual"),
            Self::Rank(RankRequirement::Srcom { game, category, variables, top, partner}) => {
                format!("SRC {} Top {}", game.0, top)
            },
            Self::Time(TimeRequirement::Srcom { game, category, variables, time, partner}) => {
                format!("SRC {} Sub {}", game.0, time)
            },
            Self::Points { leaderboard, points } => {
                format!("{} {}p", leaderboard, points)
            },
            Self::Recent(RecentRequirement::Srcom {game, category, variables, months}) => {
                format!("SRC {} Recent", game.0)
            },
            Self::Recent(RecentRequirement::Cm {months}) => {
                format!("CM Recent")
            }
        }
    }

    pub async fn format(&self, srcom_state: SrComBoardsState) -> Result<String, RoleManagerError> {
        Ok(match self {
            Self::Manual => format!("Manual"),
            Self::Rank(RankRequirement::Srcom { game, category, variables, top, partner }) => {
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

                let restriction = match partner {
                    Some(PartnerRestriction::RankGte) => " (partner.rank>=player.rank)",
                    None => ""
                };

                if variable_descs.is_empty() {
                    format!("SRC - {} - {} - Top {}{}", game.names.international, category.name, top, restriction)
                } else {
                    format!("SRC - {} - {} ({}) - Top {}{}", game.names.international, category.name, variable_descs.join(","), top, restriction)
                }
            },
            Self::Time(TimeRequirement::Srcom { game, category, variables, time, partner}) => {
                let game = srcom_state.fetch_game(game.clone()).await?;
                let category = srcom_state.fetch_category(category.clone()).await?;

                let mut variable_descs = vec![];
                for id_pair in variables.as_ref().unwrap_or(&vec![]) {
                    let variable = srcom_state.fetch_variable(id_pair.variable.clone()).await?;
                    let value = match variable.values.values.get(&id_pair.choice) {
                        Some(value) => value.clone(),
                        None => return Err(RoleManagerError::new(format!("Variable value {} is not a choice for variable {}", id_pair.choice.0, id_pair.variable.0)))
                    };

                    variable_descs.push(format!("{}", value.label));
                }

                let restriction = match partner {
                    Some(PartnerRestriction::RankGte) => " (partner.rank>=player.rank)",
                    None => ""
                };

                if variable_descs.is_empty() {
                    format!("SRC - {} - {} - Sub {}{}", game.names.international, category.name, time, restriction)
                } else {
                    format!("SRC - {} - {} ({}) - Sub {}{}", game.names.international, category.name, variable_descs.join(","), time, restriction)
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

                    variable_descs.push(format!("{}", value.label));
                }

                if variable_descs.is_empty() {
                    format!("SRC - {} - {} - Activity in last {} months", game.names.international, category.name, months)
                } else {
                    format!("SRC - {} - {} ({}) - Activity in last {} months", game.names.international, category.name, variable_descs.join(","), months)
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
            Self::Overall => write!(f, "Overall"),
            Self::SinglePlayer => write!(f, "SP"),
            Self::Coop => write!(f, "Co-op")
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
        partner: Option<PartnerRestriction>,
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
        partner: Option<PartnerRestriction>,
        time: String
    }
}

#[derive(Deserialize, Debug, Clone, Copy, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum PartnerRestriction {
    #[serde(rename = "rank>=")]
    RankGte
}
