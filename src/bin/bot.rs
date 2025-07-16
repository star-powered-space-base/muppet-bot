use anyhow::Result;
use log::{error, info};
use openai::set_key;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::sync::Arc;

use persona::commands::CommandHandler;
use persona::config::Config;
use persona::database::Database;

struct Handler {
    command_handler: Arc<CommandHandler>,
}

impl Handler {
    fn new(command_handler: CommandHandler) -> Self {
        Handler {
            command_handler: Arc::new(command_handler),
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

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected and ready!", ready.user.name);
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
    let command_handler = CommandHandler::new(database, config.openai_api_key.clone());
    let handler = Handler::new(command_handler);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(handler)
        .await?;

    info!("Bot configured successfully. Starting client...");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
        return Err(anyhow::anyhow!("Client failed to start: {}", why));
    }

    Ok(())
}

