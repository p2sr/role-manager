use std::fmt::{Display, Formatter};
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct RoleDefinition {
    pub badges: Vec<BadgeDefinition>
}

#[derive(Deserialize, Debug)]
pub struct BadgeDefinition {
    pub name: String,
    pub requirements: Vec<RequirementDefinition>
}

#[derive(Deserialize, Debug)]
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

impl Display for RequirementDefinition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => write!(f, "Manual"),
            Self::Rank(RankRequirement::Srcom { game, category, variables, top }) => {
                write!(f, "Speedrun.com - {} - {} - Top {}", game, category, top)
            },
            Self::Time(TimeRequirement::Srcom { game, category, variables, time}) => {
                write!(f, "Speedrun.com - {} - {} - Sub {}", game, category, time)
            },
            Self::Points { leaderboard, points } => {
                write!(f, "CM - {} - {} Points", leaderboard, points)
            },
            Self::Recent(RecentRequirement::Cm { months}) => {
                write!(f, "CM - Activity in last {} months", months)
            }
            Self::Recent(RecentRequirement::Srcom { game, category, variables, months}) => {
                write!(f, "Speedrun.com - {} - {} - Activity in last {} months", game, category, months)
            }
        }
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

#[derive(Deserialize, Debug)]
pub struct VariableDefinition {
    pub variable: String,
    pub choice: String
}

#[derive(Deserialize, Debug)]
pub enum Platform {
    #[serde(rename = "cm")]
    Cm,
    #[serde(rename = "srcom")]
    Srcom
}

#[derive(Deserialize, Debug)]
#[serde(tag = "platform")]
pub enum RecentRequirement {
    #[serde(rename = "srcom")]
    Srcom {
        game: String,
        category: String,
        variables: Option<Vec<VariableDefinition>>,
        months: u64
    },
    #[serde(rename = "cm")]
    Cm {
        months: u64
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "platform")]
pub enum RankRequirement {
    #[serde(rename = "srcom")]
    Srcom {
        game: String,
        category: String,
        variables: Option<Vec<VariableDefinition>>,
        top: u64
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "platform")]
pub enum TimeRequirement {
    #[serde(rename = "srcom")]
    Srcom {
        game: String,
        category: String,
        variables: Option<Vec<VariableDefinition>>,
        time: String
    }
}
