use crate::audio::AudioTranscriber;
use crate::conflict_detector::ConflictDetector;
use crate::conflict_mediator::ConflictMediator;
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
    openai_model: String,
    conflict_detector: ConflictDetector,
    conflict_mediator: ConflictMediator,
    conflict_enabled: bool,
    conflict_sensitivity_threshold: f32,
}

impl CommandHandler {
    pub fn new(
        database: Database,
        openai_api_key: String,
        openai_model: String,
        conflict_enabled: bool,
        conflict_sensitivity: &str,
        mediation_cooldown_minutes: u64,
    ) -> Self {
        // Map sensitivity to threshold
        let sensitivity_threshold = match conflict_sensitivity.to_lowercase().as_str() {
            "low" => 0.7,      // Only very high confidence conflicts
            "high" => 0.35,    // More sensitive - catches single keywords + context
            "ultra" => 0.3,    // Maximum sensitivity - triggers on single hostile keyword
            _ => 0.5,          // Medium (default)
        };

        CommandHandler {
            persona_manager: PersonaManager::new(),
            database,
            rate_limiter: RateLimiter::new(10, Duration::from_secs(60)),
            audio_transcriber: AudioTranscriber::new(openai_api_key.clone()),
            openai_api_key,
            openai_model,
            conflict_detector: ConflictDetector::new(),
            conflict_mediator: ConflictMediator::new(999, mediation_cooldown_minutes), // High limit for testing
            conflict_enabled,
            conflict_sensitivity_threshold: sensitivity_threshold,
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
        let is_dm = msg.guild_id.is_none();
        debug!("[{}] 🔍 Analyzing message content | Length: {} | Is DM: {} | Starts with command: {}",
               request_id, content.len(), is_dm, content.starts_with('!') || content.starts_with('/'));

        // Store guild messages FIRST (needed for conflict detection to have data)
        if !is_dm && !content.is_empty() && !content.starts_with('!') && !content.starts_with('/') {
            debug!("[{}] 💾 Storing guild message for analysis", request_id);
            self.database.store_message(&user_id, &channel_id, "user", content, None).await?;
        }

        // Conflict detection (only in guild channels, not DMs)
        if !is_dm && self.conflict_enabled && !content.is_empty() && !content.starts_with('!') && !content.starts_with('/') {
            debug!("[{}] 🔍 Running conflict detection analysis", request_id);
            let guild_id_opt = if guild_id != "DM" { Some(guild_id.as_str()) } else { None };
            if let Err(e) = self.check_and_mediate_conflicts(ctx, msg, &channel_id, guild_id_opt).await {
                warn!("[{}] ⚠️ Conflict detection error: {}", request_id, e);
                // Don't fail the whole message processing if conflict detection fails
            }
        }

        if content.starts_with('!') || content.starts_with('/') {
            info!("[{}] 🎯 Processing command: {}", request_id, content.split_whitespace().next().unwrap_or(""));
            self.handle_command_with_id(ctx, msg, request_id).await?;
        } else if is_dm && !content.is_empty() {
            info!("[{}] 💬 Processing DM message (auto-response mode)", request_id);
            self.handle_dm_message_with_id(ctx, msg, request_id).await?;
        } else if !is_dm && self.is_bot_mentioned(ctx, msg).await? && !content.is_empty() {
            info!("[{}] 🏷️ Bot mentioned in channel - responding", request_id);
            self.handle_mention_message_with_id(ctx, msg, request_id).await?;
        } else if !is_dm && !content.is_empty() {
            debug!("[{}] ℹ️ Guild message stored (no bot response needed)", request_id);
        } else {
            debug!("[{}] ℹ️ Message ignored (empty or DM)", request_id);
        }

        info!("[{}] ✅ Message processing completed", request_id);
        Ok(())
    }

    async fn is_bot_mentioned(&self, ctx: &Context, msg: &Message) -> Result<bool> {
        let current_user = ctx.http.get_current_user().await?;
        Ok(msg.mentions.iter().any(|user| user.id == current_user.id))
    }

    async fn is_in_thread(&self, ctx: &Context, msg: &Message) -> Result<bool> {
        use serenity::model::channel::{Channel, ChannelType};

        // Fetch the channel to check its type
        match ctx.http.get_channel(msg.channel_id.0).await {
            Ok(Channel::Guild(guild_channel)) => {
                Ok(matches!(guild_channel.kind,
                    ChannelType::PublicThread | ChannelType::PrivateThread))
            }
            _ => Ok(false),
        }
    }

    async fn fetch_thread_messages(&self, ctx: &Context, msg: &Message, limit: u8, request_id: Uuid) -> Result<Vec<(String, String)>> {
        use serenity::builder::GetMessages;

        debug!("[{}] 🧵 Fetching up to {} messages from thread", request_id, limit);

        // Fetch messages from the thread (Discord API limit is 100)
        let messages = msg.channel_id.messages(&ctx.http, |builder: &mut GetMessages| {
            builder.limit(limit as u64)
        }).await?;

        debug!("[{}] 🧵 Retrieved {} messages from thread", request_id, messages.len());

        // Get bot's user ID to identify bot messages
        let current_user = ctx.http.get_current_user().await?;
        let bot_id = current_user.id;

        // Convert messages to (role, content) format
        // Messages are returned newest first, so reverse for chronological order
        let conversation: Vec<(String, String)> = messages
            .iter()
            .rev() // Reverse to get oldest first (chronological order)
            .filter(|m| !m.content.is_empty()) // Skip empty messages
            .map(|m| {
                let role = if m.author.id == bot_id {
                    "assistant".to_string()
                } else {
                    "user".to_string()
                };
                let content = m.content.clone();
                (role, content)
            })
            .collect();

        debug!("[{}] 🧵 Processed {} non-empty messages from thread", request_id, conversation.len());

        Ok(conversation)
    }

    async fn handle_dm_message_with_id(&self, ctx: &Context, msg: &Message, request_id: Uuid) -> Result<()> {
        let user_id = msg.author.id.to_string();
        let channel_id = msg.channel_id.to_string();
        let user_message = msg.content.trim();

        debug!("[{}] 💬 Processing DM auto-response | User: {} | Message: '{}'",
               request_id, user_id, user_message.chars().take(100).collect::<String>());

        // Get user's persona
        debug!("[{}] 🎭 Fetching user persona from database", request_id);
        let user_persona = self.database.get_user_persona(&user_id).await?;
        debug!("[{}] 🎭 User persona: {}", request_id, user_persona);

        // Store user message in conversation history
        debug!("[{}] 💾 Storing user message to conversation history", request_id);
        self.database.store_message(&user_id, &channel_id, "user", user_message, Some(&user_persona)).await?;
        debug!("[{}] ✅ User message stored successfully", request_id);

        // Retrieve conversation history (last 40 messages = ~20 exchanges)
        debug!("[{}] 📚 Retrieving conversation history", request_id);
        let conversation_history = self.database.get_conversation_history(&user_id, &channel_id, 40).await?;
        info!("[{}] 📚 Retrieved {} historical messages", request_id, conversation_history.len());

        // Show typing indicator while processing
        debug!("[{}] ⌨️ Starting typing indicator", request_id);
        let typing = msg.channel_id.start_typing(&ctx.http)?;

        // Build system prompt without modifier (conversational mode)
        debug!("[{}] 📝 Building system prompt | Persona: {}", request_id, user_persona);
        let system_prompt = self.persona_manager.get_system_prompt(&user_persona, None);
        debug!("[{}] ✅ System prompt generated | Length: {} chars", request_id, system_prompt.len());

        // Log usage
        debug!("[{}] 📊 Logging usage to database", request_id);
        self.database.log_usage(&user_id, "dm_chat", Some(&user_persona)).await?;
        debug!("[{}] ✅ Usage logged successfully", request_id);

        // Get AI response with conversation history
        info!("[{}] 🚀 Calling OpenAI API for DM response", request_id);
        match self.get_ai_response_with_id(&system_prompt, user_message, conversation_history, request_id).await {
            Ok(ai_response) => {
                info!("[{}] ✅ OpenAI response received | Response length: {}",
                      request_id, ai_response.len());

                // Stop typing
                typing.stop();
                debug!("[{}] ⌨️ Stopped typing indicator", request_id);

                // Send response (handle long messages)
                if ai_response.len() > 2000 {
                    debug!("[{}] 📄 Response too long, splitting into chunks", request_id);
                    let chunks: Vec<&str> = ai_response.as_bytes()
                        .chunks(2000)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                        .collect();

                    debug!("[{}] 📄 Split response into {} chunks", request_id, chunks.len());

                    for (i, chunk) in chunks.iter().enumerate() {
                        if !chunk.trim().is_empty() {
                            debug!("[{}] 📤 Sending chunk {} of {} ({} chars)",
                                   request_id, i + 1, chunks.len(), chunk.len());
                            msg.channel_id.say(&ctx.http, chunk).await?;
                            debug!("[{}] ✅ Chunk {} sent successfully", request_id, i + 1);
                        }
                    }
                    info!("[{}] ✅ All DM response chunks sent successfully", request_id);
                } else {
                    debug!("[{}] 📤 Sending DM response ({} chars)", request_id, ai_response.len());
                    msg.channel_id.say(&ctx.http, &ai_response).await?;
                    info!("[{}] ✅ DM response sent successfully", request_id);
                }

                // Store assistant response in conversation history
                debug!("[{}] 💾 Storing assistant response to conversation history", request_id);
                self.database.store_message(&user_id, &channel_id, "assistant", &ai_response, Some(&user_persona)).await?;
                debug!("[{}] ✅ Assistant response stored successfully", request_id);
            }
            Err(e) => {
                typing.stop();
                debug!("[{}] ⌨️ Stopped typing indicator", request_id);
                error!("[{}] ❌ AI response error in DM: {}", request_id, e);

                let error_message = if e.to_string().contains("timed out") {
                    "⏱️ Sorry, I'm taking too long to think. Please try again with a shorter message."
                } else {
                    "❌ Sorry, I encountered an error. Please try again later."
                };

                debug!("[{}] 📤 Sending error message to user", request_id);
                msg.channel_id.say(&ctx.http, error_message).await?;
                warn!("[{}] ⚠️ Error message sent to user after AI failure", request_id);
            }
        }

        info!("[{}] ✅ DM message processing completed", request_id);
        Ok(())
    }

    async fn handle_mention_message_with_id(&self, ctx: &Context, msg: &Message, request_id: Uuid) -> Result<()> {
        let user_id = msg.author.id.to_string();
        let channel_id = msg.channel_id.to_string();
        let user_message = msg.content.trim();

        debug!("[{}] 🏷️ Processing mention in channel | User: {} | Message: '{}'",
               request_id, user_id, user_message.chars().take(100).collect::<String>());

        // Get user's persona
        debug!("[{}] 🎭 Fetching user persona from database", request_id);
        let user_persona = self.database.get_user_persona(&user_id).await?;
        debug!("[{}] 🎭 User persona: {}", request_id, user_persona);

        // Check if message is in a thread
        let is_thread = self.is_in_thread(ctx, msg).await?;
        debug!("[{}] 🧵 Is thread: {}", request_id, is_thread);

        // Retrieve conversation history based on context type
        let conversation_history = if is_thread {
            // Thread context: Fetch messages from Discord (all users, last 40 messages)
            info!("[{}] 🧵 Fetching thread context from Discord", request_id);
            self.fetch_thread_messages(ctx, msg, 40, request_id).await?
        } else {
            // Channel context: Use database history (user-specific, last 40 messages)
            info!("[{}] 📚 Fetching channel context from database", request_id);

            // Store user message in conversation history for channels
            debug!("[{}] 💾 Storing user message to conversation history", request_id);
            self.database.store_message(&user_id, &channel_id, "user", user_message, Some(&user_persona)).await?;
            debug!("[{}] ✅ User message stored successfully", request_id);

            self.database.get_conversation_history(&user_id, &channel_id, 40).await?
        };

        info!("[{}] 📚 Retrieved {} historical messages for context", request_id, conversation_history.len());

        // Show typing indicator while processing
        debug!("[{}] ⌨️ Starting typing indicator", request_id);
        let typing = msg.channel_id.start_typing(&ctx.http)?;

        // Build system prompt without modifier (conversational mode)
        debug!("[{}] 📝 Building system prompt | Persona: {}", request_id, user_persona);
        let system_prompt = self.persona_manager.get_system_prompt(&user_persona, None);
        debug!("[{}] ✅ System prompt generated | Length: {} chars", request_id, system_prompt.len());

        // Log usage
        debug!("[{}] 📊 Logging usage to database", request_id);
        self.database.log_usage(&user_id, "mention_chat", Some(&user_persona)).await?;
        debug!("[{}] ✅ Usage logged successfully", request_id);

        // Get AI response with conversation history
        info!("[{}] 🚀 Calling OpenAI API for mention response", request_id);
        match self.get_ai_response_with_id(&system_prompt, user_message, conversation_history, request_id).await {
            Ok(ai_response) => {
                info!("[{}] ✅ OpenAI response received | Response length: {}",
                      request_id, ai_response.len());

                // Stop typing
                typing.stop();
                debug!("[{}] ⌨️ Stopped typing indicator", request_id);

                // Send response as threaded reply (handle long messages)
                if ai_response.len() > 2000 {
                    debug!("[{}] 📄 Response too long, splitting into chunks", request_id);
                    let chunks: Vec<&str> = ai_response.as_bytes()
                        .chunks(2000)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                        .collect();

                    debug!("[{}] 📄 Split response into {} chunks", request_id, chunks.len());

                    // First chunk as threaded reply
                    if let Some(first_chunk) = chunks.first() {
                        if !first_chunk.trim().is_empty() {
                            debug!("[{}] 📤 Sending first chunk as reply ({} chars)", request_id, first_chunk.len());
                            msg.reply(&ctx.http, first_chunk).await?;
                            debug!("[{}] ✅ First chunk sent as reply", request_id);
                        }
                    }

                    // Remaining chunks as regular messages in the thread
                    for (i, chunk) in chunks.iter().skip(1).enumerate() {
                        if !chunk.trim().is_empty() {
                            debug!("[{}] 📤 Sending chunk {} of {} ({} chars)",
                                   request_id, i + 2, chunks.len(), chunk.len());
                            msg.channel_id.say(&ctx.http, chunk).await?;
                            debug!("[{}] ✅ Chunk {} sent successfully", request_id, i + 2);
                        }
                    }
                    info!("[{}] ✅ All mention response chunks sent successfully", request_id);
                } else {
                    debug!("[{}] 📤 Sending mention response as reply ({} chars)", request_id, ai_response.len());
                    msg.reply(&ctx.http, &ai_response).await?;
                    info!("[{}] ✅ Mention response sent successfully", request_id);
                }

                // Store assistant response in conversation history (only for channels, not threads)
                if !is_thread {
                    debug!("[{}] 💾 Storing assistant response to conversation history", request_id);
                    self.database.store_message(&user_id, &channel_id, "assistant", &ai_response, Some(&user_persona)).await?;
                    debug!("[{}] ✅ Assistant response stored successfully", request_id);
                } else {
                    debug!("[{}] 🧵 Skipping database storage for thread (will fetch from Discord next time)", request_id);
                }
            }
            Err(e) => {
                typing.stop();
                debug!("[{}] ⌨️ Stopped typing indicator", request_id);
                error!("[{}] ❌ AI response error in mention: {}", request_id, e);

                let error_message = if e.to_string().contains("timed out") {
                    "⏱️ Sorry, I'm taking too long to think. Please try again with a shorter message."
                } else {
                    "❌ Sorry, I encountered an error. Please try again later."
                };

                debug!("[{}] 📤 Sending error message to user as reply", request_id);
                msg.reply(&ctx.http, error_message).await?;
                warn!("[{}] ⚠️ Error message sent to user after AI failure", request_id);
            }
        }

        info!("[{}] ✅ Mention message processing completed", request_id);
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
            "forget" => {
                debug!("[{}] 🧹 Handling forget command", request_id);
                self.handle_slash_forget_with_id(ctx, command, request_id).await?;
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
        match self.get_ai_response_with_id(&system_prompt, &user_message, Vec::new(), request_id).await {
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

    async fn handle_slash_forget_with_id(&self, ctx: &Context, command: &ApplicationCommandInteraction, request_id: Uuid) -> Result<()> {
        let user_id = command.user.id.to_string();
        let channel_id = command.channel_id.to_string();

        debug!("[{}] 🧹 Processing forget command for user: {} in channel: {}", request_id, user_id, channel_id);

        // Clear conversation history
        info!("[{}] 🗑️ Clearing conversation history", request_id);
        self.database.clear_conversation_history(&user_id, &channel_id).await?;
        info!("[{}] ✅ Conversation history cleared successfully", request_id);

        // Send confirmation response
        debug!("[{}] 📤 Sending confirmation to Discord", request_id);
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("🧹 Your conversation history has been cleared! I'll start fresh from now on.")
                    })
            })
            .await?;

        info!("[{}] ✅ Forget command completed successfully", request_id);
        Ok(())
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
        self.get_ai_response_with_id(system_prompt, user_message, Vec::new(), Uuid::new_v4()).await
    }

