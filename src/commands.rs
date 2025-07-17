use crate::audio::AudioTranscriber;
use crate::database::Database;
use crate::message_components::MessageComponentHandler;
use crate::personas::PersonaManager;
use crate::rate_limiter::RateLimiter;
use crate::slash_commands::get_string_option;
use anyhow::Result;
use log::{debug, error, info, warn};
use tokio::time::{timeout, Duration as TokioDuration, Instant};
use uuid::Uuid;
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::time::Duration;

#[derive(Clone)]
pub struct CommandHandler {
    persona_manager: PersonaManager,
    database: Database,
    rate_limiter: RateLimiter,
    audio_transcriber: AudioTranscriber,
    openai_api_key: String,
}

impl CommandHandler {
    pub fn new(database: Database, openai_api_key: String) -> Self {
        CommandHandler {
            persona_manager: PersonaManager::new(),
            database,
            rate_limiter: RateLimiter::new(10, Duration::from_secs(60)), // 10 requests per minute
            audio_transcriber: AudioTranscriber::new(openai_api_key.clone()),
            openai_api_key,
        }
    }

    pub async fn handle_message(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let request_id = Uuid::new_v4();
        let user_id = msg.author.id.to_string();
        let channel_id = msg.channel_id.to_string();
        let guild_id = msg.guild_id.map(|id| id.to_string()).unwrap_or_else(|| "DM".to_string());
        
        info!("[{}] 📥 Message received | User: {} | Channel: {} | Guild: {} | Content: '{}'", 
              request_id, user_id, channel_id, guild_id, 
              msg.content.chars().take(100).collect::<String>());
        
        debug!("[{}] 🔍 Checking rate limit for user: {}", request_id, user_id);
        if !self.rate_limiter.wait_for_rate_limit(&user_id).await {
            warn!("[{}] 🚫 Rate limit exceeded for user: {}", request_id, user_id);
            debug!("[{}] 📤 Sending rate limit message to Discord", request_id);
            msg.channel_id
                .say(&ctx.http, "You're sending messages too quickly! Please slow down.")
                .await?;
            info!("[{}] ✅ Rate limit message sent successfully", request_id);
            return Ok(());
        }
        debug!("[{}] ✅ Rate limit check passed", request_id);

        if !msg.attachments.is_empty() {
            debug!("[{}] 🎵 Processing {} audio attachments", request_id, msg.attachments.len());
            self.handle_audio_attachments(ctx, msg).await?;
        }

        let content = msg.content.trim();
        debug!("[{}] 🔍 Analyzing message content | Length: {} | Starts with command: {}", 
               request_id, content.len(), content.starts_with('!') || content.starts_with('/'));
        
        if content.starts_with('!') || content.starts_with('/') {
            info!("[{}] 🎯 Processing command: {}", request_id, content.split_whitespace().next().unwrap_or(""));
            self.handle_command_with_id(ctx, msg, request_id).await?;
        } else {
            debug!("[{}] ℹ️ Message ignored (not a command)", request_id);
        }

        info!("[{}] ✅ Message processing completed", request_id);
        Ok(())
    }

    pub async fn handle_slash_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let request_id = Uuid::new_v4();
        let user_id = command.user.id.to_string();
        let channel_id = command.channel_id.to_string();
        let guild_id = command.guild_id.map(|id| id.to_string()).unwrap_or_else(|| "DM".to_string());
        
        info!("[{}] 📥 Slash command received | Command: {} | User: {} | Channel: {} | Guild: {}", 
              request_id, command.data.name, user_id, channel_id, guild_id);
        
