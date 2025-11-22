//! Context menu commands (right-click actions)

use serenity::builder::CreateApplicationCommand;
use serenity::model::application::command::CommandType;

/// Creates context menu commands
pub fn create_commands() -> Vec<CreateApplicationCommand> {
    vec![
        create_analyze_message_context_command(),
        create_explain_message_context_command(),
        create_analyze_user_context_command(),
    ]
}

/// Creates the analyze message context menu command
fn create_analyze_message_context_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("Analyze Message")
        .kind(CommandType::Message)
        .to_owned()
}

/// Creates the explain message context menu command
fn create_explain_message_context_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("Explain Message")
        .kind(CommandType::Message)
        .to_owned()
}

/// Creates the analyze user context menu command
fn create_analyze_user_context_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("Analyze User")
        .kind(CommandType::User)
        .to_owned()
}