    pub async fn get_ai_response_with_id(&self, system_prompt: &str, user_message: &str, conversation_history: Vec<(String, String)>, request_id: Uuid) -> Result<String> {
        let start_time = Instant::now();

        info!("[{}] 🤖 Starting OpenAI API request | Model: {} | History messages: {}", request_id, self.openai_model, conversation_history.len());
        debug!("[{}] 📝 System prompt length: {} chars | User message length: {} chars",
               request_id, system_prompt.len(), user_message.len());
        debug!("[{}] 📝 User message preview: '{}'",
               request_id, user_message.chars().take(100).collect::<String>());

        debug!("[{}] 🔨 Building OpenAI message objects", request_id);
        let mut messages = vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(system_prompt.to_string()),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        // Add conversation history
        for (role, content) in conversation_history {
            let message_role = match role.as_str() {
                "user" => ChatCompletionMessageRole::User,
                "assistant" => ChatCompletionMessageRole::Assistant,
                _ => continue, // Skip invalid roles
            };
            messages.push(ChatCompletionMessage {
                role: message_role,
                content: Some(content),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            });
        }

        // Add current user message
        messages.push(ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(user_message.to_string()),
            name: None,
            function_call: None,
            tool_call_id: None,
            tool_calls: None,
        });

        debug!("[{}] ✅ OpenAI message objects built successfully | Message count: {}", request_id, messages.len());

