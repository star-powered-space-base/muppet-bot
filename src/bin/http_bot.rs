use anyhow::Result;
use dotenvy::dotenv;
use env_logger;
use log::{info, error};
use tokio;

use persona::config::Config;
use persona::database::Database;
use persona::commands::CommandHandler;
use persona::http_server::start_http_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize logging
    env_logger::init();

    info!("🚀 Starting Discord Bot HTTP Server...");

    // Load configuration
    let config = Config::from_env()?;
    info!("✅ Configuration loaded");

    // Initialize database
    let database = Database::new(&config.database_path).await?;
    info!("✅ Database connected");

    // Create command handler
    let command_handler = CommandHandler::new(
        database,
        config.openai_api_key.clone(),
    );

    info!("✅ Command handler initialized");

    // Start HTTP server on port 8080 (matches ngrok configuration)
    let port = 8080;
    info!("🌐 Starting HTTP server on port {}", port);
    info!("📡 Interactions endpoint: https://0fbf2d802093.ngrok-free.app/interactions");
    
    if let Err(e) = start_http_server(config, command_handler, port).await {
        error!("❌ HTTP server failed: {}", e);
        return Err(e);
    }

    Ok(())
}