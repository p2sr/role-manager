use std::collections::HashMap;
use serenity::model::guild::Member;
use crate::model::lumadb::verified_connections;
use crate::boards::cm::CmBoardsState;
use crate::boards::srcom::SrComBoardsState;
use crate::error::RoleManagerError;

pub mod role_definition;
pub mod user;

struct BadgeAnalysis {
    count: u32,
    requirement_counts: HashMap<role_definition::RequirementDefinition, u32>
}

pub async fn full_analysis(definition: role_definition::RoleDefinition,
                           connections: Vec<verified_connections::Model>,
                           users: Vec<Member>,
                           srcom_state: SrComBoardsState,
                           cm_state: CmBoardsState) -> Result<(Vec<(String, String)>, u64, u64, u64), RoleManagerError> {
    let mut badges = HashMap::new();

    for badge in &definition.badges {
        let mut reqs = HashMap::new();
        for req in &badge.requirements {
            reqs.insert(req.clone(), 0);
        }

        badges.insert(badge.clone(), BadgeAnalysis {
            count: 0,
            requirement_counts: reqs
        });
    }

    let mut total_users = 0;
    let mut steam_users = 0;
    let mut srcom_users = 0;

    let mut i = 0;
    for user in &users {
        if i % 100 == 0 {
            println!("Analyzing user {}/{}", i, &users.len());
        }
        i += 1;

        let analysis = user::analyze_user(
            &user.user,
            &definition,
            &connections,
            srcom_state.clone(),
            cm_state.clone(),
            false
        ).await?;

        total_users += 1;
        let mut found_steam = false;
        let mut found_srcom = false;
        for ext in analysis.external_accounts {
            match ext {
                user::ExternalAccount::Cm {..} => {
                    if !found_steam {
                        steam_users += 1;
                        found_steam = true;
                    }
                },
                user::ExternalAccount::Srcom {..} => {
                    if !found_srcom {
                        srcom_users += 1;
                        found_srcom = true;
                    }
                }
            }
        }

        for badge in &analysis.badges {
            let summary = badges.get_mut(badge.definition).unwrap();

            summary.count += 1;

            for req in &badge.met_requirements {
                let req_summary = summary.requirement_counts.get_mut(req.definition).unwrap();

                *req_summary += 1;
            }
        }
    }

    let mut fields: Vec<(String, String)> = Vec::new();
    for badge in &definition.badges {
        let summary = badges.get(badge).unwrap();

        let mut requirement_descs = Vec::new();
        for req in &badge.requirements {
            let req_summary = summary.requirement_counts.get(req).unwrap();

            requirement_descs.push(format!("{} - **{}/{}**", req.format(srcom_state.clone()).await?, req_summary, summary.count))
        }

        fields.push((format!("{} - {}", badge.name, summary.count), requirement_descs.join("\n")))
    }

    Ok((fields, total_users, steam_users, srcom_users))
}
