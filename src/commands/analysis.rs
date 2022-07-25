use serenity::builder::CreateApplicationCommands;

pub fn create_command(commands: &mut CreateApplicationCommands) {
    commands.create_application_command(|command| {
        command.name("analyze").description("Analyze a skill role definitions file")
    });
}
