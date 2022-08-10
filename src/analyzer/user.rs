use serenity::model::prelude::*;
use crate::analyzer::role_definition::{BadgeDefinition, RequirementDefinition, RoleDefinition};

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
        assigned_on: Timestamp,
        note: Option<String>
    },
    FullgameRun {
        link: String,
        rank: u32,
        time: String,
        achieved_on: Timestamp
    },
    CmRun {
        chapter: String,
        chamber: String,
        rank: u32,
        time: String,
        achieved_on: Timestamp
    }
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

pub async fn analyze_user(role_definition: &RoleDefinition) -> AnalyzedUser {
    todo!("Read external accounts from DB and analyze to determine badges")
}
