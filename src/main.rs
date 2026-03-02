use anyhow::Result;
use local_memory::mcp::tools::{call_tool, list_tools, McpContext};
use local_memory::model::{check_llm_connectivity, get_unified_model};
use local_memory::storage::SqliteDatabase;
use serde_json::{json, Value};
use std::io::{self, BufRead};
use std::sync::Arc;
use local_memory::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load();

    eprintln!("--- Local Memory Readiness Check ---");

    if !config.storage_path.exists() {
        let _ = std::fs::create_dir_all(&config.storage_path);
    }

    // 1. Initialize Unified Model (Handles both Embedding and Extractor)
    let model = get_unified_model(&config).await?;
    model.prepare().await?;
    
    // 2. Database Check
    let db_path = config.storage_path.join("local-memory.db");
    let dimension = model.dimension();
    let db = match SqliteDatabase::open(&db_path, dimension) {
        Ok(db) => {
            eprintln!("  ✓ Database ready (dimension: {})", dimension);
            Arc::new(db)
        },
        Err(e) => {
            eprintln!("  ! CRITICAL: Failed to open database: {}", e);
            return Err(e);
        }
    };

    // 3. Connectivity check
    match check_llm_connectivity(model.as_ref()).await {
        Ok(_) => eprintln!("  ✓ LLM extractor ready"),
        Err(e) => eprintln!("  ! Warning: LLM extractor check failed: {}.", e),
    }

    eprintln!("--- Readiness Check Complete ---\n");

    let context = Arc::new(McpContext {
        db,
        model,
        config,
    });

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();

    while handle.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let response = handle_request(trimmed, &context).await;
        if let Some(resp) = response {
            println!("{}", serde_json::to_string(&resp)?);
        }
        
        line.clear();
    }

    Ok(())
}

async fn handle_request(line: &str, context: &McpContext) -> Option<Value> {
    let request: Value = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            return Some(json!({
                "jsonrpc": "2.0",
                "error": {"code": -32700, "message": format!("Parse error: {}", e)},
                "id": null
            }));
        }
    };

    let id = request.get("id").cloned();
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let is_notification = id.is_none();

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "local-memory", "version": "0.2.0-edgequake"}
        })),
        "initialized" => return None,
        "tools/list" => Ok(json!({"tools": list_tools()})),
        "tools/call" => {
            let name = request.get("params").and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
            let arguments = request.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));
            call_tool(name, arguments, context).await
        }
        _ => {
            if is_notification { return None; }
            Ok(json!({"status": "ok", "message": format!("Method '{}' received", method)}))
        }
    };

    if is_notification {
        None
    } else {
        match result {
            Ok(res) => Some(json!({
                "jsonrpc": "2.0",
                "id": id.unwrap(),
                "result": res
            })),
            Err(e) => {
                eprintln!("ERROR: {}", e);
                Some(json!({
                    "jsonrpc": "2.0",
                    "id": id.unwrap(),
                    "error": {"code": -32603, "message": format!("{}", e)}
                }))
            }
        }
    }
}
