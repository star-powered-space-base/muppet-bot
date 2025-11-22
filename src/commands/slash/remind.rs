//! Reminder slash commands: /remind, /reminders

use serenity::builder::CreateApplicationCommand;
use serenity::model::application::command::CommandOptionType;

/// Creates reminder commands
pub fn create_commands() -> Vec<CreateApplicationCommand> {
    vec![create_remind_command(), create_reminders_command()]
}

/// Creates the remind command
fn create_remind_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("remind")
        .description("Set a reminder - your persona will remind you later")
        .create_option(|option| {
            option
                .name("time")
                .description("When to remind you (e.g., 30m, 2h, 1d, 1h30m)")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("message")
                .description("What to remind you about")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .to_owned()
}

/// Creates the reminders command
fn create_reminders_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("reminders")
        .description("View or manage your reminders")
        .create_option(|option| {
            option
                .name("action")
                .description("What to do with reminders")
                .kind(CommandOptionType::String)
                .required(false)
                .add_string_choice("list", "list")
                .add_string_choice("cancel", "cancel")
        })
        .create_option(|option| {
            option
                .name("id")
                .description("Reminder ID to cancel (use with 'cancel' action)")
                .kind(CommandOptionType::Integer)
                .required(false)
        })
        .to_owned()
}
