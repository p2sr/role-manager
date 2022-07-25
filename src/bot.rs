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

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {}
}