        // Add timeout to the OpenAI API call (45 seconds)
        debug!("[{}] 🚀 Initiating OpenAI API call with 45-second timeout", request_id);
        let chat_completion_future = ChatCompletion::builder(&self.openai_model, messages)
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

    async fn check_and_mediate_conflicts(
        &self,
        ctx: &Context,
        msg: &Message,
        channel_id: &str,
        guild_id: Option<&str>,
    ) -> Result<()> {
        // Get the timestamp of the last mediation to avoid re-analyzing same messages
        let last_mediation_ts = self.database.get_last_mediation_timestamp(channel_id).await?;

        // Get recent messages, optionally filtering to only new messages since last mediation
        let recent_messages = if let Some(last_ts) = last_mediation_ts {
            info!("🔍 Getting messages since last mediation at timestamp {}", last_ts);
            self.database.get_recent_channel_messages_since(channel_id, last_ts, 10).await?
        } else {
            info!("🔍 No previous mediation found, getting all recent messages");
            self.database.get_recent_channel_messages(channel_id, 10).await?
        };

        info!("🔍 Conflict check: Found {} recent messages in channel {} (after last mediation)",
              recent_messages.len(), channel_id);

        if recent_messages.len() < 1 {
            info!("⏭️ Skipping conflict detection: No messages found");
            return Ok(());
        }

        // Log message samples for debugging
        let unique_users: std::collections::HashSet<_> = recent_messages.iter()
            .map(|(user_id, _, _)| user_id.clone())
            .collect();
        info!("👥 Messages from {} unique users", unique_users.len());

        for (i, (user_id, content, timestamp)) in recent_messages.iter().take(3).enumerate() {
            debug!("  Message {}: User={} | Content='{}' | Time={}", i, user_id, content, timestamp);
        }

        // Detect conflicts in recent messages
        let (is_conflict, confidence, conflict_type) =
            self.conflict_detector.detect_heated_argument(&recent_messages, 120);

        info!("📊 Detection result: conflict={} | confidence={:.2} | threshold={:.2} | type='{}'",
               is_conflict, confidence, self.conflict_sensitivity_threshold, conflict_type);

        if is_conflict && confidence >= self.conflict_sensitivity_threshold {
            info!("🔥 Conflict detected in channel {} | Confidence: {:.2} | Type: {}",
                  channel_id, confidence, conflict_type);

            // Check if we can intervene (rate limiting)
            if !self.conflict_mediator.can_intervene(channel_id) {
                info!("⏸️ Mediation on cooldown for channel {}", channel_id);
                return Ok(());
            }

            // Extract participant user IDs
            let participants: Vec<String> = recent_messages
                .iter()
                .map(|(user_id, _, _)| user_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            info!("👥 Conflict participants: {} users", participants.len());

            if participants.len() < 1 {
                info!("⏭️ Skipping mediation: No participants found");
                return Ok(());
            }

            // Record the conflict in database
            let participants_json = serde_json::to_string(&participants)?;
            let conflict_id = self.database.record_conflict_detection(
                channel_id,
                guild_id,
                &participants_json,
                &conflict_type,
                confidence,
                &msg.id.to_string(),
            ).await?;

            // Generate context-aware mediation response using OpenAI
            info!("🤖 Generating context-aware mediation response with OpenAI...");
            let mediation_text = match self.generate_mediation_response(&recent_messages, &conflict_type, confidence).await {
                Ok(response) => {
                    info!("✅ OpenAI mediation response generated successfully");
                    response
                },
                Err(e) => {
                    warn!("⚠️ Failed to generate AI mediation response: {}. Using fallback.", e);
                    self.conflict_mediator.get_mediation_response(&conflict_type, confidence)
                }
            };

            // Send mediation message as Obi-Wan with proper error handling
            match msg.channel_id.say(&ctx.http, &mediation_text).await {
                Ok(mediation_msg) => {
                    info!("☮️ Mediation sent successfully in channel {} | Message: {}", channel_id, mediation_text);

                    // Record the intervention
                    self.conflict_mediator.record_intervention(channel_id);

                    // Record in database
                    self.database.mark_mediation_triggered(conflict_id, &mediation_msg.id.to_string()).await?;
                    self.database.record_mediation(conflict_id, channel_id, &mediation_text).await?;
                },
                Err(e) => {
                    warn!("⚠️ Failed to send mediation message to Discord: {}. Recording intervention to prevent spam.", e);

                    // Still record the intervention to prevent repeated mediation attempts
                    self.conflict_mediator.record_intervention(channel_id);

                    // Try to record in database with no message ID
                    if let Err(db_err) = self.database.record_mediation(conflict_id, channel_id, &mediation_text).await {
                        warn!("⚠️ Failed to record mediation in database: {}", db_err);
                    }
                }
            }

            // Update user interaction patterns
            if participants.len() == 2 {
                let user_a = &participants[0];
                let user_b = &participants[1];
                self.database.update_user_interaction_pattern(user_a, user_b, channel_id, true).await?;
            }
        }

        Ok(())
    }

    /// Generate a context-aware mediation response using OpenAI
    async fn generate_mediation_response(
        &self,
        messages: &[(String, String, String)], // (user_id, content, timestamp)
        conflict_type: &str,
        confidence: f32,
    ) -> Result<String> {
        // Build conversation context from recent messages
        let mut conversation_context = String::new();
        for (user_id, content, _timestamp) in messages.iter().rev().take(5) {
            conversation_context.push_str(&format!("User {}: {}\n", user_id, content));
        }

        // Create system prompt for Obi-Wan as mediator
        let mediation_prompt = format!(
            "You are Obi-Wan Kenobi observing a conversation that has become heated. \
            Your role is to gently mediate and bring calm wisdom to the situation.\n\n\
            Conflict type detected: {}\n\
            Confidence: {:.0}%\n\n\
            Recent conversation:\n{}\n\n\
            Respond with a brief, characteristic Obi-Wan comment that:\n\
            1. Acknowledges what's being discussed specifically\n\
            2. Offers a calming philosophical perspective\n\
            3. Encourages understanding or reflection\n\
            4. Stays in character with Obi-Wan's wise, measured tone\n\n\
            Keep it to 1-2 sentences maximum. Be natural and conversational, not preachy.",
            conflict_type,
            confidence * 100.0,
            conversation_context
        );

        // Call OpenAI (API key set at startup)
        let chat_completion = ChatCompletion::builder(&self.openai_model, vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(mediation_prompt),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ])
        .create()
        .await?;

        let response = chat_completion
            .choices
            .first()
            .and_then(|choice| choice.message.content.clone())
            .unwrap_or_else(|| "I sense tension here. Perhaps a moment of calm reflection would serve us all well.".to_string());

        Ok(response)
    }
}