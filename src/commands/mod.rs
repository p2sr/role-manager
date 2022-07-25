mod analysis;

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use serenity::builder::CreateApplicationCommands;
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
    Ok(())
}
