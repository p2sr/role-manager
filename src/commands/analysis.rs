use serenity::builder::CreateApplicationCommands;
use serenity::futures::TryFutureExt;
use serenity::model::application::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::channel::Attachment;
use serenity::model::prelude::application_command::ApplicationCommandInteractionDataOptionValue;
use serenity::prelude::*;
use crate::analyzer::role_definition::RoleDefinition;
use crate::error::RoleManagerError;

pub fn create_command(commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
        command.name("analyze").description("Analyze a skill role definitions file")
            .create_option(|option| {
                option.kind(CommandOptionType::Attachment)
                    .name("definitions")
                    .description("Json5 file describing skill role definitions")
                    .required(true)
            })
    });
}

pub async fn create_analyze_response(ctx: &Context, command: &ApplicationCommandInteraction) -> Result<(), RoleManagerError> {
    println!("Message command: {:?}", command);

    let mut def_file_opt: Option<&Attachment> = None;

    for option in &command.data.options {
        match option.name.as_str() {
            "definitions" => {
                if let ApplicationCommandInteractionDataOptionValue::Attachment(attachment) = &option.resolved.as_ref()
                    .ok_or(RoleManagerError::new(format!("Couldn't resolve skill role definition file parameter")))? {
                    def_file_opt = Some(attachment);
                } else {
                    return Err(format!("Unexpected type for skill role definition file param").into())
                }
            }
            unknown => {
                return Err(format!("Unknown param: {}", unknown).into())
            }
        }
    }

    let definition_file = match def_file_opt {
        Some(definition) => definition,
        None => {
            return Err(format!("Skill role definition file not provided").into())
        }
    };

    // Create response to persist while we download + analyze the given definition file
    command.create_interaction_response(&ctx.http, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|resp_message| {
                resp_message.content(format!("Analyzing definition file..."))
                    .ephemeral(true)
                    .allowed_mentions(|f| f.empty_parse())
            })
    }).await.map_err(|err| format!("Failed to send interaction response: {}", err))?;

    // Download the definition file
    let response = reqwest::get(definition_file.url.clone())
        .await.map_err(|err| RoleManagerError::newEdit(format!("Failed to download provided role definition file: {}", err)))?
        .text().await.map_err(|err| RoleManagerError::newEdit(format!("Failed to interpret provided role definition file download: {}", err)))?;

    let definition: RoleDefinition = json5::from_str(response.as_str())
        .map_err(|err| RoleManagerError::newEdit(format!("Invalid role definition file: {}", err)))?;

    command.edit_original_interaction_response(&ctx.http, |response| {
        response.embed(|embed| {
            embed.title("Role Definition Summary")
        })
    }).await.map_err(|err| RoleManagerError::newEdit(format!("Failed to send interaction response: {}", err)))?;

    Ok(())
}
