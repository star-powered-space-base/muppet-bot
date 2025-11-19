use anyhow::Result;
use log::info;
use serenity::builder::CreateApplicationCommand;
use serenity::model::application::command::{Command, CommandOptionType, CommandType};
use serenity::model::application::interaction::application_command::CommandDataOption;
use serenity::model::id::GuildId;
use serenity::prelude::Context;

/// Creates all slash command definitions
pub fn create_slash_commands() -> Vec<CreateApplicationCommand> {
    vec![
        create_ping_command(),
        create_help_command(),
        create_personas_command(),
        create_set_persona_command(),
        create_hey_command(),
        create_explain_command(),
        create_simple_command(),
        create_steps_command(),
        create_recipe_command(),
        create_forget_command(),
    ]
}

/// Creates all context menu commands
pub fn create_context_menu_commands() -> Vec<CreateApplicationCommand> {
    vec![
        create_analyze_message_context_command(),
        create_explain_message_context_command(),
        create_analyze_user_context_command(),
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

/// Creates the personas command
fn create_personas_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("personas")
        .description("List all available personas and show your current one")
        .to_owned()
}

/// Creates the set_persona command
fn create_set_persona_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("set_persona")
        .description("Set your default persona")
        .create_option(|option| {
            option
                .name("persona")
                .description("The persona to set as your default")
                .kind(CommandOptionType::String)
                .required(true)
                .add_string_choice("muppet", "muppet")
                .add_string_choice("chef", "chef")
                .add_string_choice("obi", "obi")
                .add_string_choice("teacher", "teacher")
                .add_string_choice("analyst", "analyst")
        })
        .to_owned()
}

/// Creates the hey command
fn create_hey_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("hey")
        .description("Chat with your current persona")
        .create_option(|option| {
            option
                .name("message")
                .description("Your message to the persona")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .to_owned()
}

/// Creates the explain command
fn create_explain_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("explain")
        .description("Get a detailed explanation from your persona")
        .create_option(|option| {
            option
                .name("topic")
                .description("What you want explained")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .to_owned()
}

/// Creates the simple command
fn create_simple_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("simple")
        .description("Get a simple explanation with analogies")
        .create_option(|option| {
            option
                .name("topic")
                .description("What you want explained simply")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .to_owned()
}

/// Creates the steps command
fn create_steps_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("steps")
        .description("Break something down into clear, actionable steps")
        .create_option(|option| {
            option
                .name("task")
                .description("What you want broken down into steps")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .to_owned()
}

/// Creates the recipe command
fn create_recipe_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("recipe")
        .description("Get a recipe for the specified food")
        .create_option(|option| {
            option
                .name("food")
                .description("The food you want a recipe for")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .to_owned()
}

/// Creates the forget command
fn create_forget_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("forget")
        .description("Clear your conversation history with the bot")
        .to_owned()
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

/// Registers all slash commands globally
pub async fn register_global_commands(ctx: &Context) -> Result<()> {
    let slash_commands = create_slash_commands();
    let context_commands = create_context_menu_commands();
    
    for command in slash_commands {
        Command::create_global_application_command(&ctx.http, |c| {
            *c = command;
            c
        })
        .await?;
    }
    
    for command in context_commands {
        Command::create_global_application_command(&ctx.http, |c| {
            *c = command;
            c
        })
        .await?;
    }
    
    info!("Global slash commands and context menu commands registered successfully");
    Ok(())
}

/// Registers all slash commands for a specific guild (faster for testing)
pub async fn register_guild_commands(ctx: &Context, guild_id: GuildId) -> Result<()> {
    let slash_commands = create_slash_commands();
    let context_commands = create_context_menu_commands();
    
    for command in slash_commands {
        guild_id
            .create_application_command(&ctx.http, |c| {
                *c = command;
                c
            })
            .await?;
    }
    
    for command in context_commands {
        guild_id
            .create_application_command(&ctx.http, |c| {
                *c = command;
                c
            })
            .await?;
    }
    
    info!("Guild slash commands and context menu commands registered successfully for guild: {}", guild_id);
    Ok(())
}

/// Utility function to get string option from slash command
pub fn get_string_option(options: &[CommandDataOption], name: &str) -> Option<String> {
    options
        .iter()
        .find(|opt| opt.name == name)
        .and_then(|opt| opt.value.as_ref())
        .and_then(|val| val.as_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_slash_commands() {
        let commands = create_slash_commands();
        assert_eq!(commands.len(), 10);

        // Test that all expected commands are created
        let command_names: Vec<String> = commands
            .iter()
            .map(|cmd| cmd.0.get("name").unwrap().as_str().unwrap().to_string())
            .collect();

        let expected_commands = vec![
            "ping", "help", "personas", "set_persona", "hey",
            "explain", "simple", "steps", "recipe", "forget"
        ];

        for expected in expected_commands {
            assert!(command_names.contains(&expected.to_string()));
        }
    }

    #[test]
    fn test_get_string_option() {
        // This is a simplified test - in practice, CommandDataOption 
        // would be created by Discord and passed to the bot
        let options = vec![];
        
        // Test with empty options
        let no_result = get_string_option(&options, "nonexistent");
        assert_eq!(no_result, None);
        
        // Note: Creating CommandDataOption manually is complex due to 
        // non-exhaustive struct. In real usage, Discord provides these.
    }
}