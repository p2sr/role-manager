mod analysis;

use serenity::builder::CreateApplicationCommands;
use serenity::prelude::*;

pub fn create_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    analysis::create_command(commands);

    commands
}