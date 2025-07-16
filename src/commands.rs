use crate::audio::AudioTranscriber;
use crate::database::Database;
use crate::message_components::MessageComponentHandler;
use crate::personas::PersonaManager;
use crate::rate_limiter::RateLimiter;
use crate::slash_commands::get_string_option;
use anyhow::Result;
use log::{error, info, warn};
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
}

impl CommandHandler {
    pub fn new(database: Database, openai_api_key: String) -> Self {
        CommandHandler {
            persona_manager: PersonaManager::new(),
            database,
            rate_limiter: RateLimiter::new(10, Duration::from_secs(60)), // 10 requests per minute
            audio_transcriber: AudioTranscriber::new(openai_api_key),
        }
    }

    pub async fn handle_message(&self, ctx: &Context, msg: &Message) -> Result<()> {
        let user_id = msg.author.id.to_string();
        
        if !self.rate_limiter.wait_for_rate_limit(&user_id).await {
            warn!("Rate limit exceeded for user: {}", user_id);
            msg.channel_id
                .say(&ctx.http, "You're sending messages too quickly! Please slow down.")
                .await?;
            return Ok(());
        }

        if !msg.attachments.is_empty() {
            self.handle_audio_attachments(ctx, msg).await?;
        }

        let content = msg.content.trim();
        
        if content.starts_with('!') || content.starts_with('/') {
            self.handle_command(ctx, msg).await?;
        }

        Ok(())
    }

    pub async fn handle_slash_command(&self, ctx: &Context, command: &ApplicationCommandInteraction) -> Result<()> {
        let user_id = command.user.id.to_string();
        
        if !self.rate_limiter.wait_for_rate_limit(&user_id).await {
            warn!("Rate limit exceeded for user: {}", user_id);
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.content("You're sending commands too quickly! Please slow down.")
                        })
                })
                .await?;
            return Ok(());
        }

        info!("Processing slash command: {} from user: {}", command.data.name, user_id);

        match command.data.name.as_str() {
            "ping" => {
                self.handle_slash_ping(ctx, command).await?;
            }
            "help" => {
                self.handle_slash_help(ctx, command).await?;
            }
            "personas" => {
                self.handle_slash_personas(ctx, command).await?;
            }
            "set_persona" => {
                self.handle_slash_set_persona(ctx, command).await?;
            }
            "hey" | "explain" | "simple" | "steps" | "recipe" => {
                self.handle_slash_ai_command(ctx, command).await?;
            }
            "Analyze Message" | "Explain Message" => {
                self.handle_context_menu_message(ctx, command).await?;
            }
            "Analyze User" => {
                self.handle_context_menu_user(ctx, command).await?;
            }
            _ => {
                command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.content("Unknown command. Use `/help` to see available commands.")
                            })
                    })
                    .await?;
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
        let option_name = match command.data.name.as_str() {
            "hey" => "message",
            "explain" => "topic",
            "simple" => "topic",
            "steps" => "task",
            "recipe" => "food",
            _ => "message",
        };

        let user_message = get_string_option(&command.data.options, option_name)
            .ok_or_else(|| anyhow::anyhow!("Missing message parameter"))?;

        let user_id = command.user.id.to_string();
        let user_persona = self.database.get_user_persona(&user_id).await?;
        
        let modifier = match command.data.name.as_str() {
            "explain" => Some("explain"),
            "simple" => Some("simple"),
            "steps" => Some("steps"),
            "recipe" => Some("recipe"),
            _ => None,
        };

        let system_prompt = self.persona_manager.get_system_prompt(&user_persona, modifier);

        self.database.log_usage(&user_id, &command.data.name, Some(&user_persona)).await?;

        // Immediately defer the interaction to prevent timeout (required within 3 seconds)
        command
            .create_interaction_response(&ctx.http, |response| {
                response.kind(serenity::model::application::interaction::InteractionResponseType::DeferredChannelMessageWithSource)
            })
            .await?;

        // Get AI response and edit the message
        match self.get_ai_response(&system_prompt, &user_message).await {
            Ok(ai_response) => {
                if ai_response.len() > 2000 {
                    // For long responses, edit with the first part and send follow-ups
                    let chunks: Vec<&str> = ai_response.as_bytes()
                        .chunks(2000)
                        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                        .collect();
                    
                    if let Some(first_chunk) = chunks.first() {
                        command
                            .edit_original_interaction_response(&ctx.http, |response| {
                                response.content(first_chunk)
                            })
                            .await?;
                    }

                    // Send remaining chunks as follow-up messages
                    for chunk in chunks.iter().skip(1) {
                        if !chunk.trim().is_empty() {
                            command
                                .create_followup_message(&ctx.http, |message| {
                                    message.content(chunk)
                                })
                                .await?;
                        }
                    }
                } else {
                    command
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response.content(&ai_response)
                        })
                        .await?;
                }
            }
            Err(e) => {
                error!("OpenAI API error: {}", e);
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content("Sorry, I encountered an error processing your request. Please try again later.")
                    })
                    .await?;
            }
        }

        Ok(())
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
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content("❌ Sorry, I encountered an error analyzing the message.")
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
                command
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content("❌ Sorry, I encountered an error analyzing the user.")
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
                msg.channel_id
                    .say(&ctx.http, "Sorry, I encountered an error processing your request. Please try again later.")
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn get_ai_response(&self, system_prompt: &str, user_message: &str) -> Result<String> {
        let messages = vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(system_prompt.to_string()),
                name: None,
                function_call: None,
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(user_message.to_string()),
                name: None,
                function_call: None,
            },
        ];

        let chat_completion = ChatCompletion::builder("gpt-3.5-turbo", messages)
            .create()
            .await?;

        let response = chat_completion
            .choices
            .first()
            .and_then(|choice| choice.message.content.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No response from OpenAI"))?;

        Ok(response.trim().to_string())
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