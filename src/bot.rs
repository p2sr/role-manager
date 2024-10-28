use std::collections::HashSet;
use std::sync::Arc;
use std::fmt::Write;
use poise::futures_util::StreamExt;
use itertools::Itertools;

use poise::{CreateReply, serenity_prelude as serenity};

use sea_orm::DatabaseConnection;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::ColumnTrait;
use serenity::all::{CreateEmbed, Http};
use serenity::builder::CreateAllowedMentions;
use serenity::model::prelude::*;

use crate::analyzer;
use crate::boards::cm::CmBoardsState;
use crate::boards::srcom::SrComBoardsState;
use crate::error::RoleManagerError;
use crate::analyzer::role_definition::{BadgeDefinition, RoleDefinition};
use crate::analyzer::user;
use crate::analyzer::user::{analyze_user, ExternalAccount};
use crate::config::Config;
use crate::model::lumadb::{manual_role_assignments, verified_connections};
use crate::server::ServerConfig;

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

    let db2 = Arc::clone(&db);
    let srcom_state2 = srcom_state.clone();
    let cm_state2 = cm_state.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![server(), analyze(), user(), generate_report()],
            on_error: |error| Box::pin(on_error(error)),
            ..Default::default()
        })
        .setup(move |ctx,_ready, framework| Box::pin(async move {
            GuildId::new(146404426746167296).set_commands(ctx,
                poise::builtins::create_application_commands(&framework.options().commands)
            ).await.unwrap();

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
    let http = Arc::clone(&client.http);

    tokio::spawn(async move {
        if let Err(e) = client.start().await {
            eprintln!("Bot instance crashed: {}", e);
        }
        std::process::exit(-1);
    });

    // Start a loop updating badges in P2SR every 5 minutes
    tokio::spawn(async move {
        loop {
            let guild_id = GuildId::new(146404426746167296);
            if let Err(e) = update_badge_roles(guild_id, &db2, &http, &srcom_state2, &cm_state2).await {
                eprintln!("Encountered error while updating badge roles:\n{:?}", e);
            }

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    return;
                },
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {}
            }
        }
    }).await?;

    Ok(())
}

async fn update_badge_roles(guild_id: GuildId, db: &DatabaseConnection, client: &Http, srcom_state: &SrComBoardsState, cm_state: &CmBoardsState) -> Result<(), RoleManagerError> {
    println!("Updating badge roles for server {}...", guild_id);

    let server_config = match ServerConfig::read(guild_id.get()).await? {
        Some(config) => config,
        None => {
            eprintln!("Warning: Default server {} doesn't have a set configuration.", guild_id.get());
            return Ok(());
        }
    };
    let definition_path = format!("server_definitions/{}.json5", guild_id.get());

    if tokio::fs::try_exists(&definition_path).await? {
        let definition_content = tokio::fs::read_to_string(&definition_path).await?;
        let definition = json5::from_str(&definition_content)?;
        let valid_badges = server_config.valid_badges(&definition);

        let badge_set: HashSet<&BadgeDefinition> = valid_badges.keys().into_iter()
            .map(|x| *x)
            .filter(|x| x.can_autoremove())
            .collect();

        let connections: Vec<verified_connections::Model> = verified_connections::Entity::find()
            .filter(verified_connections::Column::Removed.eq(0))
            .all(db).await?;

        let manual_assignments: Vec<manual_role_assignments::Model> = manual_role_assignments::Entity::find()
            .all(db).await?;

        let mut members = guild_id.members_iter(client).boxed();
        while let Some(member) = members.next().await {
            let member = member?;

            let analysis = user::analyze_user(
                member.user.id.get(),
                &definition,
                &connections,
                srcom_state.clone(),
                cm_state.clone(),
                false
            ).await?;

            let mut badges_to_analyze = badge_set.clone();

            // Make sure the user has roles that they are supposed to
            for analyzed_badge in analysis.badges {
                match valid_badges.get(&analyzed_badge.definition) {
                    Some(role_id) => {
                        let role_id = RoleId::new(*role_id);

                        if !member.roles.contains(&role_id) {
                            let short_reason = analyzed_badge.met_requirements.iter()
                                .map(|r| r.definition.short_description())
                                .join(", ");

                            println!("Trying to add role {} to user {}", analyzed_badge.definition.name, member.display_name());
                            println!(" - {}", short_reason);

                            if !&server_config.dry_run {
                                client.add_member_role(guild_id, member.user.id, role_id, Some(&short_reason)).await?
                            }
                        }
                    },
                    None => {}
                }
                badges_to_analyze.remove(analyzed_badge.definition);
            }

            // Make sure the user doesn't have roles they're not supposed to
            for badge_definition in badges_to_analyze {
                match valid_badges.get(badge_definition) {
                    Some(role_id) => {
                        let role_id = RoleId::new(*role_id);

                        if member.roles.contains(&role_id) {
                            // See if this role is manually assigned
                            let mut manually_assigned = false;
                            for assignment in &manual_assignments {
                                if (*(&assignment.user_id) as u64) == member.user.id.get() && (*(&assignment.role_id) as u64) == role_id.get() {
                                    manually_assigned = true;
                                    break;
                                }
                            }
                            if !manually_assigned {
                                println!("Trying to remove role {} from user {}", badge_definition.name, member.display_name());

                                if !&server_config.dry_run {
                                    client.remove_member_role(guild_id, member.user.id, role_id, None).await?
                                }
                            }
                        }
                    },
                    None => {}
                }
            }
        }
    }

    Ok(())
}

