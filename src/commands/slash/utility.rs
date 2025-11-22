//! Utility slash commands: /ping, /help, /forget

use serenity::builder::CreateApplicationCommand;

/// Creates utility commands
pub fn create_commands() -> Vec<CreateApplicationCommand> {
    vec![
        create_ping_command(),
        create_help_command(),
        create_forget_command(),
    ]
}

/// Creates the ping command
fn create_ping_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("ping")
        .description("Test bot responsiveness")
        .to_owned()
}

/// Creates the help command
fn create_help_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("help")
        .description("Show available commands and usage information")
        .to_owned()
}

/// Creates the forget command
fn create_forget_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("forget")
        .description("Clear your conversation history with the bot")
        .to_owned()
}
