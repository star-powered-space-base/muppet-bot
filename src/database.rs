use anyhow::Result;
use log::info;
use sqlite::{Connection, State};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn new(database_path: &str) -> Result<Self> {
        let connection = sqlite::open(database_path)?;
        let db = Database {
            connection: Arc::new(Mutex::new(connection)),
        };
        
        db.init_tables().await?;
        info!("Database initialized at: {}", database_path);
        Ok(db)
    }

    async fn init_tables(&self) -> Result<()> {
        let conn = self.connection.lock().await;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_preferences (
                user_id TEXT PRIMARY KEY,
                default_persona TEXT DEFAULT 'muppet',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS usage_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                command TEXT NOT NULL,
                persona TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS conversation_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                persona TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_channel
             ON conversation_history(user_id, channel_id)",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp
             ON conversation_history(timestamp)",
        )?;

        Ok(())
    }

    pub async fn get_user_persona(&self, user_id: &str) -> Result<String> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare("SELECT default_persona FROM user_preferences WHERE user_id = ?")?;
        statement.bind((1, user_id))?;

        if let Ok(State::Row) = statement.next() {
            Ok(statement.read::<String, _>("default_persona")?)
        } else {
            Ok("muppet".to_string())
        }
    }

    pub async fn set_user_persona(&self, user_id: &str, persona: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO user_preferences (user_id, default_persona, updated_at) 
             VALUES (?, ?, CURRENT_TIMESTAMP)",
        )?;
        
        let mut statement = conn.prepare(
            "INSERT OR REPLACE INTO user_preferences (user_id, default_persona, updated_at) 
             VALUES (?, ?, CURRENT_TIMESTAMP)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, persona))?;
        statement.next()?;
        
        info!("Updated persona for user {} to {}", user_id, persona);
        Ok(())
    }

    pub async fn log_usage(&self, user_id: &str, command: &str, persona: Option<&str>) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO usage_stats (user_id, command, persona) VALUES (?, ?, ?)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, command))?;
        statement.bind((3, persona.unwrap_or("")))?;
        statement.next()?;
        Ok(())
    }

    pub async fn store_message(&self, user_id: &str, channel_id: &str, role: &str, content: &str, persona: Option<&str>) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO conversation_history (user_id, channel_id, role, content, persona) VALUES (?, ?, ?, ?, ?)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, channel_id))?;
        statement.bind((3, role))?;
        statement.bind((4, content))?;
        statement.bind((5, persona.unwrap_or("")))?;
        statement.next()?;
        Ok(())
    }

    pub async fn get_conversation_history(&self, user_id: &str, channel_id: &str, limit: i64) -> Result<Vec<(String, String)>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT role, content FROM conversation_history
             WHERE user_id = ? AND channel_id = ?
             ORDER BY timestamp DESC
             LIMIT ?"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, channel_id))?;
        statement.bind((3, limit))?;

        let mut history = Vec::new();
        while let Ok(State::Row) = statement.next() {
            let role = statement.read::<String, _>("role")?;
            let content = statement.read::<String, _>("content")?;
            history.push((role, content));
        }

        // Reverse to get chronological order (oldest first)
        history.reverse();
        Ok(history)
    }

    pub async fn clear_conversation_history(&self, user_id: &str, channel_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "DELETE FROM conversation_history WHERE user_id = ? AND channel_id = ?"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, channel_id))?;
        statement.next()?;
        info!("Cleared conversation history for user {} in channel {}", user_id, channel_id);
        Ok(())
    }

    pub async fn cleanup_old_messages(&self, days: i64) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "DELETE FROM conversation_history WHERE timestamp < datetime('now', ? || ' days')"
        )?;
        statement.bind((1, format!("-{}", days).as_str()))?;
        statement.next()?;
        info!("Cleaned up conversation history older than {} days", days);
        Ok(())
    }
}