/// Manage skill roles in this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD", subcommands("redefine", "roles", "refresh", "dryrun"))]
async fn server(_ctx: PoiseContext<'_>) -> Result<(), RoleManagerError> {
    Err(RoleManagerError::new("Impossible state reached, cannot run menu commands".to_string()))
}

/// Manage roles used for badges in this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD", subcommands("list", "add", "remove"))]
async fn roles(_ctx: PoiseContext<'_>) -> Result<(), RoleManagerError> {
    Err(RoleManagerError::new("Impossible state reached, cannot run menu commands".to_string()))
}

/// List Roles and Badges used in this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
async fn list(ctx: PoiseContext<'_>) -> Result<(), RoleManagerError> {
    let response = if let Some(id) = ctx.guild_id() {
        let mut response: String = "Current Badge Roles:\n".to_string();

        if let Some(config) = ServerConfig::read(id.get()).await? {
            for (badge, role_id) in config.badge_roles {
                write!(&mut response, "- **{}** - <@&{}>\n", badge, role_id)?;
            }
        }

        response
    } else {
        "Can only use command on servers!".to_string()
    };

    ctx.send(CreateReply::default()
        .allowed_mentions(CreateAllowedMentions::default().empty_roles().empty_users())
        .content(response)).await?;

    Ok(())
}

/// Add a badge and a corresponding role to give on this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
async fn add(
    ctx: PoiseContext<'_>,
    #[description = "The badge name (as used in Json5 definition files)"]
    badge_name: String,
    #[description = "The role assigned to this badge"]
    role: Role
) -> Result<(), RoleManagerError> {
    let response = if let Some(id) = ctx.guild_id() {
        let mut config = ServerConfig::read(id.get()).await?
            .unwrap_or_default();

        config.badge_roles.insert(badge_name, role.id.get());
        config.write(id.get()).await?;

        "Updated badge roles for this server".to_string()
    } else {
        "Can only use command on servers!".to_string()
    };

    ctx.send(CreateReply::default()
        .allowed_mentions(CreateAllowedMentions::default().empty_roles().empty_users())
        .content(response)).await?;

    Ok(())
}

/// Remove a badge from being given on this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
async fn remove(
    ctx: PoiseContext<'_>,
    #[description = "The badge name (as used in Json5 definition files)"]
    badge_name: String
) -> Result<(), RoleManagerError> {
    let response = if let Some(id) = ctx.guild_id() {
        let mut config = ServerConfig::read(id.get()).await?
            .unwrap_or_default();

        match config.badge_roles.remove(&badge_name) {
            Some(_) => {
                config.write(id.get()).await?;
                "Updated badge roles for this server".to_string()
            },
            None => format!("Badge not found with name `{}` in this server", badge_name)
        }
    } else {
        "Can only use command on servers!".to_string()
    };

    ctx.send(CreateReply::default()
        .allowed_mentions(CreateAllowedMentions::default().empty_roles().empty_users())
        .content(response)).await?;

    Ok(())
}

