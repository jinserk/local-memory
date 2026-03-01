use anyhow::Result;
use rusqlite::{params, Connection};
use sqlite_vec::sqlite3_vec_init;
use uuid::Uuid;
use zerocopy::IntoBytes;
use serde_json::Value;

pub struct SqliteDatabase {
    conn: Connection,
}

impl SqliteDatabase {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        // Register sqlite-vec as an auto-extension
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> Result<()> {
        // Core tables
        self.conn.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                title TEXT,
                content TEXT,
                metadata TEXT,
                created_at INTEGER
             );
             CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT,
                type TEXT,
                description TEXT,
                metadata TEXT,
                UNIQUE(name, type)
             );
             CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                source_id TEXT,
                target_id TEXT,
                predicate TEXT,
                description TEXT,
                metadata TEXT,
                FOREIGN KEY(source_id) REFERENCES entities(id),
                FOREIGN KEY(target_id) REFERENCES entities(id)
             );
             CREATE TABLE IF NOT EXISTS communities (
                id TEXT PRIMARY KEY,
                name TEXT,
                summary TEXT,
                metadata TEXT
             );
             COMMIT;"
        )?;

        // Vector tables (using vec0)
        // We use 768 dimensions for Nomic Embed Text v1.5
        self.conn.execute_batch(
            "BEGIN;
             CREATE VIRTUAL TABLE IF NOT EXISTS vec_documents USING vec0(
                id TEXT PRIMARY KEY,
                embedding float[768]
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS vec_entities USING vec0(
                id TEXT PRIMARY KEY,
                embedding float[768]
             );
             CREATE VIRTUAL TABLE IF NOT EXISTS vec_bit_documents USING vec0(
                id TEXT PRIMARY KEY,
                embedding bit[768]
             );
             COMMIT;"
        )?;

        Ok(())
    }

    pub fn insert_document(&self, id: Uuid, title: &str, content: &str, metadata: &Value, vector: &[f32]) -> Result<()> {
        let metadata_str = serde_json::to_string(metadata)?;
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        self.conn.execute(
            "INSERT INTO documents (id, title, content, metadata, created_at) VALUES (?, ?, ?, ?, ?)",
            params![id.to_string(), title, content, metadata_str, created_at],
        )?;

        self.conn.execute(
            "INSERT INTO vec_documents (id, embedding) VALUES (?, ?)",
            params![id.to_string(), vector.as_bytes()],
        )?;

        Ok(())
    }

    pub fn search_documents(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32, Value)>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, v.distance, d.metadata 
             FROM vec_documents v
             JOIN documents d ON v.id = d.id
             WHERE v.embedding MATCH ? AND k = ?
             ORDER BY v.distance ASC",
        )?;

        let rows = stmt.query_map(params![query_vector.as_bytes(), top_k], |row| {
            let id_str: String = row.get(0)?;
            let id = Uuid::parse_str(&id_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let distance: f32 = row.get(1)?;
            let metadata_str: String = row.get(2)?;
            let metadata: Value = serde_json::from_str(&metadata_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok((id, distance, metadata))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn count_entities(&self) -> Result<i64> {
        let mut stmt = self.conn.prepare("SELECT count(*) FROM entities")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    pub fn list_entities(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT name, type, description FROM entities LIMIT ?")?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn list_relationships(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e1.name, r.predicate, e2.name 
             FROM relationships r
             JOIN entities e1 ON r.source_id = e1.id
             JOIN entities e2 ON r.target_id = e2.id
             LIMIT ?"
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn insert_entity(&self, name: &str, entity_type: &str, description: &str, metadata: &Value, vector: Option<&[f32]>) -> Result<Uuid> {
        // Check if entity already exists
        if let Some((id, _, _)) = self.get_entity_by_name(name)? {
            return Ok(id);
        }

        let id = Uuid::new_v4();
        let metadata_str = serde_json::to_string(metadata)?;

        self.conn.execute(
            "INSERT INTO entities (id, name, type, description, metadata) VALUES (?, ?, ?, ?, ?)",
            params![id.to_string(), name, entity_type, description, metadata_str],
        )?;

        if let Some(v) = vector {
            self.conn.execute(
                "INSERT OR REPLACE INTO vec_entities (id, embedding) VALUES (?, ?)",
                params![id.to_string(), v.as_bytes()],
            )?;
        }

        Ok(id)
    }

    pub fn get_entity_by_name(&self, name: &str) -> Result<Option<(Uuid, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, type, description FROM entities WHERE name = ?")?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            let id_str: String = row.get(0)?;
            let id = Uuid::parse_str(&id_str)?;
            let entity_type: String = row.get(1)?;
            let description: String = row.get(2)?;
            Ok(Some((id, entity_type, description)))
        } else {
            Ok(None)
        }
    }

    pub fn insert_relationship(&self, source_id: Uuid, target_id: Uuid, predicate: &str, description: &str, metadata: &Value) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let metadata_str = serde_json::to_string(metadata)?;

        self.conn.execute(
            "INSERT INTO relationships (id, source_id, target_id, predicate, description, metadata) VALUES (?, ?, ?, ?, ?, ?)",
            params![id.to_string(), source_id.to_string(), target_id.to_string(), predicate, description, metadata_str],
        )?;

        Ok(id)
    }

    pub fn get_neighborhood(&self, entity_name: &str) -> Result<Value> {
        let entity = self.get_entity_by_name(entity_name)?;
        if entity.is_none() {
            return Ok(serde_json::json!({"error": "Entity not found"}));
        }
        let (id, entity_type, description) = entity.unwrap();

        // Get outbound relationships
        let mut stmt = self.conn.prepare(
            "SELECT r.predicate, e.name, e.type, r.description 
             FROM relationships r
             JOIN entities e ON r.target_id = e.id
             WHERE r.source_id = ?"
        )?;
        let outbound = stmt.query_map(params![id.to_string()], |row| {
            Ok(serde_json::json!({
                "predicate": row.get::<_, String>(0)?,
                "target": row.get::<_, String>(1)?,
                "target_type": row.get::<_, String>(2)?,
                "description": row.get::<_, String>(3)?
            }))
        })?.collect::<Result<Vec<_>, _>>()?;

        // Get inbound relationships
        let mut stmt = self.conn.prepare(
            "SELECT r.predicate, e.name, e.type, r.description 
             FROM relationships r
             JOIN entities e ON r.source_id = e.id
             WHERE r.target_id = ?"
        )?;
        let inbound = stmt.query_map(params![id.to_string()], |row| {
            Ok(serde_json::json!({
                "predicate": row.get::<_, String>(0)?,
                "source": row.get::<_, String>(1)?,
                "source_type": row.get::<_, String>(2)?,
                "description": row.get::<_, String>(3)?
            }))
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(serde_json::json!({
            "entity": {
                "name": entity_name,
                "type": entity_type,
                "description": description
            },
            "relationships": {
                "outbound": outbound,
                "inbound": inbound
            }
        }))
    }
}
