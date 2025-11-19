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

        // Enhanced Interaction Tracking
        conn.execute(
            "CREATE TABLE IF NOT EXISTS message_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                attachment_urls TEXT,
                embed_data TEXT,
                reactions TEXT,
                edited_at DATETIME,
                deleted_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_message_id
             ON message_metadata(message_id)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS interaction_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                guild_id TEXT,
                session_start DATETIME DEFAULT CURRENT_TIMESTAMP,
                session_end DATETIME,
                message_count INTEGER DEFAULT 0,
                last_activity DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_user
             ON interaction_sessions(user_id, session_start)",
        )?;

        // Feature-Specific Data
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                bookmark_name TEXT,
                bookmark_note TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bookmark_user
             ON user_bookmarks(user_id)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                reminder_text TEXT NOT NULL,
                remind_at DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed BOOLEAN DEFAULT 0,
                completed_at DATETIME
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reminder_time
             ON reminders(remind_at, completed)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS custom_commands (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command_name TEXT NOT NULL,
                response_text TEXT NOT NULL,
                created_by_user_id TEXT NOT NULL,
                guild_id TEXT,
                is_global BOOLEAN DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(command_name, guild_id)
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_custom_command
             ON custom_commands(command_name, guild_id)",
        )?;

        // Analytics & Metrics
        conn.execute(
            "CREATE TABLE IF NOT EXISTS daily_analytics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date DATE UNIQUE NOT NULL,
                total_messages INTEGER DEFAULT 0,
                unique_users INTEGER DEFAULT 0,
                total_commands INTEGER DEFAULT 0,
                total_errors INTEGER DEFAULT 0,
                persona_usage TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_analytics_date
             ON daily_analytics(date)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS performance_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                metric_type TEXT NOT NULL,
                value REAL NOT NULL,
                unit TEXT,
                metadata TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_type
             ON performance_metrics(metric_type, timestamp)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS error_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                error_type TEXT NOT NULL,
                error_message TEXT NOT NULL,
                stack_trace TEXT,
                user_id TEXT,
                channel_id TEXT,
                command TEXT,
                metadata TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_error_type
             ON error_logs(error_type, timestamp)",
        )?;

        // Extended Configuration
        conn.execute(
            "CREATE TABLE IF NOT EXISTS feature_flags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                feature_name TEXT NOT NULL,
                enabled BOOLEAN DEFAULT 0,
                user_id TEXT,
                guild_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(feature_name, user_id, guild_id)
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_feature_flag
             ON feature_flags(feature_name, user_id, guild_id)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS guild_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                guild_id TEXT NOT NULL,
                setting_key TEXT NOT NULL,
                setting_value TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(guild_id, setting_key)
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_guild_setting
             ON guild_settings(guild_id, setting_key)",
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS extended_user_preferences (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                preference_key TEXT NOT NULL,
                preference_value TEXT,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(user_id, preference_key)
            )",
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_pref
             ON extended_user_preferences(user_id, preference_key)",
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

    // Message Metadata Methods
    pub async fn store_message_metadata(
        &self,
        message_id: &str,
        user_id: &str,
        channel_id: &str,
        attachment_urls: Option<&str>,
        embed_data: Option<&str>,
        reactions: Option<&str>,
    ) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO message_metadata (message_id, user_id, channel_id, attachment_urls, embed_data, reactions)
             VALUES (?, ?, ?, ?, ?, ?)"
        )?;
        statement.bind((1, message_id))?;
        statement.bind((2, user_id))?;
        statement.bind((3, channel_id))?;
        statement.bind((4, attachment_urls.unwrap_or("")))?;
        statement.bind((5, embed_data.unwrap_or("")))?;
        statement.bind((6, reactions.unwrap_or("")))?;
        statement.next()?;
        Ok(())
    }

    pub async fn update_message_metadata_reactions(&self, message_id: &str, reactions: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "UPDATE message_metadata SET reactions = ? WHERE message_id = ?"
        )?;
        statement.bind((1, reactions))?;
        statement.bind((2, message_id))?;
        statement.next()?;
        Ok(())
    }

    pub async fn mark_message_deleted(&self, message_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "UPDATE message_metadata SET deleted_at = CURRENT_TIMESTAMP WHERE message_id = ?"
        )?;
        statement.bind((1, message_id))?;
        statement.next()?;
        Ok(())
    }

    pub async fn mark_message_edited(&self, message_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "UPDATE message_metadata SET edited_at = CURRENT_TIMESTAMP WHERE message_id = ?"
        )?;
        statement.bind((1, message_id))?;
        statement.next()?;
        Ok(())
    }

    // Interaction Session Methods
    pub async fn start_session(&self, user_id: &str, guild_id: Option<&str>) -> Result<i64> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO interaction_sessions (user_id, guild_id) VALUES (?, ?)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, guild_id.unwrap_or("")))?;
        statement.next()?;

        // Get the last inserted row id
        let mut stmt = conn.prepare("SELECT last_insert_rowid()")?;
        stmt.next()?;
        let session_id = stmt.read::<i64, _>(0)?;
        Ok(session_id)
    }

    pub async fn update_session_activity(&self, session_id: i64) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "UPDATE interaction_sessions
             SET message_count = message_count + 1, last_activity = CURRENT_TIMESTAMP
             WHERE id = ?"
        )?;
        statement.bind((1, session_id))?;
        statement.next()?;
        Ok(())
    }

    pub async fn end_session(&self, session_id: i64) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "UPDATE interaction_sessions SET session_end = CURRENT_TIMESTAMP WHERE id = ?"
        )?;
        statement.bind((1, session_id))?;
        statement.next()?;
        Ok(())
    }

    // User Bookmark Methods
    pub async fn add_bookmark(
        &self,
        user_id: &str,
        channel_id: &str,
        message_id: &str,
        bookmark_name: Option<&str>,
        bookmark_note: Option<&str>,
    ) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO user_bookmarks (user_id, channel_id, message_id, bookmark_name, bookmark_note)
             VALUES (?, ?, ?, ?, ?)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, channel_id))?;
        statement.bind((3, message_id))?;
        statement.bind((4, bookmark_name.unwrap_or("")))?;
        statement.bind((5, bookmark_note.unwrap_or("")))?;
        statement.next()?;
        info!("Added bookmark for user {}", user_id);
        Ok(())
    }

    pub async fn get_user_bookmarks(&self, user_id: &str) -> Result<Vec<(String, String, String, String)>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT message_id, channel_id, bookmark_name, bookmark_note
             FROM user_bookmarks WHERE user_id = ?
             ORDER BY created_at DESC"
        )?;
        statement.bind((1, user_id))?;

        let mut bookmarks = Vec::new();
        while let Ok(State::Row) = statement.next() {
            let message_id = statement.read::<String, _>(0)?;
            let channel_id = statement.read::<String, _>(1)?;
            let bookmark_name = statement.read::<String, _>(2)?;
            let bookmark_note = statement.read::<String, _>(3)?;
            bookmarks.push((message_id, channel_id, bookmark_name, bookmark_note));
        }
        Ok(bookmarks)
    }

    pub async fn delete_bookmark(&self, user_id: &str, message_id: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "DELETE FROM user_bookmarks WHERE user_id = ? AND message_id = ?"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, message_id))?;
        statement.next()?;
        Ok(())
    }

    // Reminder Methods
    pub async fn add_reminder(
        &self,
        user_id: &str,
        channel_id: &str,
        reminder_text: &str,
        remind_at: &str,
    ) -> Result<i64> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO reminders (user_id, channel_id, reminder_text, remind_at)
             VALUES (?, ?, ?, ?)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, channel_id))?;
        statement.bind((3, reminder_text))?;
        statement.bind((4, remind_at))?;
        statement.next()?;

        let mut stmt = conn.prepare("SELECT last_insert_rowid()")?;
        stmt.next()?;
        let reminder_id = stmt.read::<i64, _>(0)?;
        info!("Added reminder {} for user {}", reminder_id, user_id);
        Ok(reminder_id)
    }

    pub async fn get_pending_reminders(&self) -> Result<Vec<(i64, String, String, String)>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT id, user_id, channel_id, reminder_text
             FROM reminders
             WHERE completed = 0 AND remind_at <= datetime('now')
             ORDER BY remind_at ASC"
        )?;

        let mut reminders = Vec::new();
        while let Ok(State::Row) = statement.next() {
            let id = statement.read::<i64, _>(0)?;
            let user_id = statement.read::<String, _>(1)?;
            let channel_id = statement.read::<String, _>(2)?;
            let reminder_text = statement.read::<String, _>(3)?;
            reminders.push((id, user_id, channel_id, reminder_text));
        }
        Ok(reminders)
    }

    pub async fn complete_reminder(&self, reminder_id: i64) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "UPDATE reminders SET completed = 1, completed_at = CURRENT_TIMESTAMP WHERE id = ?"
        )?;
        statement.bind((1, reminder_id))?;
        statement.next()?;
        Ok(())
    }

    pub async fn get_user_reminders(&self, user_id: &str) -> Result<Vec<(i64, String, String, String)>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT id, channel_id, reminder_text, remind_at
             FROM reminders
             WHERE user_id = ? AND completed = 0
             ORDER BY remind_at ASC"
        )?;
        statement.bind((1, user_id))?;

        let mut reminders = Vec::new();
        while let Ok(State::Row) = statement.next() {
            let id = statement.read::<i64, _>(0)?;
            let channel_id = statement.read::<String, _>(1)?;
            let reminder_text = statement.read::<String, _>(2)?;
            let remind_at = statement.read::<String, _>(3)?;
            reminders.push((id, channel_id, reminder_text, remind_at));
        }
        Ok(reminders)
    }

    // Custom Command Methods
    pub async fn add_custom_command(
        &self,
        command_name: &str,
        response_text: &str,
        created_by_user_id: &str,
        guild_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.connection.lock().await;
        let is_global = guild_id.is_none();
        let mut statement = conn.prepare(
            "INSERT OR REPLACE INTO custom_commands (command_name, response_text, created_by_user_id, guild_id, is_global, updated_at)
             VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"
        )?;
        statement.bind((1, command_name))?;
        statement.bind((2, response_text))?;
        statement.bind((3, created_by_user_id))?;
        statement.bind((4, guild_id.unwrap_or("")))?;
        statement.bind((5, if is_global { 1i64 } else { 0i64 }))?;
        statement.next()?;
        info!("Added custom command: {}", command_name);
        Ok(())
    }

    pub async fn get_custom_command(&self, command_name: &str, guild_id: Option<&str>) -> Result<Option<String>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT response_text FROM custom_commands
             WHERE command_name = ? AND (guild_id = ? OR is_global = 1)
             ORDER BY is_global ASC
             LIMIT 1"
        )?;
        statement.bind((1, command_name))?;
        statement.bind((2, guild_id.unwrap_or("")))?;

        if let Ok(State::Row) = statement.next() {
            Ok(Some(statement.read::<String, _>(0)?))
        } else {
            Ok(None)
        }
    }

    pub async fn delete_custom_command(&self, command_name: &str, guild_id: Option<&str>) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "DELETE FROM custom_commands WHERE command_name = ? AND guild_id = ?"
        )?;
        statement.bind((1, command_name))?;
        statement.bind((2, guild_id.unwrap_or("")))?;
        statement.next()?;
        Ok(())
    }

    // Analytics Methods
    pub async fn increment_daily_stat(&self, stat_type: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        match stat_type {
            "message" => {
                conn.execute(
                    "INSERT INTO daily_analytics (date, total_messages) VALUES (?, 1)
                     ON CONFLICT(date) DO UPDATE SET total_messages = total_messages + 1"
                )?;
            }
            "command" => {
                conn.execute(
                    "INSERT INTO daily_analytics (date, total_commands) VALUES (?, 1)
                     ON CONFLICT(date) DO UPDATE SET total_commands = total_commands + 1"
                )?;
            }
            "error" => {
                conn.execute(
                    "INSERT INTO daily_analytics (date, total_errors) VALUES (?, 1)
                     ON CONFLICT(date) DO UPDATE SET total_errors = total_errors + 1"
                )?;
            }
            _ => {}
        }

        let mut statement = conn.prepare(
            "INSERT INTO daily_analytics (date, total_messages) VALUES (?, 0)
             ON CONFLICT(date) DO NOTHING"
        )?;
        statement.bind((1, date.as_str()))?;
        statement.next()?;
        Ok(())
    }

    pub async fn add_performance_metric(&self, metric_type: &str, value: f64, unit: Option<&str>, metadata: Option<&str>) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO performance_metrics (metric_type, value, unit, metadata) VALUES (?, ?, ?, ?)"
        )?;
        statement.bind((1, metric_type))?;
        statement.bind((2, value))?;
        statement.bind((3, unit.unwrap_or("")))?;
        statement.bind((4, metadata.unwrap_or("")))?;
        statement.next()?;
        Ok(())
    }

    pub async fn log_error(
        &self,
        error_type: &str,
        error_message: &str,
        stack_trace: Option<&str>,
        user_id: Option<&str>,
        channel_id: Option<&str>,
        command: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT INTO error_logs (error_type, error_message, stack_trace, user_id, channel_id, command, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )?;
        statement.bind((1, error_type))?;
        statement.bind((2, error_message))?;
        statement.bind((3, stack_trace.unwrap_or("")))?;
        statement.bind((4, user_id.unwrap_or("")))?;
        statement.bind((5, channel_id.unwrap_or("")))?;
        statement.bind((6, command.unwrap_or("")))?;
        statement.bind((7, metadata.unwrap_or("")))?;
        statement.next()?;

        // Also increment daily error count
        self.increment_daily_stat("error").await?;
        Ok(())
    }

    // Feature Flag Methods
    pub async fn set_feature_flag(
        &self,
        feature_name: &str,
        enabled: bool,
        user_id: Option<&str>,
        guild_id: Option<&str>,
    ) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT OR REPLACE INTO feature_flags (feature_name, enabled, user_id, guild_id, updated_at)
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)"
        )?;
        statement.bind((1, feature_name))?;
        statement.bind((2, if enabled { 1i64 } else { 0i64 }))?;
        statement.bind((3, user_id.unwrap_or("")))?;
        statement.bind((4, guild_id.unwrap_or("")))?;
        statement.next()?;
        Ok(())
    }

    pub async fn is_feature_enabled(&self, feature_name: &str, user_id: Option<&str>, guild_id: Option<&str>) -> Result<bool> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT enabled FROM feature_flags
             WHERE feature_name = ? AND user_id = ? AND guild_id = ?
             LIMIT 1"
        )?;
        statement.bind((1, feature_name))?;
        statement.bind((2, user_id.unwrap_or("")))?;
        statement.bind((3, guild_id.unwrap_or("")))?;

        if let Ok(State::Row) = statement.next() {
            let enabled = statement.read::<i64, _>(0)?;
            Ok(enabled == 1)
        } else {
            Ok(false)
        }
    }

    // Guild Settings Methods
    pub async fn set_guild_setting(&self, guild_id: &str, setting_key: &str, setting_value: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT OR REPLACE INTO guild_settings (guild_id, setting_key, setting_value, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)"
        )?;
        statement.bind((1, guild_id))?;
        statement.bind((2, setting_key))?;
        statement.bind((3, setting_value))?;
        statement.next()?;
        Ok(())
    }

    pub async fn get_guild_setting(&self, guild_id: &str, setting_key: &str) -> Result<Option<String>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT setting_value FROM guild_settings WHERE guild_id = ? AND setting_key = ?"
        )?;
        statement.bind((1, guild_id))?;
        statement.bind((2, setting_key))?;

        if let Ok(State::Row) = statement.next() {
            Ok(Some(statement.read::<String, _>(0)?))
        } else {
            Ok(None)
        }
    }

    // Extended User Preferences Methods
    pub async fn set_user_preference(&self, user_id: &str, preference_key: &str, preference_value: &str) -> Result<()> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "INSERT OR REPLACE INTO extended_user_preferences (user_id, preference_key, preference_value, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, preference_key))?;
        statement.bind((3, preference_value))?;
        statement.next()?;
        Ok(())
    }

    pub async fn get_user_preference(&self, user_id: &str, preference_key: &str) -> Result<Option<String>> {
        let conn = self.connection.lock().await;
        let mut statement = conn.prepare(
            "SELECT preference_value FROM extended_user_preferences WHERE user_id = ? AND preference_key = ?"
        )?;
        statement.bind((1, user_id))?;
        statement.bind((2, preference_key))?;

        if let Ok(State::Row) = statement.next() {
            Ok(Some(statement.read::<String, _>(0)?))
        } else {
            Ok(None)
        }
    }
}