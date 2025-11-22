//! Image generation slash command: /imagine

use serenity::builder::CreateApplicationCommand;
use serenity::model::application::command::CommandOptionType;

/// Creates image generation commands
pub fn create_commands() -> Vec<CreateApplicationCommand> {
    vec![create_imagine_command()]
}

/// Creates the imagine command for DALL-E image generation
fn create_imagine_command() -> CreateApplicationCommand {
    CreateApplicationCommand::default()
        .name("imagine")
        .description("Generate an image using DALL-E 3")
        .create_option(|option| {
            option
                .name("prompt")
                .description("Describe the image you want to generate")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("size")
                .description("Image dimensions (default: square)")
                .kind(CommandOptionType::String)
                .required(false)
                .add_string_choice("Square (1024x1024)", "square")
                .add_string_choice("Landscape (1792x1024)", "landscape")
                .add_string_choice("Portrait (1024x1792)", "portrait")
        })
        .create_option(|option| {
            option
                .name("style")
                .description("Image style (default: vivid)")
                .kind(CommandOptionType::String)
                .required(false)
                .add_string_choice("Vivid - dramatic and hyper-real", "vivid")
                .add_string_choice("Natural - more realistic", "natural")
        })
        .to_owned()
}
