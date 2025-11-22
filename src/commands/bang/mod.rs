//! # Bang Commands (!)
//!
//! Text-based commands prefixed with exclamation point for quick operations.
//!
//! - **Version**: 1.0.0
//! - **Since**: 0.2.0
//! - **Toggleable**: false
//!
//! ## Changelog
//! - 1.0.0: Initial implementation with info, quick, and admin commands

pub mod admin;
pub mod info;
pub mod quick;

/// Represents a parsed bang command
#[derive(Debug, Clone)]
pub struct BangCommand {
    /// The command name (without the ! prefix)
    pub name: String,
    /// Arguments passed to the command
    pub args: Vec<String>,
    /// The full original input (without the ! prefix)
    pub raw: String,
}

impl BangCommand {
    /// Check if the command matches a given name (case-insensitive)
    pub fn is(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    /// Get the first argument, if any
    pub fn first_arg(&self) -> Option<&str> {
        self.args.first().map(|s| s.as_str())
    }

    /// Get remaining arguments as a single string
    pub fn rest(&self) -> String {
        self.args.join(" ")
    }
}

/// Parse a bang command from input text
///
/// # Arguments
/// * `input` - The text after the `!` prefix
///
/// # Returns
/// A `BangCommand` struct with parsed name and arguments
///
/// # Example
/// ```
/// use persona::commands::bang::parse_bang_command;
///
/// let cmd = parse_bang_command("toggle conflict_detection");
/// assert_eq!(cmd.name, "toggle");
/// assert_eq!(cmd.args, vec!["conflict_detection"]);
/// ```
pub fn parse_bang_command(input: &str) -> BangCommand {
    let input = input.trim();
    let mut parts = input.split_whitespace();

    let name = parts.next().unwrap_or("").to_string();
    let args: Vec<String> = parts.map(|s| s.to_string()).collect();

    BangCommand {
        name,
        args,
        raw: input.to_string(),
    }
}

/// Get help text for all bang commands
pub fn get_help_text() -> String {
    let mut help = String::from("**Bang Commands (!)**\n\n");

    help.push_str("**Info Commands:**\n");
    help.push_str("`!help` - Show this help message\n");
    help.push_str("`!status` - Show bot status and uptime\n");
    help.push_str("`!version` - Show bot and feature versions\n");
    help.push_str("`!uptime` - Show how long the bot has been running\n\n");

    help.push_str("**Quick Commands:**\n");
    help.push_str("`!ping` - Quick ping (text only)\n");
    help.push_str("`!features` - List all features with versions\n\n");

    help.push_str("**Admin Commands** (require MANAGE_GUILD):\n");
    help.push_str("`!toggle <feature>` - Enable/disable a feature\n");
    help.push_str("`!reload` - Reload guild settings\n");
    help.push_str("`!sync` - Force sync slash commands\n");

    help
}

/// All available bang command names
pub const COMMANDS: &[&str] = &[
    // Info commands
    "help",
    "status",
    "version",
    "uptime",
    // Quick commands
    "ping",
    "features",
    // Admin commands
    "toggle",
    "reload",
    "sync",
];

/// Check if a string is a valid bang command
pub fn is_valid_command(name: &str) -> bool {
    COMMANDS.iter().any(|&cmd| cmd.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let cmd = parse_bang_command("ping");
        assert_eq!(cmd.name, "ping");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_parse_command_with_args() {
        let cmd = parse_bang_command("toggle conflict_detection");
        assert_eq!(cmd.name, "toggle");
        assert_eq!(cmd.args, vec!["conflict_detection"]);
    }

    #[test]
    fn test_parse_command_with_multiple_args() {
        let cmd = parse_bang_command("some command with many args");
        assert_eq!(cmd.name, "some");
        assert_eq!(cmd.args, vec!["command", "with", "many", "args"]);
    }

    #[test]
    fn test_command_is_check() {
        let cmd = parse_bang_command("PING");
        assert!(cmd.is("ping"));
        assert!(cmd.is("PING"));
        assert!(!cmd.is("pong"));
    }

    #[test]
    fn test_is_valid_command() {
        assert!(is_valid_command("ping"));
        assert!(is_valid_command("PING"));
        assert!(is_valid_command("toggle"));
        assert!(!is_valid_command("nonexistent"));
    }
}
