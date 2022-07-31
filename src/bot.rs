use std::sync::Arc;

use sea_orm::DatabaseConnection;
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::commands;

pub struct BotEventHandler {
    pub(crate) db: Arc<DatabaseConnection>
}

#[async_trait]
impl EventHandler for BotEventHandler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let cmds = GuildId::set_application_commands(
            &GuildId(299658323500990464),
            &ctx.http,
            commands::create_commands
        )
        .await
        .expect("Failed to create application commands");
    }

    async fn message(&self, ctx: Context, message: Message) {
        if let Some(attachment) = message.attachments.get(0) {}
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match &interaction {
            Interaction::ApplicationCommand(command) => {
                if let Err(err) = commands::create_command_response(self.db.clone(), &ctx, command).await {
                    eprintln!("Encountered error while processing application command: {}", err);
                    if err.report_via_edit {
                        if let Err(err) = command.edit_original_interaction_response(&ctx.http, |response| {
                            response.content(format!("Couldn't process request: {}", err.cause))
                        }).await {
                            eprintln!("Encountered error while sending error message: {}", err);
                        }
                    } else {
                        if let Err(err) = command.create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| {
                                    message.content(format!("Couldn't process request: {}", err.cause))
                                        .ephemeral(true)
                                })
                        }).await {
                            eprintln!("Encountered error while sending error message: {}", err);
                        }
                    }
                }
            }
            _ => {
                eprintln!("Unexpected interaction type: {:?}", &interaction.kind());
            }
        }
    }
}