        debug!("[{}] 🔍 Checking rate limit for user: {}", request_id, user_id);
        if !self.rate_limiter.wait_for_rate_limit(&user_id).await {
            warn!("[{}] 🚫 Rate limit exceeded for user: {} in slash command", request_id, user_id);
            debug!("[{}] 📤 Sending rate limit response to Discord", request_id);
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.content("You're sending commands too quickly! Please slow down.")
                        })
                })
                .await?;
            info!("[{}] ✅ Rate limit response sent successfully", request_id);
            return Ok(());
        }
        debug!("[{}] ✅ Rate limit check passed", request_id);

        info!("[{}] 🎯 Processing slash command: {} from user: {}", request_id, command.data.name, user_id);

        match command.data.name.as_str() {
            "ping" => {
                debug!("[{}] 🏓 Handling ping command", request_id);
                self.handle_slash_ping_with_id(ctx, command, request_id).await?;
            }
            "help" => {
                debug!("[{}] 📚 Handling help command", request_id);
                self.handle_slash_help_with_id(ctx, command, request_id).await?;
            }
            "personas" => {
                debug!("[{}] 🎭 Handling personas command", request_id);
                self.handle_slash_personas_with_id(ctx, command, request_id).await?;
            }
            "set_persona" => {
                debug!("[{}] ⚙️ Handling set_persona command", request_id);
                self.handle_slash_set_persona_with_id(ctx, command, request_id).await?;
            }
            "hey" | "explain" | "simple" | "steps" | "recipe" => {
                debug!("[{}] 🤖 Handling AI command: {}", request_id, command.data.name);
                self.handle_slash_ai_command_with_id(ctx, command, request_id).await?;
            }
            "Analyze Message" | "Explain Message" => {
                debug!("[{}] 🔍 Handling context menu message command: {}", request_id, command.data.name);
                self.handle_context_menu_message_with_id(ctx, command, request_id).await?;
            }
            "Analyze User" => {
                debug!("[{}] 👤 Handling context menu user command", request_id);
                self.handle_context_menu_user_with_id(ctx, command, request_id).await?;
            }
            _ => {
                warn!("[{}] ❓ Unknown slash command: {}", request_id, command.data.name);
                debug!("[{}] 📤 Sending unknown command response to Discord", request_id);
                command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.content("Unknown command. Use `/help` to see available commands.")
                            })
                    })
                    .await?;
                info!("[{}] ✅ Unknown command response sent successfully", request_id);
            }
        }

        info!("[{}] ✅ Slash command processing completed", request_id);
        Ok(())
    }

    async fn handle_command_with_id(&self, ctx: &Context, msg: &Message, request_id: Uuid) -> Result<()> {
        let user_id = msg.author.id.to_string();
        let parts: Vec<&str> = msg.content.split_whitespace().collect();
        
        if parts.is_empty() {
            debug!("[{}] 🔍 Empty command parts array", request_id);
            return Ok(());
        }

        let command = parts[0];
        let args = &parts[1..];

        info!("[{}] 🎯 Processing text command: {} | Args: {} | User: {}", 
              request_id, command, args.len(), user_id);

        match command {
            "!ping" => {
                debug!("[{}] 🏓 Processing ping command", request_id);
                self.database.log_usage(&user_id, "ping", None).await?;
                debug!("[{}] 📤 Sending pong response to Discord", request_id);
                msg.channel_id.say(&ctx.http, "Pong!").await?;
                info!("[{}] ✅ Pong response sent successfully", request_id);
            }
            "/help" => {
                debug!("[{}] 📚 Processing help command", request_id);
                self.handle_help_command_with_id(ctx, msg, request_id).await?;
            }
            "/personas" => {
                debug!("[{}] 🎭 Processing personas command", request_id);
                self.handle_personas_command_with_id(ctx, msg, request_id).await?;
            }
            "/set_persona" => {
                debug!("[{}] ⚙️ Processing set_persona command", request_id);
                self.handle_set_persona_command_with_id(ctx, msg, args, request_id).await?;
            }
            "/hey" | "/explain" | "/simple" | "/steps" | "/recipe" => {
                debug!("[{}] 🤖 Processing AI command: {}", request_id, command);
                self.handle_ai_command_with_id(ctx, msg, command, args, request_id).await?;
            }
            _ => {
                debug!("[{}] ❓ Unknown command: {}", request_id, command);
                debug!("[{}] 📤 Sending unknown command response to Discord", request_id);
                msg.channel_id
                    .say(&ctx.http, "Unknown command. Use `/help` to see available commands.")
                    .await?;
                info!("[{}] ✅ Unknown command response sent successfully", request_id);
            }
        }

        Ok(())
    }

    async fn handle_slash_ping(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let user_id = command.user.id.to_string();
        self.database.log_usage(&user_id, "ping", None).await?;
        
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("Pong!")
                    })
            })
            .await?;
        Ok(())
    }

    async fn handle_slash_help(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let help_text = r#"**Available Slash Commands:**
`/ping` - Test bot responsiveness
`/help` - Show this help message
`/personas` - List available personas
`/set_persona` - Set your default persona
`/hey <message>` - Chat with your current persona
`/explain <topic>` - Get an explanation
`/simple <topic>` - Get a simple explanation with analogies
`/steps <task>` - Break something into steps
`/recipe <food>` - Get a recipe for the specified food

**Available Personas:**
- `muppet` - Muppet expert (default)
- `chef` - Cooking expert
- `teacher` - Patient teacher
- `analyst` - Step-by-step analyst

**Interactive Features:**
Use the buttons below for more help or to try custom prompts!"#;

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .content(help_text)
                            .set_components(MessageComponentHandler::create_help_buttons())
                    })
            })
            .await?;
        Ok(())
    }

    async fn handle_slash_personas(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let personas = self.persona_manager.list_personas();
        let mut response = "**Available Personas:**\n".to_string();
        
        for (name, persona) in personas {
            response.push_str(&format!("• `{}` - {}\n", name, persona.description));
        }
        
        let user_id = command.user.id.to_string();
        let current_persona = self.database.get_user_persona(&user_id).await?;
        response.push_str(&format!("\nYour current persona: `{}`", current_persona));
        response.push_str("\n\n**Quick Switch:**\nUse the dropdown below to change your persona!");
        
        command
            .create_interaction_response(&ctx.http, |response_builder| {
                response_builder
                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .content(response)
                            .set_components(MessageComponentHandler::create_persona_select_menu())
                    })
            })
            .await?;
        Ok(())
    }

    async fn handle_slash_set_persona(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let persona_name = get_string_option(&command.data.options, "persona")
            .ok_or_else(|| anyhow::anyhow!("Missing persona parameter"))?;

        if self.persona_manager.get_persona(&persona_name).is_none() {
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.content("Invalid persona. Use `/personas` to see available options.")
                        })
                })
                .await?;
            return Ok(());
        }

        let user_id = command.user.id.to_string();
        self.database.set_user_persona(&user_id, &persona_name).await?;
        
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content(&format!("Your persona has been set to: `{}`", persona_name))
                    })
            })
            .await?;
        Ok(())
    }

    async fn handle_slash_ai_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        self.handle_slash_ai_command_with_id(ctx, command, Uuid::new_v4()).await
    }

    async fn handle_slash_ai_command_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        let start_time = Instant::now();
        
        debug!("[{}] 🤖 Starting AI slash command processing | Command: {}", request_id, command.data.name);
        
        let option_name = match command.data.name.as_str() {
            "hey" => "message",
            "explain" => "topic",
            "simple" => "topic",
            "steps" => "task",
            "recipe" => "food",
            _ => "message",
        };

        debug!("[{}] 🔍 Extracting option '{}' from command parameters", request_id, option_name);
        let user_message = get_string_option(&command.data.options, option_name)
            .ok_or_else(|| anyhow::anyhow!("Missing message parameter"))?;

        let user_id = command.user.id.to_string();
        debug!("[{}] 👤 Processing for user: {} | Message: '{}'", 
               request_id, user_id, user_message.chars().take(100).collect::<String>());

        debug!("[{}] 🔍 Getting user persona from database", request_id);
        let user_persona = self.database.get_user_persona(&user_id).await?;
        debug!("[{}] 🎭 User persona: {}", request_id, user_persona);
        
        let modifier = match command.data.name.as_str() {
            "explain" => Some("explain"),
            "simple" => Some("simple"),
            "steps" => Some("steps"),
            "recipe" => Some("recipe"),
            _ => None,
        };

        debug!("[{}] 📝 Building system prompt | Persona: {} | Modifier: {:?}", 
               request_id, user_persona, modifier);
        let system_prompt = self.persona_manager.get_system_prompt(&user_persona, modifier);
        debug!("[{}] ✅ System prompt generated | Length: {} chars", request_id, system_prompt.len());

        debug!("[{}] 📊 Logging usage to database", request_id);
        self.database.log_usage(&user_id, &command.data.name, Some(&user_persona)).await?;
        debug!("[{}] ✅ Usage logged successfully", request_id);

        // Immediately defer the interaction to prevent timeout (required within 3 seconds)
        info!("[{}] ⏰ Deferring Discord interaction response (3s rule)", request_id);
        debug!("[{}] 📤 Sending DeferredChannelMessageWithSource to Discord", request_id);
        command
            .create_interaction_response(&ctx.http, |response| {
                response.kind(serenity::model::application::interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await
            .map_err(|e| {
                error!("[{}] ❌ Failed to defer interaction response: {}", request_id, e);
                anyhow::anyhow!("Failed to defer interaction: {}", e)
            })?;
        info!("[{}] ✅ Interaction deferred successfully", request_id);

        // Get AI response and edit the message
        info!("[{}] 🚀 Calling OpenAI API", request_id);
        match self.get_ai_response_with_id(&system_prompt, &user_message, request_id).await {
            Ok(ai_response) => {
                let processing_time = start_time.elapsed();
                info!("[{}] ✅ OpenAI response received | Processing time: {:?} | Response length: {}", 
                      request_id, processing_time, ai_response.len());
                
                if ai_response.len() > 2000 {
                    debug!("[{}] 📄 Response too long, splitting into chunks", request_id);
                    // For long responses, edit with the first part and send follow-ups
                    let chunks: Vec<&str> = ai_response.as_bytes()
                        .chunks(2000)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                        .collect();
                    
                    debug!("[{}] 📄 Split response into {} chunks", request_id, chunks.len());
                    
                    if let Some(first_chunk) = chunks.first() {
                        debug!("[{}] 📤 Editing original interaction response with first chunk ({} chars)", 
                               request_id, first_chunk.len());
                        command
                            .edit_original_interaction_response(&ctx.http, |response| {
                                response.content(first_chunk)
                            })
                            .await
                            .map_err(|e| {
                                error!("[{}] ❌ Failed to edit original interaction response: {}", request_id, e);
                                anyhow::anyhow!("Failed to edit original response: {}", e)
                            })?;
                        info!("[{}] ✅ Original interaction response edited successfully", request_id);
                    }

                    // Send remaining chunks as follow-up messages
                    for (i, chunk) in chunks.iter().skip(1).enumerate() {
                        if !chunk.trim().is_empty() {
                            debug!("[{}] 📤 Sending follow-up message {} of {} ({} chars)", 
                                   request_id, i + 2, chunks.len(), chunk.len());
                            command
                                .create_followup_message(&ctx.http, |message| {
                                    message.content(chunk)
                                })
                                .await
                                .map_err(|e| {
                                    error!("[{}] ❌ Failed to send follow-up message {}: {}", request_id, i + 2, e);
                                    anyhow::anyhow!("Failed to send follow-up message: {}", e)
                                })?;
                            debug!("[{}] ✅ Follow-up message {} sent successfully", request_id, i + 2);
                        }
                    }
                    info!("[{}] ✅ All response chunks sent successfully", request_id);
                } else {
                    debug!("[{}] 📤 Editing original interaction response with complete response ({} chars)", 
                           request_id, ai_response.len());
                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(&ai_response)
                        })
                        .await
                        .map_err(|e| {
                            error!("[{}] ❌ Failed to edit original interaction response: {}", request_id, e);
                            anyhow::anyhow!("Failed to edit original response: {}", e)
                        })?;
                    info!("[{}] ✅ Original interaction response edited successfully", request_id);
                }
                
                let total_time = start_time.elapsed();
                info!("[{}] 🎉 AI command completed successfully | Total time: {:?}", request_id, total_time);
            }
            Err(e) => {
                let processing_time = start_time.elapsed();
                error!("[{}] ❌ OpenAI API error after {:?}: {}", request_id, processing_time, e);
                
                let error_message = if e.to_string().contains("timed out") {
                    debug!("[{}] ⏱️ Error type: timeout", request_id);
                    "⏱️ **Request timed out** - The AI service is taking too long to respond. Please try again with a shorter message or try again later."
                } else if e.to_string().contains("OpenAI API error") {
                    debug!("[{}] 🔧 Error type: OpenAI API error", request_id);
                    "🔧 **AI service error** - There's an issue with the AI service. Please try again in a moment."
                } else {
                    debug!("[{}] ❓ Error type: unknown - {}", request_id, e);
                    "❌ **Error processing request** - Something went wrong. Please try again later."
                };
                
                debug!("[{}] 📤 Sending error message to Discord: '{}'", request_id, error_message);
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(error_message)
                    })
                    .await
                    .map_err(|discord_err| {
                        error!("[{}] ❌ Failed to send error message to Discord: {}", request_id, discord_err);
                        anyhow::anyhow!("Failed to send error response: {}", discord_err)
                    })?;
                info!("[{}] ✅ Error message sent to Discord successfully", request_id);
                
                let total_time = start_time.elapsed();
                error!("[{}] 💥 AI command failed | Total time: {:?}", request_id, total_time);
            }
        }

        Ok(())
    }

    // Placeholder methods with basic logging - can be enhanced later
    async fn handle_slash_ping_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        debug!("[{}] 🏓 Processing ping slash command", request_id);
        self.handle_slash_ping(ctx, command).await
    }

    async fn handle_slash_help_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        debug!("[{}] 📚 Processing help slash command", request_id);
        self.handle_slash_help(ctx, command).await
    }

    async fn handle_slash_personas_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        debug!("[{}] 🎭 Processing personas slash command", request_id);
        self.handle_slash_personas(ctx, command).await
    }

    async fn handle_slash_set_persona_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        debug!("[{}] ⚙️ Processing set_persona slash command", request_id);
        self.handle_slash_set_persona(ctx, command).await
    }

    async fn handle_context_menu_message_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        debug!("[{}] 🔍 Processing context menu message command", request_id);
        self.handle_context_menu_message(ctx, command).await
    }

    async fn handle_context_menu_user_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        debug!("[{}] 👤 Processing context menu user command", request_id);
        self.handle_context_menu_user(ctx, command).await
    }

    async fn handle_help_command_with_id(&self, ctx: &Context, msg: &Message, request_id: Uuid) -> Result<()> {
        debug!("[{}] 📚 Processing help text command", request_id);
        self.handle_help_command(ctx, msg).await
    }

    async fn handle_personas_command_with_id(&self, ctx: &Context, msg: &Message, request_id: Uuid) -> Result<()> {
        debug!("[{}] 🎭 Processing personas text command", request_id);
        self.handle_personas_command(ctx, msg).await
    }

    async fn handle_set_persona_command_with_id(&self, ctx: &Context, msg: &Message, args: &[&str], request_id: Uuid) -> Result<()> {
        debug!("[{}] ⚙️ Processing set_persona text command", request_id);
        self.handle_set_persona_command(ctx, msg, args).await
    }

    async fn handle_ai_command_with_id(&self, ctx: &Context, msg: &Message, command: &str, args: &[&str], request_id: Uuid) -> Result<()> {
        debug!("[{}] 🤖 Processing AI text command: {}", request_id, command);
        self.handle_ai_command(ctx, msg, command, args).await
    }

    async fn handle_context_menu_message(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let user_id = command.user.id.to_string();
        
        // Get the message data from the interaction
        // For now, we'll use a placeholder since resolved data structure varies by version
        let message_content = "Message content will be analyzed".to_string();

        let user_persona = self.database.get_user_persona(&user_id).await?;
        
        let system_prompt = match command.data.name.as_str() {
            "Analyze Message" => {
                self.persona_manager.get_system_prompt(&user_persona, Some("steps"))
            }
            "Explain Message" => {
                self.persona_manager.get_system_prompt(&user_persona, Some("explain"))
            }
            _ => self.persona_manager.get_system_prompt(&user_persona, None)
        };

        let prompt = format!("Please analyze this message: \"{}\"", message_content);
        
        self.database.log_usage(&user_id, &command.data.name, Some(&user_persona)).await?;

        // Immediately defer the interaction to prevent timeout
        command
            .create_interaction_response(&ctx.http, |response| {
                response.kind(serenity::model::application::interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        // Get AI response and edit the message
        match self.get_ai_response(&system_prompt, &prompt).await {
            Ok(ai_response) => {
                let response_text = format!("📝 **{}:**\n{}", command.data.name, ai_response);
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(&response_text)
                    })
                    .await?;
            }
            Err(e) => {
                error!("AI response error in context menu: {}", e);
                let error_message = if e.to_string().contains("timed out") {
                    "⏱️ **Analysis timed out** - The AI service is taking too long. Please try again."
                } else {
                    "❌ **Error analyzing message** - Something went wrong. Please try again later."
                };
                
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(error_message)
                    })
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_context_menu_user(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let user_id = command.user.id.to_string();
        
        // Get the user data from the interaction
        // For now, we'll use a placeholder since resolved data structure varies by version
        let target_user = "Discord User".to_string();

        let user_persona = self.database.get_user_persona(&user_id).await?;
        let system_prompt = self.persona_manager.get_system_prompt(&user_persona, Some("explain"));
        
        let prompt = format!("Please provide general information about Discord users and their roles in communities. The user being analyzed is: {}", target_user);
        
        self.database.log_usage(&user_id, "analyze_user", Some(&user_persona)).await?;

        // Immediately defer the interaction to prevent timeout
        command
            .create_interaction_response(&ctx.http, |response| {
                response.kind(serenity::model::application::interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        // Get AI response and edit the message
        match self.get_ai_response(&system_prompt, &prompt).await {
            Ok(ai_response) => {
                let response_text = format!("👤 **User Analysis:**\n{}", ai_response);
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(&response_text)
                    })
                    .await?;
            }
            Err(e) => {
                error!("AI response error in user context menu: {}", e);
                let error_message = if e.to_string().contains("timed out") {
                    "⏱️ **Analysis timed out** - The AI service is taking too long. Please try again."
                } else {
                    "❌ **Error analyzing user** - Something went wrong. Please try again later."
                };
                
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(error_message)
                    })
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_command(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let user_id = msg.author.id.to_string();
        let parts: Vec<&str> = msg.content.split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(());
        }

        let command = parts[0];
        let args = &parts[1..];

        info!("Processing command: {} from user: {}", command, user_id);

        match command {
            "!ping" => {
                self.database.log_usage(&user_id, "ping", None).await?;
                msg.channel_id.say(&ctx.http, "Pong!").await?;
            }
            "/help" => {
                self.handle_help_command(ctx, msg).await?;
            }
            "/personas" => {
                self.handle_personas_command(ctx, msg).await?;
            }
            "/set_persona" => {
                self.handle_set_persona_command(ctx, msg, args).await?;
            }
            "/hey" | "/explain" | "/simple" | "/steps" | "/recipe" => {
                self.handle_ai_command(ctx, msg, command, args).await?;
            }
            _ => {
                msg.channel_id
                    .say(&ctx.http, "Unknown command. Use `/help` to see available commands.")
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_help_command(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let help_text = r#"**Available Commands:**
`!ping` - Test bot responsiveness
`/help` - Show this help message
`/personas` - List available personas
`/set_persona <name>` - Set your default persona
`/hey <message>` - Chat with your current persona
`/explain <message>` - Get an explanation
`/simple <message>` - Get a simple explanation with analogies
`/steps <message>` - Break something into steps
`/recipe <food>` - Get a recipe for the specified food

**Available Personas:**
- `muppet` - Muppet expert (default)
- `chef` - Cooking expert
- `teacher` - Patient teacher
- `analyst` - Step-by-step analyst"#;

        msg.channel_id.say(&ctx.http, help_text).await?;
        Ok(())
    }

    async fn handle_personas_command(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let personas = self.persona_manager.list_personas();
        let mut response = "**Available Personas:**\n".to_string();
        
        for (name, persona) in personas {
            response.push_str(&format!("• `{}` - {}\n", name, persona.description));
        }
        
        let user_id = msg.author.id.to_string();
        let current_persona = self.database.get_user_persona(&user_id).await?;
        response.push_str(&format!("\nYour current persona: `{}`", current_persona));
        
        msg.channel_id.say(&ctx.http, response).await?;
        Ok(())
    }

    async fn handle_set_persona_command(&self, ctx: &Context, msg: &Message, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Please specify a persona. Use `/personas` to see available options.")
                .await?;
            return Ok(());
        }

        let persona_name = args[0];
        if self.persona_manager.get_persona(persona_name).is_none() {
            msg.channel_id
                .say(&ctx.http, "Invalid persona. Use `/personas` to see available options.")
                .await?;
            return Ok(());
        }

        let user_id = msg.author.id.to_string();
        self.database.set_user_persona(&user_id, persona_name).await?;
        
        msg.channel_id
            .say(&ctx.http, &format!("Your persona has been set to: `{}`", persona_name))
            .await?;
        Ok(())
    }

    async fn handle_ai_command(&self, ctx: &Context, msg: &Message, command: &str, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            msg.channel_id
                .say(&ctx.http, "Please provide a message to process.")
                .await?;
            return Ok(());
        }

        let user_id = msg.author.id.to_string();
        let user_persona = self.database.get_user_persona(&user_id).await?;
        
        let modifier = match command {
            "/explain" => Some("explain"),
            "/simple" => Some("simple"),
            "/steps" => Some("steps"),
            "/recipe" => Some("recipe"),
            _ => None,
        };

        let system_prompt = self.persona_manager.get_system_prompt(&user_persona, modifier);
        let user_message = args.join(" ");

        self.database.log_usage(&user_id, command, Some(&user_persona)).await?;

        match self.get_ai_response(&system_prompt, &user_message).await {
            Ok(response) => {
                if response.len() > 2000 {
                    let chunks: Vec<&str> = response.as_bytes()
                        .chunks(2000)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                        .collect();
                    
                    for chunk in chunks {
                        if !chunk.trim().is_empty() {
                            msg.channel_id.say(&ctx.http, chunk).await?;
                        }
                    }
                } else {
                    msg.channel_id.say(&ctx.http, &response).await?;
                }
            }
            Err(e) => {
                error!("OpenAI API error: {}", e);
                let error_message = if e.to_string().contains("timed out") {
                    "⏱️ **Request timed out** - The AI service is taking too long to respond. Please try again with a shorter message or try again later."
                } else if e.to_string().contains("OpenAI API error") {
                    "🔧 **AI service error** - There's an issue with the AI service. Please try again in a moment."
                } else {
                    "❌ **Error processing request** - Something went wrong. Please try again later."
                };
                
                msg.channel_id.say(&ctx.http, error_message).await?;
            }
        }

        Ok(())
    }

    pub async fn get_ai_response(&self, system_prompt: &str, user_message: &str) -> Result<String> {
        self.get_ai_response_with_id(system_prompt, user_message, Uuid::new_v4()).await
    }

    pub async fn get_ai_response_with_id(&self, system_prompt: &str, user_message: &str, request_id: Uuid) -> Result<String> {
        let start_time = Instant::now();
        
        info!("[{}] 🤖 Starting OpenAI API request | Model: gpt-3.5-turbo", request_id);
        debug!("[{}] 📝 System prompt length: {} chars | User message length: {} chars", 
               request_id, system_prompt.len(), user_message.len());
        debug!("[{}] 📝 User message preview: '{}'", 
               request_id, user_message.chars().take(100).collect::<String>());
        
        // Set the API key for this request
        debug!("[{}] 🔑 Setting OpenAI API key environment variable", request_id);
        std::env::set_var("OPENAI_API_KEY", &self.openai_api_key);
        debug!("[{}] ✅ OpenAI API key set successfully", request_id);
        
        debug!("[{}] 🔨 Building OpenAI message objects", request_id);
        let messages = vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(system_prompt.to_string()),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(user_message.to_string()),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ];
        debug!("[{}] ✅ OpenAI message objects built successfully | Message count: {}", request_id, messages.len());

        // Add timeout to the OpenAI API call (45 seconds)
        debug!("[{}] 🚀 Initiating OpenAI API call with 45-second timeout", request_id);
        let chat_completion_future = ChatCompletion::builder("gpt-3.5-turbo", messages)
            .create();
        
        info!("[{}] ⏰ Waiting for OpenAI API response (timeout: 45s)", request_id);
        let chat_completion = timeout(TokioDuration::from_secs(45), chat_completion_future)
            .await
            .map_err(|_| {
                let elapsed = start_time.elapsed();
                error!("[{}] ⏱️ OpenAI API request timed out after {:?}", request_id, elapsed);
                anyhow::anyhow!("OpenAI API request timed out after 45 seconds")
            })?
            .map_err(|e| {
                let elapsed = start_time.elapsed();
                error!("[{}] ❌ OpenAI API error after {:?}: {}", request_id, elapsed, e);
                anyhow::anyhow!("OpenAI API error: {}", e)
            })?;

        let elapsed = start_time.elapsed();
        info!("[{}] ✅ OpenAI API response received after {:?}", request_id, elapsed);

        debug!("[{}] 🔍 Parsing OpenAI API response", request_id);
        debug!("[{}] 📊 Response choices count: {}", request_id, chat_completion.choices.len());
        
        let response = chat_completion
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| {
                error!("[{}] ❌ No content in OpenAI response", request_id);
                anyhow::anyhow!("No response from OpenAI")
            })?;

        let trimmed_response = response.trim().to_string();
        info!("[{}] ✅ OpenAI response processed | Length: {} chars | First 100 chars: '{}'", 
              request_id, trimmed_response.len(), 
              trimmed_response.chars().take(100).collect::<String>());

        Ok(trimmed_response)
    }

    async fn handle_audio_attachments(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let user_id = msg.author.id.to_string();
        
        for attachment in &msg.attachments {
            if self.is_audio_attachment(&attachment.filename) {
                info!("Processing audio attachment: {}", attachment.filename);
                
                msg.channel_id
                    .say(&ctx.http, "🎵 Transcribing your audio... please wait!")
                    .await?;

                match self
                    .audio_transcriber
                    .download_and_transcribe_attachment(&attachment.url, &attachment.filename)
                    .await
                {
                    Ok(transcription) => {
                        if transcription.trim().is_empty() {
                            msg.channel_id
                                .say(&ctx.http, "I couldn't hear anything in that audio file.")
                                .await?;
                        } else {
                            let response = format!("📝 **Transcription:**\n{}", transcription);
                            
                            if response.len() > 2000 {
                                let chunks: Vec<&str> = response.as_bytes()
                                    .chunks(2000)
                                    .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                                    .collect();
                                
                                for chunk in chunks {
                                    if !chunk.trim().is_empty() {
                                        msg.channel_id.say(&ctx.http, chunk).await?;
                                    }
                                }
                            } else {
                                msg.channel_id.say(&ctx.http, &response).await?;
                            }

                            if !msg.content.trim().is_empty() {
                                let user_persona = self.database.get_user_persona(&user_id).await?;
                                let system_prompt = self.persona_manager.get_system_prompt(&user_persona, None);
                                let combined_message = format!("Based on this transcription: '{}', {}", transcription, msg.content);
                                
                                match self.get_ai_response(&system_prompt, &combined_message).await {
                                    Ok(ai_response) => {
                                        msg.channel_id.say(&ctx.http, &ai_response).await?;
                                    }
                                    Err(e) => {
                                        error!("AI response error: {}", e);
                                    }
                                }
                            }
                        }
                        
                        self.database.log_usage(&user_id, "audio_transcription", None).await?;
                    }
                    Err(e) => {
                        error!("Transcription error: {}", e);
                        msg.channel_id
                            .say(&ctx.http, "Sorry, I couldn't transcribe that audio file. Please make sure it's a valid audio format.")
                            .await?;
                    }
                }
            }
        }
        
        Ok(())
    }

    fn is_audio_attachment(&self, filename: &str) -> bool {
        let audio_extensions = [
            ".mp3", ".wav", ".m4a", ".flac", ".ogg", ".aac", ".wma", ".mp4", ".mov", ".avi"
        ];
        
        let filename_lower = filename.to_lowercase();
        audio_extensions.iter().any(|ext| filename_lower.ends_with(ext))
    }
}