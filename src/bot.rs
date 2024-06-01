use std::sync::Arc;
use std::borrow::Cow;
use std::collections::HashMap;

use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use serenity::model::prelude::*;

use crate::analyzer;
use crate::boards::cm::CmBoardsState;
use crate::boards::srcom::SrComBoardsState;
use crate::error::RoleManagerError;
use crate::analyzer::role_definition::{RequirementDefinition, RoleDefinition};
use crate::analyzer::user::{analyze_user, ExternalAccount};
use crate::config::Config;
use crate::model::lumadb::verified_connections;

#[derive(Debug)]
pub struct BotState {
    pub(crate) db: Arc<DatabaseConnection>,
    pub(crate) srcom_state: SrComBoardsState,
    pub(crate) cm_state: CmBoardsState
}

type PoiseContext<'a> = poise::Context<'a, BotState, RoleManagerError>;

async fn on_error(error: poise::FrameworkError<'_, BotState, RoleManagerError>) {
    match error {
        poise::FrameworkError::Command { error , ctx } => {
            if let Err(err) = ctx.send(|response| {
                response.embed(|embed| embed.title("Failed to execute command").description(format!("{}", error)))
            }).await {
                eprintln!("Sending error response failed: {}", err);
                eprintln!("Caused by: {}", error);
            }
        }
        _ => {
            eprintln!("Experienced generic error: {:#?}", error);
        }
    }
}

pub async fn create_bot(config: Config, db: Arc<DatabaseConnection>, srcom_state: SrComBoardsState, cm_state: CmBoardsState) {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![analyze(), user()],
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .token(config.discord_bot_token.as_str())
        .intents(GatewayIntents::all())
        .user_data_setup(move |ctx,_ready, framework| Box::pin(async move {
            GuildId(299658323500990464).set_application_commands(ctx, |b| {
                *b = poise::builtins::create_application_commands(&framework.options().commands);
                b
            }).await.unwrap();

            GuildId(713630719582404609).set_application_commands(ctx, |b| {
                *b = poise::builtins::create_application_commands(&framework.options().commands);
                b
            }).await.unwrap();

            Ok(BotState { db, srcom_state, cm_state })
        }));

    framework.run_autosharded().await.unwrap();
}


/// Provides a general analysis of a skill role file
#[poise::command(slash_command)]
async fn analyze(
    ctx: PoiseContext<'_>,
    #[description = "Json5 file describing skill role definitions"]
    definition_file: Attachment
) -> Result<(), RoleManagerError> {
    ctx.defer().await?;

    // Download the definition file
    let response = reqwest::get(definition_file.url.clone())
        .await.map_err(|err| RoleManagerError::new_edit(format!("Failed to download provided role definition file: {}", err)))?
        .text().await.map_err(|err| RoleManagerError::new_edit(format!("Failed to interpret provided role definition file download: {}", err)))?;
    let response_str = response.as_str();

    let definition: RoleDefinition = json5::from_str(response_str)
        .map_err(|err| RoleManagerError::new_edit(format!("Invalid role definition file: {}", err)))?;

    // Request relevant (steam,srcom) accounts from database
    let connections: Vec<verified_connections::Model> = verified_connections::Entity::find()
        .filter(verified_connections::Column::Removed.eq(0))
        .all(ctx.data().db.as_ref())
        .await?;

    let mut users: Vec<Member> = Vec::new();
    let mut offset: Option<u64> = None;

    loop {
        let iteration = ctx.discord().http.get_guild_members(146404426746167296, Some(1_000), offset).await?;
        if !iteration.is_empty() {
            offset = Some(iteration.get(iteration.len() - 1).unwrap().user.id.0);
        }

        let end = iteration.len() < 1000;

        users.extend(iteration);

        if end {
            break;
        }
    }

    let (fields, total_users, steam_users, srcom_users) = analyzer::full_analysis(definition, connections, users, ctx.data().srcom_state.clone(), ctx.data().cm_state.clone()).await?;

    ctx.send(|response| {
        response.embed(|embed| {
            let mut builder = embed.title("Role Definition Summary")
                .description(format!("Analyzed **{} Users** ({} CM, {} SRC)", total_users, steam_users, srcom_users))
                .footer(|f| f.text(format!("Context: {}", definition_file.filename)));

            for field in fields {
                builder = builder.field(field.0, field.1, false);
            }

            builder
        }).attachment(AttachmentType::Bytes {
            data: Cow::Owned(response_str.as_bytes().to_vec()),
            filename: definition_file.filename
        })
    }).await?;

    Ok(())
}

