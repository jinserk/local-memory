use anyhow::Result;
use local_memory::mcp::tools::{call_tool, list_tools, list_resources, read_resource, McpContext};
use local_memory::model::{check_llm_connectivity, get_unified_model};
use local_memory::storage::SqliteDatabase;
use local_memory::engine::git::spawn_git_observer;
use local_memory::engine::shell::spawn_shell_observer;
use local_memory::engine::conversation::spawn_conversation_observer;
use local_memory::engine::graph::spawn_graph_observer;
use local_memory::engine::communities::spawn_community_service;
use local_memory::engine::decay::spawn_decay_service;
use local_memory::KnowledgeEvent;
use serde_json::{json, Value};
use std::io::{self, BufRead};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{Duration, Instant};
use local_memory::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load();

    eprintln!("--- Local Memory Readiness Check ---");

    if !config.storage_path.exists() {
        let _ = std::fs::create_dir_all(&config.storage_path);
    }

    // Initialize Event Channel
    let (event_tx, _) = broadcast::channel::<KnowledgeEvent>(100);

    // 1. Initialize Unified Model
    let model = get_unified_model(&config).await?;
    model.prepare().await?;
    
    // 2. Database Check
    let db_path = config.storage_path.join("local-memory.db");
    
    if let Ok(registry) = local_memory::storage::registry::Registry::open_global() {
        let cwd = std::env::current_dir().unwrap_or_default();
        let _ = registry.register_project(
            &cwd.to_string_lossy(),
            &db_path.to_string_lossy()
        );
    }

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
        event_tx: event_tx.clone(),
    });

    // 4. Spawn Observers (Opt-in)
    if context.config.enable_observers {
        eprintln!("  ✓ Starting background observers (Git, Shell, Conversation, Graph, Community)");
        spawn_git_observer(context.clone()).await;
        spawn_shell_observer(context.clone()).await;
        spawn_conversation_observer(context.clone()).await;
        spawn_graph_observer(context.clone(), event_tx.subscribe()).await;
        spawn_community_service(context.clone(), event_tx.subscribe()).await;
        spawn_decay_service(context.clone()).await;
    }

    // 5. Idle Timeout Monitor
    let last_activity = Arc::new(RwLock::new(Instant::now()));
    let monitor_last_activity = last_activity.clone();
    let idle_timeout = context.config.idle_timeout_seconds;

    if idle_timeout > 0 {
        tokio::spawn(async move {
            let timeout_duration = Duration::from_secs(idle_timeout);
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let last = *monitor_last_activity.read().await;
                if last.elapsed() > timeout_duration {
                    eprintln!("[idle] Timeout reached ({}s). Terminating.", idle_timeout);
                    std::process::exit(0);
                }
            }
        });
    }

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();

    while handle.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        {
            let mut last = last_activity.write().await;
            *last = Instant::now();
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
            "capabilities": {
                "tools": {"listChanged": true},
                "resources": {"subscribe": true, "listChanged": true}
            },
            "serverInfo": {"name": "local-memory", "version": "0.3.0-supermemory"}
        })),
        "initialized" => return None,
        "tools/list" => Ok(json!({"tools": list_tools()})),
        "tools/call" => {
            let name = request.get("params").and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
            let arguments = request.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));
            call_tool(name, arguments, context).await
        }
        "resources/list" => Ok(json!({"resources": list_resources()})),
        "resources/read" => {
            let uri = request.get("params").and_then(|p| p.get("uri")).and_then(|u| u.as_str()).unwrap_or("");
            read_resource(uri, context).await
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
