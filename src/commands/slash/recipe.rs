//! Recipe slash command: /recipe

use serenity::builder::CreateApplicationCommand;
use serenity::model::application::command::CommandOptionType;

/// Creates recipe commands
pub fn create_commands() -> Vec<CreateApplicationCommand> {
    vec![create_recipe_command()]
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