/// Redefine the skill role definition used for this server
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
async fn redefine(
    ctx: PoiseContext<'_>,
    #[description = "Json5 file describing skill role definitions"]
    definition_file: Attachment
) -> Result<(), RoleManagerError> {
    println!("Deferring response");
    ctx.defer().await?;
    println!("Finished deferring response");

    if definition_file.size > 1_000_000 {
        ctx.reply("Error: Definition files cannot be this large.\n\
        If you wish to use this definition file, please contact the developers.").await?;
        return Ok(());
    }

    if let Some(server_id) = ctx.guild_id() {
        let def_contents = definition_file.download().await?;

        tokio::fs::create_dir_all("server_definitions").await?;
        tokio::fs::write(format!("server_definitions/{}.json5", server_id.get()), def_contents).await?;

        ctx.reply("Updated definitions file for this server!").await?;
    } else {
        ctx.reply("Can only use command on servers!").await?;
    }

    Ok(())
}

/// Manually trigger a role assignments refresh
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
async fn refresh(
    ctx: PoiseContext<'_>
) -> Result<(), RoleManagerError> {
    println!("Deferring response");
    ctx.defer().await?;
    println!("Finished deferring response");

    let response = match ctx.guild_id() {
        Some(guild_id) => {
            update_badge_roles(guild_id, &ctx.data().db, ctx.http(), &(ctx.data().srcom_state.clone()), &(ctx.data().cm_state.clone())).await?;

            "Updated badge in servers".to_string()
        }
        None => {
            "Can only use command on servers!".to_string()
        }
    };

    ctx.reply(response).await?;

    Ok(())
}

