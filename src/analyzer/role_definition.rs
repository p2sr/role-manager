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

#[derive(Deserialize, Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[serde(tag = "leaderboard")]
pub enum CmLeaderboard {
    #[serde(rename = "aggregated/overall")]
    Overall,
    #[serde(rename = "aggregated/sp")]
    SinglePlayer,
    #[serde(rename = "aggregated/coop")]
    Coop
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
