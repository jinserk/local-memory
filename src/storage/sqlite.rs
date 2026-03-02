use anyhow::Result;
use rusqlite::{params, Connection};
use sqlite_vec::sqlite3_vec_init;
use uuid::Uuid;
use zerocopy::IntoBytes;
use serde_json::{json, Value};

pub struct SqliteDatabase {
    conn: Connection,
    dimension: usize,
}

impl SqliteDatabase {
    pub fn open<P: AsRef<std::path::Path>>(path: P, dimension: usize) -> Result<Self> {
        // Register sqlite-vec as an auto-extension
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite3_vec_init as *const (),
            )));
        }

        let conn = Connection::open(path)?;
        
        // Enable WAL mode for better concurrency
        let _ = conn.pragma_update(None, "journal_mode", "WAL");

        let db = Self { conn, dimension };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> Result<()> {
        // Core tables
        self.conn.execute("CREATE TABLE IF NOT EXISTS documents (id TEXT PRIMARY KEY, title TEXT, content TEXT, metadata TEXT, created_at INTEGER)", [])?;
        self.conn.execute("CREATE TABLE IF NOT EXISTS entities (id TEXT PRIMARY KEY, name TEXT, type TEXT, description TEXT, metadata TEXT, UNIQUE(name, type))", [])?;
        self.conn.execute("CREATE TABLE IF NOT EXISTS relationships (id TEXT PRIMARY KEY, source_id TEXT, target_id TEXT, predicate TEXT, description TEXT, metadata TEXT, FOREIGN KEY(source_id) REFERENCES entities(id), FOREIGN KEY(target_id) REFERENCES entities(id))", [])?;

        // MULTI-STAGE VECTOR TABLES
        let s2_dim = self.dimension / 3;

        let sql_bit = format!("CREATE VIRTUAL TABLE IF NOT EXISTS vec_bit_docs USING vec0(id TEXT PRIMARY KEY, embedding bit[{}])", self.dimension);
        let sql_short = format!("CREATE VIRTUAL TABLE IF NOT EXISTS vec_short_docs USING vec0(id TEXT PRIMARY KEY, embedding float[{}])", s2_dim);
        let sql_full = format!("CREATE VIRTUAL TABLE IF NOT EXISTS vec_full_docs USING vec0(id TEXT PRIMARY KEY, embedding float[{}])", self.dimension);

        self.conn.execute(&sql_bit, [])?;
        self.conn.execute(&sql_short, [])?;
        self.conn.execute(&sql_full, [])?;

        Ok(())
    }

    pub fn insert_document(
        &self, 
        id: Uuid, 
        title: &str, 
        content: &str, 
        metadata: &Value, 
        v_full: &[f32],
        v_short: &[f32],
        v_bit: &[u8]
    ) -> Result<()> {
        let metadata_str = serde_json::to_string(metadata)?;
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        self.conn.execute(
            "INSERT INTO documents (id, title, content, metadata, created_at) VALUES (?, ?, ?, ?, ?)",
            params![id.to_string(), title, content, metadata_str, created_at],
        )?;

        self.conn.execute(
            "INSERT INTO vec_full_docs (id, embedding) VALUES (?, ?)",
            params![id.to_string(), v_full.as_bytes()],
        )?;
        self.conn.execute(
            "INSERT INTO vec_short_docs (id, embedding) VALUES (?, ?)",
            params![id.to_string(), v_short.as_bytes()],
        )?;
        self.conn.execute(
            "INSERT INTO vec_bit_docs (id, embedding) VALUES (?, vec_bit(?))",
            params![id.to_string(), v_bit],
        )?;

        Ok(())
    }

    pub fn search_stage1_bit(&self, query_bit: &[u8], limit: usize) -> Result<Vec<Uuid>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM vec_bit_docs WHERE embedding MATCH vec_bit(?) AND k = ? ORDER BY distance ASC"
        )?;
        let rows = stmt.query_map(params![query_bit, limit], |row| {
            let id_str: String = row.get(0)?;
            Ok(Uuid::parse_str(&id_str).unwrap())
        })?;
        
        let mut results = Vec::new();
        for row in rows { results.push(row?); }
        Ok(results)
    }

    pub fn search_stage2_short(&self, ids: &[Uuid], query_short: &[f32], limit: usize) -> Result<Vec<(Uuid, f32)>> {
        if ids.is_empty() { return Ok(vec![]); }
        let id_list: Vec<String> = ids.iter().map(|id| format!("'{}'", id)).collect();
        let sql = format!(
            "SELECT id, distance FROM vec_short_docs WHERE id IN ({}) AND embedding MATCH ? AND k = ? ORDER BY distance ASC",
            id_list.join(",")
        );
        
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![query_short.as_bytes(), limit], |row| {
            let id_str: String = row.get(0)?;
            Ok((Uuid::parse_str(&id_str).unwrap(), row.get(1)?))
        })?;

        let mut results = Vec::new();
        for row in rows { results.push(row?); }
        Ok(results)
    }

    pub fn get_document_content(&self, id: Uuid) -> Result<Option<(String, Value)>> {
        let mut stmt = self.conn.prepare("SELECT content, metadata FROM documents WHERE id = ?")?;
        let mut rows = stmt.query(params![id.to_string()])?;
        if let Some(row) = rows.next()? {
            let content: String = row.get(0)?;
            let meta_str: String = row.get(1)?;
            let meta: Value = serde_json::from_str(&meta_str)?;
            Ok(Some((content, meta)))
        } else {
            Ok(None)
        }
    }

    pub fn insert_entity(&self, name: &str, entity_type: &str, description: &str) -> Result<Uuid> {
        let mut stmt = self.conn.prepare("SELECT id FROM entities WHERE name = ?")?;
        if let Ok(id_str) = stmt.query_row(params![name], |r| r.get::<_, String>(0)) {
            return Ok(Uuid::parse_str(&id_str)?);
        }

        let id = Uuid::new_v4();
        self.conn.execute(
            "INSERT INTO entities (id, name, type, description, metadata) VALUES (?, ?, ?, ?, ?)",
            params![id.to_string(), name, entity_type, description, "{}"],
        )?;
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

    pub fn insert_relationship(&self, source_id: Uuid, target_id: Uuid, predicate: &str, description: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO relationships (id, source_id, target_id, predicate, description, metadata) VALUES (?, ?, ?, ?, ?, ?)",
            params![Uuid::new_v4().to_string(), source_id.to_string(), target_id.to_string(), predicate, description, "{}"],
        )?;
        Ok(())
    }

    pub fn get_neighborhood(&self, entity_name: &str) -> Result<Value> {
        let entity = self.get_entity_by_name(entity_name)?;
        if let Some((id, etype, desc)) = entity {
            let mut stmt = self.conn.prepare(
                "SELECT r.predicate, e.name FROM relationships r JOIN entities e ON r.target_id = e.id WHERE r.source_id = ?"
            )?;
            let relations = stmt.query_map(params![id.to_string()], |row| {
                Ok(json!({"predicate": row.get::<_, String>(0)?, "target": row.get::<_, String>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?;

            Ok(json!({
                "entity": {"name": entity_name, "type": etype, "description": desc},
                "relationships": relations
            }))
        } else {
            Ok(json!({"error": "Entity not found"}))
        }
    }

    pub fn count_entities(&self) -> Result<i64> {
        let count: i64 = self.conn.query_row("SELECT count(*) FROM entities", [], |r| r.get(0))?;
        Ok(count)
    }

    pub fn list_entities(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT name, type, description FROM entities LIMIT ?")?;
        let rows = stmt.query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        let mut results = Vec::new();
        for row in rows { results.push(row?); }
        Ok(results)
    }

    pub fn list_relationships(&self, limit: usize) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e1.name, r.predicate, e2.name FROM relationships r JOIN entities e1 ON r.source_id = e1.id JOIN entities e2 ON r.target_id = e2.id LIMIT ?"
        )?;
        let rows = stmt.query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        let mut results = Vec::new();
        for row in rows { results.push(row?); }
        Ok(results)
    }
}
