use crate::mcp::tools::McpContext;
use anyhow::Result;
use rusqlite::{params, Connection, OpenFlags};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::{json, Value};
use std::path::PathBuf;

pub async fn spawn_conversation_observer(context: Arc<McpContext>) {
    let mut last_processed_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let db_path = context.config.opencode_db_path.clone()
        .or_else(get_default_opencode_db_path);

    if db_path.is_none() {
        eprintln!("DEBUG: OpenCode database not found. Conversation observer disabled.");
        return;
    }
    let db_path = db_path.unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok(new_messages) = get_new_messages(&db_path, last_processed_time) {
                for (msg_id, role, time_created) in new_messages {
                    if role == "assistant" {
                        // When an assistant message is found, try to reconstruct the Q&A step
                        if let Ok(Some(step_text)) = reconstruct_conversation_step(&db_path, &msg_id) {
                            eprintln!("DEBUG: New conversation step detected (ID: {})", msg_id);
                            let _ = context.get_pipeline().run_with_namespace(
                                &step_text,
                                json!({"type": "conversation_step", "message_id": msg_id, "time": time_created}),
                                "conversation"
                            ).await;
                        }
                    }
                    if time_created > last_processed_time {
                        last_processed_time = time_created;
                    }
                }
            }
            sleep(Duration::from_secs(60)).await;
        }
    });
}

fn get_default_opencode_db_path() -> Option<PathBuf> {
    let home = home::home_dir()?;
    let db_path = home.join(".local/share/opencode/opencode.db");
    if db_path.exists() {
        Some(db_path)
    } else {
        None
    }
}

fn get_new_messages(db_path: &PathBuf, since_time: i64) -> Result<Vec<(String, String, i64)>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT id, json_extract(data, '$.role'), time_created 
         FROM message 
         WHERE time_created > ? 
         ORDER BY time_created ASC"
    )?;
    
    let rows = stmt.query_map(params![since_time], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

fn reconstruct_conversation_step(db_path: &PathBuf, assistant_msg_id: &str) -> Result<Option<String>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    
    // 1. Get current assistant message parts
    let mut assistant_text = String::new();
    let mut stmt = conn.prepare("SELECT data FROM part WHERE message_id = ? ORDER BY time_created ASC")?;
    let rows = stmt.query_map(params![assistant_msg_id], |row| row.get::<_, String>(0))?;

    for row in rows {
        let data: Value = serde_json::from_str(&row?)?;
        if data["type"] == "text" {
            if let Some(t) = data["text"].as_str() {
                assistant_text.push_str(t);
            }
        } else if data["type"] == "tool" {
            let tool = data["tool"].as_str().unwrap_or("unknown");
            let output = data["state"]["output"].as_str().unwrap_or("");
            assistant_text.push_str(&format!("\n[Tool: {}] -> {}\n", tool, output));
        }
    }

    if assistant_text.is_empty() {
        return Ok(None);
    }

    // 2. Find the preceding user message in the same session
    let mut stmt = conn.prepare(
        "SELECT m2.id 
         FROM message m1 
         JOIN message m2 ON m1.session_id = m2.session_id 
         WHERE m1.id = ? AND json_extract(m2.data, '$.role') = 'user' AND m2.time_created < m1.time_created 
         ORDER BY m2.time_created DESC LIMIT 1"
    )?;
    let user_msg_id: Option<String> = stmt.query_row(params![assistant_msg_id], |row| row.get(0)).ok();

    let mut user_text = String::new();
    if let Some(uid) = user_msg_id {
        let mut stmt = conn.prepare("SELECT data FROM part WHERE message_id = ? ORDER BY time_created ASC")?;
        let rows = stmt.query_map(params![uid], |row| row.get::<_, String>(0))?;
        for row in rows {
            let data: Value = serde_json::from_str(&row?)?;
            if data["type"] == "text"
                && let Some(t) = data["text"].as_str() {
                    user_text.push_str(t);
                }
        }
    }

    if user_text.is_empty() {
        Ok(Some(format!("Assistant: {}", assistant_text)))
    } else {
        Ok(Some(format!("User: {}\nAssistant: {}", user_text, assistant_text)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use uuid::Uuid;

    fn setup_mock_db(db_path: &PathBuf) -> Result<()> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE message (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                data TEXT,
                time_created INTEGER
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE part (
                id TEXT PRIMARY KEY,
                message_id TEXT,
                data TEXT,
                time_created INTEGER
            )",
            [],
        )?;
        Ok(())
    }

    fn insert_message(conn: &Connection, id: &str, session_id: &str, role: &str, time: i64) -> Result<()> {
        conn.execute(
            "INSERT INTO message (id, session_id, data, time_created) VALUES (?, ?, ?, ?)",
            params![id, session_id, json!({"role": role}).to_string(), time],
        )?;
        Ok(())
    }

    fn insert_text_part(conn: &Connection, msg_id: &str, text: &str, time: i64) -> Result<()> {
        conn.execute(
            "INSERT INTO part (id, message_id, data, time_created) VALUES (?, ?, ?, ?)",
            params![Uuid::new_v4().to_string(), msg_id, json!({"type": "text", "text": text}).to_string(), time],
        )?;
        Ok(())
    }

    fn insert_tool_part(conn: &Connection, msg_id: &str, tool: &str, output: &str, time: i64) -> Result<()> {
        conn.execute(
            "INSERT INTO part (id, message_id, data, time_created) VALUES (?, ?, ?, ?)",
            params![
                Uuid::new_v4().to_string(),
                msg_id,
                json!({
                    "type": "tool",
                    "tool": tool,
                    "state": {"output": output}
                }).to_string(),
                time
            ],
        )?;
        Ok(())
    }

    #[test]
    fn test_reconstruct_qa_step() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test_opencode.db");
        setup_mock_db(&db_path)?;
        let conn = Connection::open(&db_path)?;

        let session_id = "sess_1";
        
        // 1. User message
        insert_message(&conn, "u1", session_id, "user", 1000)?;
        insert_text_part(&conn, "u1", "What is Rust?", 1001)?;

        // 2. Assistant message with text and tool
        insert_message(&conn, "a1", session_id, "assistant", 2000)?;
        insert_text_part(&conn, "a1", "Rust is a language.", 2001)?;
        insert_tool_part(&conn, "a1", "google_search", "Rust details...", 2002)?;

        let step = reconstruct_conversation_step(&db_path, "a1")?.unwrap();
        assert!(step.contains("User: What is Rust?"));
        assert!(step.contains("Assistant: Rust is a language."));
        assert!(step.contains("[Tool: google_search] -> Rust details..."));

        Ok(())
    }

    #[test]
    fn test_get_new_messages() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("test_new_msgs.db");
        setup_mock_db(&db_path)?;
        let conn = Connection::open(&db_path)?;

        insert_message(&conn, "m1", "s1", "user", 100)?;
        insert_message(&conn, "m2", "s1", "assistant", 200)?;

        let msgs = get_new_messages(&db_path, 150)?;
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].0, "m2");
        assert_eq!(msgs[0].1, "assistant");

        Ok(())
    }
}
