use std::sync::Arc;

use poise::serenity_prelude as serenity;

use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use serenity::all::CreateEmbed;
use serenity::model::prelude::*;

use crate::analyzer;
use crate::boards::cm::CmBoardsState;
use crate::boards::srcom::SrComBoardsState;
use crate::error::RoleManagerError;
use crate::analyzer::role_definition::RoleDefinition;
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
        poise::FrameworkError::Command { error , ctx, .. } => {
            if let Err(err) = ctx.send(
                poise::CreateReply::default()
                    .embed(serenity::CreateEmbed::new()
                            .title("Failed to execute command")
                            .description(format!("{}", error)))
            ).await {
                eprintln!("Sending error response failed: {}", err);
                eprintln!("Caused by: {}", error);
            }
        }
        _ => {
            eprintln!("Experienced generic error: {:#?}", error);
        }
    }
}

pub async fn create_bot(config: Config, db: Arc<DatabaseConnection>, srcom_state: SrComBoardsState, cm_state: CmBoardsState) -> Result<(), RoleManagerError> {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![analyze(), user()],
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(move |ctx,_ready, framework| Box::pin(async move {
            GuildId::new(299658323500990464).set_commands(ctx,
                poise::builtins::create_application_commands(&framework.options().commands)
            ).await.unwrap();

            GuildId::new(713630719582404609).set_commands(ctx,
                poise::builtins::create_application_commands(&framework.options().commands)
            ).await.unwrap();

            Ok(BotState { db, srcom_state, cm_state })
        }))
        .build();

    let mut client = serenity::ClientBuilder::new(config.discord_bot_token.as_str(), GatewayIntents::all())
        .framework(framework)
        .await?;

    client.start().await?;

    // Start a loop updating badges in P2SR every 5 minutes
    /*tokio::spawn(async || {
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {

                }
            }
        }
    });*/

    Ok(())
}


/// Provides a general analysis of a skill role file
#[poise::command(slash_command)]
async fn analyze(
    ctx: PoiseContext<'_>,
    #[description = "Json5 file describing skill role definitions"]
    definition_file: Attachment
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

    // Request relevant (steam,srcom) accounts from database
    let connections: Vec<verified_connections::Model> = verified_connections::Entity::find()
        .filter(verified_connections::Column::Removed.eq(0))
        .all(ctx.data().db.as_ref())
        .await?;

    let mut users: Vec<Member> = Vec::new();
    let mut offset: Option<u64> = None;

    loop {
        let iteration = ctx.http().get_guild_members(GuildId::new(146404426746167296), Some(1_000), offset).await?;
        if !iteration.is_empty() {
            offset = Some(iteration.get(iteration.len() - 1).unwrap().user.id.get());
        }

        let end = iteration.len() < 1000;

        users.extend(iteration);

        if end {
            break;
        }
    }

    let report = analyzer::full_analysis(definition, connections, users, ctx.data().srcom_state.clone(), ctx.data().cm_state.clone()).await?;

    let mut embed = CreateEmbed::new()
        .description(format!("Analyzed **{} Users** ({} CM, {} SRC)", report.total_users, report.steam_users, report.srcom_users))
        .footer(serenity::CreateEmbedFooter::new(format!("Context: {}", definition_file.filename)));
    for field in report.badge_summary(ctx.data().srcom_state.clone()).await? {
        embed = embed.field(field.0, field.1, false);
    }

    ctx.send(poise::CreateReply::default()
        .embed(embed)
        .attachment(serenity::CreateAttachment::bytes(response_str.as_bytes(), definition_file.filename))
    ).await?;

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

    println!("Analyzing {}", user.name);

    // Request relevant (steam,srcom) accounts from database
    let connections: Vec<verified_connections::Model> = verified_connections::Entity::find()
        .filter(verified_connections::Column::UserId.eq(user.id.get() as i64))
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
            requirement_descs.push(format!("{}\n - {}", met_requirement.definition.format(ctx.data().srcom_state.clone()).await?, met_requirement.cause));
        }

        fields.push((badge.definition.name.clone(), requirement_descs.join("\n")));
    }

    println!("Completed analysis: {:#?}", &analysis);

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

    let mut embed = serenity::CreateEmbed::new()
        .footer(serenity::CreateEmbedFooter::new(format!("Context: {}", definition_file.filename)))
        .author(serenity::CreateEmbedAuthor::new(&user.name).icon_url(user.avatar_url().unwrap_or(user.default_avatar_url())))
        .description(format!("**__External Accounts__**\n{}\n**__Badges__**", account_descs.join("\n")));
    for field in fields {
        embed = embed.field(field.0, field.1, false);
    }

    ctx.send(poise::CreateReply::default()
        .embed(embed)
        .attachment(serenity::CreateAttachment::bytes(response_str.as_bytes(), definition_file.filename))
    ).await?;

    Ok(())
}