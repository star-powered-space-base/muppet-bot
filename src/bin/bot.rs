use anyhow::Result;
use log::{error, info};
use openai::set_key;
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::sync::Arc;

use persona::commands::CommandHandler;
use persona::config::Config;
use persona::database::Database;
use persona::message_components::MessageComponentHandler;
use persona::personas::PersonaManager;
use persona::slash_commands::register_global_commands;

struct Handler {
    command_handler: Arc<CommandHandler>,
    component_handler: Arc<MessageComponentHandler>,
}

impl Handler {
    fn new(command_handler: CommandHandler, component_handler: MessageComponentHandler) -> Self {
        Handler {
            command_handler: Arc::new(command_handler),
            component_handler: Arc::new(component_handler),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        if let Err(e) = self.command_handler.handle_message(&ctx, &msg).await {
            error!("Error handling message: {}", e);
            if let Err(why) = msg
                .channel_id
                .say(&ctx.http, "Sorry, I encountered an error processing your message.")
                .await
            {
                error!("Failed to send error message: {}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("🎉 {} is connected and ready!", ready.user.name);
        info!("📡 Connected to {} guilds", ready.guilds.len());
        info!("🔗 Gateway session ID: {:?}", ready.session_id);
        info!("🤖 Bot ID: {}", ready.user.id);
        info!("🌐 Gateway version: {}", ready.version);
        
        // Log shard information
        if let Some(shard) = ready.shard {
            info!("⚡ Shard: {}/{}", shard[0] + 1, shard[1]);
        }
        
        // Register slash commands globally
        if let Err(e) = register_global_commands(&ctx).await {
            error!("❌ Failed to register global slash commands: {}", e);
        } else {
            info!("✅ Successfully registered slash commands globally");
        }
    }



    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::ApplicationCommand(command) => {
                if let Err(e) = self.command_handler.handle_slash_command(&ctx, &command).await {
                    error!("Error handling slash command '{}': {}", command.data.name, e);
                    
                    // Try to edit the deferred response with error message
                    let error_message = if e.to_string().contains("timeout") || e.to_string().contains("OpenAI") {
                        "⏱️ Sorry, the AI service is taking longer than expected. Please try again in a moment."
                    } else {
                        "❌ Sorry, I encountered an error processing your command. Please try again."
                    };
                    
                    // Try to edit the deferred response, fallback to new response if that fails
                    if let Err(_) = command.edit_original_interaction_response(&ctx.http, |response| {
                        response.content(error_message)
                    }).await {
                        let _ = command.create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| {
                                    message.content(error_message)
                                })
                        }).await;
                    }
                }
            }
            Interaction::MessageComponent(component) => {
                if let Err(e) = self.component_handler.handle_component_interaction(&ctx, &component).await {
                    error!("Error handling component interaction '{}': {}", component.data.custom_id, e);
                    
                    let error_message = "❌ Sorry, I encountered an error processing your interaction. Please try again.";
                    
                    // Try to update the message, fallback to new response if that fails
                    if let Err(_) = component.create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(serenity::model::application::interaction::InteractionResponseType::UpdateMessage)
                            .interaction_response_data(|message| {
                                message.content(error_message)
                            })
                    }).await {
                        let _ = component.create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| {
                                    message.content(error_message)
                                })
                        }).await;
                    }
                }
            }
            Interaction::ModalSubmit(modal) => {
                if let Err(e) = self.component_handler.handle_modal_submit(&ctx, &modal).await {
                    error!("Error handling modal submit '{}': {}", modal.data.custom_id, e);
                    
                    let error_message = if e.to_string().contains("timeout") || e.to_string().contains("OpenAI") {
                        "⏱️ Sorry, the AI service is taking longer than expected. Please try again in a moment."
                    } else {
                        "❌ Sorry, I encountered an error processing your submission. Please try again."
                    };
                    
                    // Try to edit the deferred response, fallback to new response if that fails
                    if let Err(_) = modal.edit_original_interaction_response(&ctx.http, |response| {
                        response.content(error_message)
                    }).await {
                        let _ = modal.create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| {
                                    message.content(error_message)
                                })
                        }).await;
                    }
                }
            }
            Interaction::Autocomplete(autocomplete) => {
                info!("Autocomplete interaction received for command: {}", autocomplete.data.name);
                // Handle autocomplete interactions - for now just acknowledge
                let _ = autocomplete
                    .create_autocomplete_response(&ctx.http, |response| {
                        response.add_string_choice("Example suggestion", "example_value")
                    })
                    .await;
            }
            Interaction::Ping(_) => {
                info!("Ping interaction received - Discord health check");
                // Ping interactions are automatically handled by Serenity
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&config.log_level))
        .init();

    info!("Starting Persona Discord Bot...");

    set_key(config.openai_api_key.clone());

    let database = Database::new(&config.database_path).await?;
    let persona_manager = PersonaManager::new();
    let command_handler = CommandHandler::new(database.clone(), config.openai_api_key.clone());
    let component_handler = MessageComponentHandler::new(
        command_handler.clone(),
        persona_manager,
        database.clone()
    );
    let handler = Handler::new(command_handler, component_handler);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Build the Discord client with proper gateway configuration
    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(handler)
        .await
        .map_err(|e| {
            error!("Failed to create Discord client: {}", e);
            error!("This could indicate:");
            error!("  - Invalid bot token format");
            error!("  - Network issues reaching Discord API");
            error!("  - Insufficient permissions");
            anyhow::anyhow!("Client creation failed: {}", e)
        })?;

    info!("Bot configured successfully. Connecting to Discord gateway...");

    // Log gateway connection attempt
    info!("Establishing WebSocket connection to Discord gateway...");
    info!("Gateway intents: {:?}", intents);
    
    if let Err(why) = client.start().await {
        error!("Gateway connection failed: {:?}", why);
        error!("This could be due to:");
        error!("  - Invalid bot token");
        error!("  - Network connectivity issues"); 
        error!("  - Discord API outage");
        error!("  - Missing required permissions");
        return Err(anyhow::anyhow!("Failed to establish gateway connection: {}", why));
    }

    Ok(())
}

