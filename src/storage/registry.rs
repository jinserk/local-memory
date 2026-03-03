use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

pub struct Registry {
    conn: Connection,
}

impl Registry {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let registry = Self { conn };
        registry.initialize()?;
        Ok(registry)
    }

    pub fn open_global() -> Result<Self> {
        let mut path = home::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".local-memory");
        std::fs::create_dir_all(&path)?;
        path.push("registry.db");
        Self::open(path)
    }

    fn initialize(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                path TEXT UNIQUE,
                db_path TEXT,
                last_accessed INTEGER
            )",
            [],
        )?;
        Ok(())
    }

    pub fn register_project(&self, project_path: &str, db_path: &str) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        self.conn.execute(
            "INSERT INTO projects (id, path, db_path, last_accessed) 
             VALUES (?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET db_path = ?, last_accessed = ?",
            params![
                uuid::Uuid::new_v4().to_string(),
                project_path,
                db_path,
                now,
                db_path,
                now
            ],
        )?;
        Ok(())
    }

    pub fn list_projects(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT path, db_path FROM projects ORDER BY last_accessed DESC")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        let mut results = Vec::new();
        for row in rows { results.push(row?); }
        Ok(results)
    }
}