/// Set whether the bot's refreshes should perform a "dry run" or actually change roles
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
async fn dryrun(
    ctx: PoiseContext<'_>,
    #[description = "Whether the server should perform \"Dry run\" refreshes instead of normal"]
    enabled: bool
) -> Result<(), RoleManagerError> {
    println!("Deferring response");
    ctx.defer().await?;
    println!("Finished deferring response");

    let response = if let Some(id) = ctx.guild_id() {
        let mut config = ServerConfig::read(id.get()).await?
            .unwrap_or_default();

        config.dry_run = enabled;
        config.write(id.get()).await?;

        "Updated this server's `dryrun` attribute".to_string()
    } else {
        "Can only use command on servers!".to_string()
    };

    ctx.send(CreateReply::default()
        .allowed_mentions(CreateAllowedMentions::default().empty_roles().empty_users())
        .content(response)).await?;

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

/// Generates a CSV file reporting which users satisfy which requirements of a badge
#[poise::command(slash_command)]
async fn generate_report(
    ctx: PoiseContext<'_>,
    #[description = "Badge to analyze"]
    badge_name: String,
    #[description = "Json5 file describing skill role definitions"]
    definition_file: Option<Attachment>,
) -> Result<(), RoleManagerError> {
    println!("Deferring response");
    ctx.defer().await?;
    println!("Finished deferring response");

    let (response_str, definition_filename): (String, String) = match definition_file {
        Some(definition_file) => {
            // Download the definition file
            let response = reqwest::get(definition_file.url.clone())
                .await.map_err(|err| RoleManagerError::new_edit(format!("Failed to download provided role definition file: {}", err)))?
                .text().await.map_err(|err| RoleManagerError::new_edit(format!("Failed to interpret provided role definition file download: {}", err)))?;
            (response.as_str().to_string(), definition_file.filename)
        }
        None => {
            if let Some(guild_id) = ctx.guild_id() {
                // Use this server's definition file
                let definition_path = format!("server_definitions/{}.json5", guild_id.get());

                if tokio::fs::try_exists(&definition_path).await? {
                    (tokio::fs::read_to_string(&definition_path).await?, format!("{}.json5", guild_id.get()))
                } else {
                    ctx.reply("This server doesn't have a definition file set for it! Try attatching one.").await?;
                    return Ok(())
                }
            } else {
                ctx.reply("This command can only be run in servers.").await?;
                return Ok(())
            }
        }
    };

    let definition: RoleDefinition = json5::from_str(&response_str)
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

    // Look up the badge definition and build a header for our sheet with it
    let badge_definition = match definition.badges.iter().find(|badge| badge.name == badge_name) {
        Some(bd) => bd,
        None => {
            ctx.reply(format!("This definition file does not contain a badge `{}`", badge_name)).await?;
            return Ok(())
        }
    };
    let mut header = vec!["Discord User".to_string(), "Num Reqs Satisfied".to_string()];
    for req in badge_definition.requirements.iter() {
        header.push(req.format(ctx.data().srcom_state.clone()).await?);
    }

    // Write a CSV report
    let mut report = csv::Writer::from_writer(vec![]);
    report.write_record(&header).map_err(|e| RoleManagerError::new(format!("Failed to write to report: {}", e)))?;

    let mut users_meeting_requirement = 0;

    for user in &users {
        let analysis = user::analyze_user(
            user.user.id.get(),
            &definition,
            &connections,
            ctx.data().srcom_state.clone(),
            ctx.data().cm_state.clone(),
            false
        ).await?;
        let badge_analysis = match analysis.badges.iter().find(|analyzed_badge| analyzed_badge.definition == badge_definition) {
            Some(badge_analysis) => badge_analysis,
            None => continue
        };

        users_meeting_requirement += 1;

        // Build report row
        let mut row = vec![];
        row.push(user.user.name.clone());

        let mut met_requirements = 0;
        for req in &badge_definition.requirements {
            if badge_analysis.met_requirements.iter().any(|met_req| *met_req.definition == *req) {
                row.push(format!("true"));
                met_requirements += 1;
            } else {
                row.push(format!("false"));
            }
        }

        row.insert(1, format!("{}", met_requirements));

        report.write_record(&row).map_err(|e| RoleManagerError::new(format!("Failed to write to report: {}", e)))?;
    }

    // Send response
    let embed = CreateEmbed::new()
        .description(format!("{}/{} Discord users meet the requirement for badge {}", users_meeting_requirement, users.len(), badge_name))
        .footer(serenity::CreateEmbedFooter::new(format!("Context: {}", definition_filename)));

    let report_inner = report.into_inner()
        .map_err(|e| RoleManagerError::new(format!("Failed to generate report: {}", e)))?;

    ctx.send(poise::CreateReply::default()
        .embed(embed)
        .attachment(serenity::CreateAttachment::bytes(report_inner, format!("{}.csv", badge_name)))
        .attachment(serenity::CreateAttachment::bytes(response_str.as_bytes(), definition_filename))
    ).await?;

    Ok(())
}

/// Provides an analysis of a user under a skill role file
#[poise::command(slash_command)]
pub async fn user(
    ctx: PoiseContext<'_>,
    #[description = "User to analyze"]
    user: Option<User>,
    #[description = "Json5 file describing skill role definitions"]
    definition_file: Option<Attachment>
) -> Result<(), RoleManagerError> {
    println!("Deferring response");
    ctx.defer().await?;
    println!("Finished deferring response");

    let (response_str, definition_filename): (String, String) = match definition_file {
        Some(definition_file) => {
            // Download the definition file
            let response = reqwest::get(definition_file.url.clone())
                .await.map_err(|err| RoleManagerError::new_edit(format!("Failed to download provided role definition file: {}", err)))?
                .text().await.map_err(|err| RoleManagerError::new_edit(format!("Failed to interpret provided role definition file download: {}", err)))?;
            (response.as_str().to_string(), definition_file.filename)
        }
        None => {
            if let Some(guild_id) = ctx.guild_id() {
                // Use this server's definition file
                let definition_path = format!("server_definitions/{}.json5", guild_id.get());

                if tokio::fs::try_exists(&definition_path).await? {
                    (tokio::fs::read_to_string(&definition_path).await?, format!("{}.json5", guild_id.get()))
                } else {
                    ctx.reply("This server doesn't have a definition file set for it! Try attatching one.").await?;
                    return Ok(())
                }
            } else {
                ctx.reply("This command can only be run in servers.").await?;
                return Ok(())
            }
        }
    };

    let definition: RoleDefinition = json5::from_str(&response_str)
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
        user.id.get(),
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

    let color = user.accent_colour.unwrap_or_else(|| Color::DARK_GREY);

    let mut embed = serenity::CreateEmbed::new()
        .footer(serenity::CreateEmbedFooter::new(format!("Context: {}", definition_filename)))
        .color(color)
        .thumbnail(user.avatar_url().unwrap_or(user.default_avatar_url()))
        .author(serenity::CreateEmbedAuthor::new(&user.name).icon_url(user.avatar_url().unwrap_or(user.default_avatar_url())))
        .description(format!("**__External Accounts__**\n{}\n**__Badges__**", account_descs.join("\n")));
    for field in fields {
        embed = embed.field(field.0, field.1, false);
    }

    ctx.send(poise::CreateReply::default()
        .embed(embed)
        .attachment(serenity::CreateAttachment::bytes(response_str.as_bytes(), definition_filename))
    ).await?;

    Ok(())
}