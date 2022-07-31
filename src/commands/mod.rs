mod analysis;

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use serenity::builder::CreateApplicationCommands;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::prelude::*;

use crate::error::RoleManagerError;

pub fn create_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    analysis::create_command(commands);

    commands
}

pub async fn create_command_response(
    db: Arc<DatabaseConnection>, ctx: &Context, command: &ApplicationCommandInteraction
) -> Result<(), RoleManagerError> {
    match command.data.name.as_str() {
        "analyze" => {
            analysis::create_analyze_response(ctx, command).await
        }
        _ => {
            command.create_interaction_response(&ctx.http, |response| {
                response.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content("Unknown command")
                        .ephemeral(true))
            }).await.map_err(|err| format!("Failed to respond to command: {}", err).into())
        }
    }
}
