use serde::{Deserialize};

#[derive(Deserialize)]
pub struct RoleDefinition {
    pub badges: Vec<BadgeDefinition>
}

#[derive(Deserialize)]
pub struct BadgeDefinition {
    pub name: String,
    pub requirements: Vec<RequirementDefinition>
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum RequirementDefinition {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "rank")]
    Rank {
        platform: String,
        game: String,
        category: String,
        variables: Option<Vec<VariableDefinition>>
    },
    #[serde(rename = "time")]
    Time {
        platform: String,
        game: String,
        category: String,
        variables: Option<Vec<VariableDefinition>>
    },
    #[serde(rename = "points")]
    Points {
        leaderboard: String,
        points: u64
    },
    #[serde(rename = "recent")]
    Recent {
        platform: String,
        game: Option<String>,
        category: Option<String>,
        variables: Option<Vec<VariableDefinition>>,
        months: u64
    }
}

#[derive(Deserialize)]
pub struct VariableDefinition {
    variable: String,
    choice: String
}
