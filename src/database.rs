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
}