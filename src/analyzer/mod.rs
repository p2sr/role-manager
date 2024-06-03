use std::collections::HashMap;
use serenity::model::guild::Member;
use crate::analyzer::role_definition::{BadgeDefinition, RoleDefinition};
use crate::model::lumadb::verified_connections;
use crate::boards::cm::CmBoardsState;
use crate::boards::srcom::SrComBoardsState;
use crate::error::RoleManagerError;

pub mod role_definition;
pub mod user;

pub struct RoleDefinitionReport {
    definition: RoleDefinition,

    badge_analyses: HashMap<BadgeDefinition, BadgeAnalysis>,

    pub total_users: u64,
    pub steam_users: u64,
    pub srcom_users: u64
}

impl RoleDefinitionReport {
    pub fn new(definition: RoleDefinition) -> Self {
        RoleDefinitionReport {
            definition,
            badge_analyses: HashMap::new(),
            total_users: 0,
            steam_users: 0,
            srcom_users: 0
        }
    }

    pub async fn badge_summary(&self, srcom_state: SrComBoardsState) -> Result<Vec<(String, String)>, RoleManagerError> {
        let mut fields: Vec<(String, String)> = Vec::new();

        for badge in &self.definition.badges {
            let summary = self.badge_analyses.get(badge).unwrap();

            let mut requirement_descs = Vec::new();
            for req in &badge.requirements {
                let req_summary = summary.requirement_counts.get(req).unwrap();

                requirement_descs.push(format!("{} - **{}/{}**", req.format(srcom_state.clone()).await?, req_summary, summary.count))
            }

            fields.push((format!("{} - {}", badge.name, summary.count), requirement_descs.join("\n")))
        }

        Ok(fields)
    }
}

struct BadgeAnalysis {
    count: u32,
    requirement_counts: HashMap<role_definition::RequirementDefinition, u32>
}

pub async fn full_analysis(definition: RoleDefinition,
                           connections: Vec<verified_connections::Model>,
                           users: Vec<Member>,
                           srcom_state: SrComBoardsState,
                           cm_state: CmBoardsState) -> Result<RoleDefinitionReport, RoleManagerError> {
    let mut report = RoleDefinitionReport::new(definition);

    // Set up analysis objects for each badge in the definition file
    for badge in &report.definition.badges {
        let mut reqs = HashMap::new();
        for req in &badge.requirements {
            reqs.insert(req.clone(), 0);
        }

        report.badge_analyses.insert(badge.clone(), BadgeAnalysis {
            count: 0,
            requirement_counts: reqs
        });
    }

    let mut i = 0;
    for user in &users {
        if i % 100 == 0 {
            println!("Analyzing user {}/{}", i, &users.len());
        }
        i += 1;

        // Analyze the user
        let analysis = user::analyze_user(
            &user.user,
            &report.definition,
            &connections,
            srcom_state.clone(),
            cm_state.clone(),
            false
        ).await?;

        // Add to account counts
        report.total_users += 1;
        if analysis.external_accounts.iter()
            .find(|act| matches!(act, user::ExternalAccount::Cm { .. }) )
            .is_some() {
            report.steam_users += 1;
        }
        if analysis.external_accounts.iter()
            .find(|act| matches!(act, user::ExternalAccount::Srcom { .. }))
            .is_some() {
            report.srcom_users += 1;
        }

        // Add to the analyses of each badge this user has
        for badge in &analysis.badges {
            let summary = report.badge_analyses.get_mut(badge.definition).unwrap();
            summary.count += 1;

            for req in &badge.met_requirements {
                let req_summary = summary.requirement_counts.get_mut(req.definition).unwrap();
                *req_summary += 1;
            }
        }
    }


    Ok(report)
}
