use std::sync::Arc;

use sea_orm::DatabaseConnection;
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::CmBoardsState;
use crate::error::RoleManagerError;
use crate::analyzer::role_definition::RoleDefinition;
use crate::config::Config;

#[derive(Debug)]
pub struct BotState {
    pub(crate) db: Arc<DatabaseConnection>,
    pub(crate) cm_state: Arc<Mutex<CmBoardsState>>
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

pub async fn create_bot(config: Config, db: Arc<DatabaseConnection>, cm_state: Arc<Mutex<CmBoardsState>>) {
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

            Ok(BotState { db, cm_state })
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
        .await.map_err(|err| RoleManagerError::newEdit(format!("Failed to download provided role definition file: {}", err)))?
        .text().await.map_err(|err| RoleManagerError::newEdit(format!("Failed to interpret provided role definition file download: {}", err)))?;

    let definition: RoleDefinition = json5::from_str(response.as_str())
        .map_err(|err| RoleManagerError::newEdit(format!("Invalid role definition file: {}", err)))?;

    println!("Definition: {:#?}", definition);

    ctx.send(|response| {
        response.embed(|embed| {
            embed.title("Role Definition Summary")
                .description(format!("`{}`", definition_file.filename))
                .footer(|f| f.text(format!("Comparing against xyz")))
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
    ctx.defer().await?;

    // Download the definition file
    let response = reqwest::get(definition_file.url.clone())
        .await.map_err(|err| RoleManagerError::newEdit(format!("Failed to download provided role definition file: {}", err)))?
        .text().await.map_err(|err| RoleManagerError::newEdit(format!("Failed to interpret provided role definition file download: {}", err)))?;

    let definition: RoleDefinition = json5::from_str(response.as_str())
        .map_err(|err| RoleManagerError::newEdit(format!("Invalid role definition file: {}", err)))?;

    let name = user.map(|u| u.name).unwrap_or(ctx.author().name.clone());

    ctx.say(format!("{}", name)).await?;

    Ok(())
}