# Persona Discord Bot

A sophisticated Discord bot that provides AI-powered conversations through different personas. Built with Rust, Serenity, and OpenAI integration.

## Features

- **Multiple Personas**: Switch between different AI personalities (Muppet Expert, Chef, Teacher, Analyst)
- **User Preferences**: Each user can set their default persona
- **Rate Limiting**: Prevents API abuse with configurable rate limits
- **Database Storage**: SQLite database for user preferences and usage statistics
- **Comprehensive Logging**: Detailed logging with configurable levels
- **Error Handling**: Robust error handling throughout the application

## Available Commands

- `!ping` - Test bot responsiveness
- `/help` - Show help message with all commands
- `/personas` - List available personas and show current persona
- `/set_persona <name>` - Set your default persona
- `/hey <message>` - Chat with your current persona
- `/explain <message>` - Get an explanation
- `/simple <message>` - Get a simple explanation with analogies
- `/steps <message>` - Break something into steps
- `/recipe <food>` - Get a recipe for the specified food

## Available Personas

- **muppet** - Enthusiastic Muppet expert (default)
- **chef** - Passionate cooking expert
- **teacher** - Patient teacher who explains things clearly
- **analyst** - Step-by-step analyst who breaks down complex processes

## Setup

### Prerequisites

- Rust (latest stable version)
- Discord Bot Token
- OpenAI API Key

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd persona
   ```

2. Copy the environment file and fill in your credentials:
   ```bash
   cp .env.example .env
   ```

3. Edit `.env` with your actual tokens:
   ```
   DISCORD_MUPPET_FRIEND=your_discord_bot_token_here
   OPENAI_API_KEY=your_openai_api_key_here
   ```

4. Build and run the bot:
   ```bash
   cargo run --bin bot
   ```

### Discord Bot Setup

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application
3. Go to the "Bot" section and create a bot
4. Copy the token and add it to your `.env` file
5. Under "Privileged Gateway Intents", enable "Message Content Intent"
6. Use the OAuth2 URL generator to invite the bot to your server with appropriate permissions

### OpenAI Setup

1. Go to [OpenAI Platform](https://platform.openai.com/api-keys)
2. Create a new API key
3. Add it to your `.env` file

## Configuration

### Environment Variables

- `DISCORD_MUPPET_FRIEND` - Your Discord bot token (required)
- `OPENAI_API_KEY` - Your OpenAI API key (required)
- `DATABASE_PATH` - Path to SQLite database file (optional, defaults to "persona.db")
- `LOG_LEVEL` - Logging level (optional, defaults to "info")

### Logging Levels

- `error` - Only errors
- `warn` - Warnings and errors
- `info` - General information, warnings, and errors
- `debug` - Debug information and above
- `trace` - All logging information

## Architecture

The bot is structured with the following modules:

- `config.rs` - Configuration management
- `database.rs` - SQLite database operations
- `personas.rs` - Persona definitions and management
- `commands.rs` - Command handling logic
- `rate_limiter.rs` - Rate limiting functionality
- `bin/bot.rs` - Main bot entry point

## Database Schema

The bot uses SQLite with the following tables:

- `user_preferences` - Stores user's default persona settings
- `usage_stats` - Tracks command usage for analytics

## Rate Limiting

The bot implements rate limiting to prevent abuse:
- 10 requests per minute per user
- Automatic backoff and user notification when limits are exceeded

## Audio Transcription

The bot includes an audio transcription script (`scripts/audio.sh`) that uses OpenAI's Whisper API:

```bash
./scripts/audio.sh <path-to-audio-file>
```

## Development

### Building

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the bot
cargo run --bin bot

# Run tests (when implemented)
cargo test
```

### Adding New Personas

To add a new persona, edit `src/personas.rs` and add a new entry to the `PersonaManager::new()` function:

```rust
personas.insert("your_persona".to_string(), Persona {
    name: "Your Persona Name".to_string(),
    system_prompt: "Your persona's system prompt here.".to_string(),
    description: "Brief description of your persona".to_string(),
});
```

## License

This project is open source. Please check the license file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request