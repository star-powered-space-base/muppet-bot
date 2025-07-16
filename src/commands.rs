use crate::audio::AudioTranscriber;
use crate::database::Database;
use crate::personas::PersonaManager;
use crate::rate_limiter::RateLimiter;
use anyhow::Result;
use log::{error, info, warn};
use openai::chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole};
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::time::Duration;

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

    async fn get_ai_response(&self, system_prompt: &str, user_message: &str) -> Result<String> {
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