/// Provides an analysis of a user under a skill role file
#[poise::command(slash_command)]
pub async fn user(
    ctx: PoiseContext<'_>,
    #[description = "Json5 file describing skill role definitions"]
    definition_file: Attachment,
    #[description = "User to analyze"]
    user: Option<User>
) -> Result<(), RoleManagerError> {
    println!("Deferring response");
    ctx.defer().await?;
    println!("Finished deferring response");

    // Download the definition file
    let response = reqwest::get(definition_file.url.clone())
        .await.map_err(|err| RoleManagerError::new_edit(format!("Failed to download provided role definition file: {}", err)))?
        .text().await.map_err(|err| RoleManagerError::new_edit(format!("Failed to interpret provided role definition file download: {}", err)))?;
    let response_str = response.as_str();

    let definition: RoleDefinition = json5::from_str(response_str)
        .map_err(|err| RoleManagerError::new_edit(format!("Invalid role definition file: {}", err)))?;

    let user = user.as_ref().unwrap_or(ctx.author());

    println!("Analyzing {}#{:04}", user.name, user.discriminator);

    // Request relevant (steam,srcom) accounts from database
    let connections: Vec<verified_connections::Model> = verified_connections::Entity::find()
        .filter(verified_connections::Column::UserId.eq(user.id.0 as i64))
        .filter(verified_connections::Column::Removed.eq(0))
        .all(ctx.data().db.as_ref())
        .await?;

    let analysis = analyze_user(
        user,
        &definition,
        &connections,
        ctx.data().srcom_state.clone(),
        ctx.data().cm_state.clone(),
        true
    ).await?;

    let mut fields: Vec<(String, String)> = Vec::new();
    for badge in &analysis.badges {
        let mut requirement_descs = Vec::new();
        for met_requirement in &badge.met_requirements {
            requirement_descs.push(format!("{}\n - {}", met_requirement.definition.format(ctx.data().srcom_state.clone(), ctx.data().cm_state.clone()).await?, met_requirement.cause));
        }

        fields.push((badge.definition.name.clone(), requirement_descs.join("\n")));
    }

    println!("Completed analysis: {:#?}", &analysis);

    ctx.send(|response| {
        response.embed(|embed| {
            let mut builder = embed.footer(|f| f.text(format!("Context: {}", definition_file.filename)))
                .author(|author| {
                    author.name(&user.name.clone())
                        .icon_url(user.avatar_url().unwrap_or(user.default_avatar_url()))
                });

            let mut account_descs = Vec::new();
            for external_account in analysis.external_accounts {
                match external_account {
                    ExternalAccount::Cm { id, username } => {
                        account_descs.push(format!("- [{} (Steam)](https://board.portal2.sr/profile/{})", username, id));
                    }
                    ExternalAccount::Srcom { username, link, .. } => {
                        account_descs.push(format!("- [{} (Speedrun.com)]({})", username, link));
                    }
                }
            }

            builder = builder.description(format!("**__External Accounts__**\n{}\n**__Badges__**", account_descs.join("\n")));

            for field in fields {
                builder = builder.field(field.0, field.1, false);
            }

            builder
        }).attachment(AttachmentType::Bytes {
            data: Cow::Owned(response_str.as_bytes().to_vec()),
            filename: definition_file.filename
        })
    }).await?;

    Ok(())
}