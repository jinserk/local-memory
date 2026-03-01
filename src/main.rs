use anyhow::Result;
use local_memory::config::Config;
use local_memory::mcp::tools::{call_tool, list_tools, McpContext};
use local_memory::model::nomic::{NomicModel, MockEmbedder, Embedder};
use local_memory::model::downloader::ensure_model_files;
use local_memory::model::llm::{get_llm_provider, check_llm_connectivity};
use local_memory::storage::SqliteDatabase;
use candle_core::Device;
use serde_json::{json, Value};
use std::io::{self, BufRead};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load();

    eprintln!("--- Local Memory Readiness Check ---");

    // 1. Storage & Database Check
    if !config.storage_path.exists() {
        let _ = std::fs::create_dir_all(&config.storage_path);
    }

    let db_path = config.storage_path.join("local-memory.db");
    let db = match SqliteDatabase::open(&db_path) {
        Ok(db) => {
            eprintln!("  ✓ Database ready");
            Arc::new(db)
        },
        Err(e) => {
            eprintln!("  ! CRITICAL: Failed to open database: {}", e);
            return Err(e);
        }
    };
    
    let llm = get_llm_provider(&config);

    if config.embedding_model.auto_download {
        let _ = ensure_model_files(&config.embedding_model.name, &config.model_path, true);
    }

    let embedder: Arc<dyn Embedder + Send + Sync> = match load_model(&config.model_path) {
        Ok(model) => {
            eprintln!("  ✓ Local embedder loaded");
            Arc::new(model)
        },
        Err(e) => {
            eprintln!("  ! Warning: Local embedder load failed ({}). Using MockEmbedder.", e);
            Arc::new(MockEmbedder)
        }
    };

    if let Some(ref provider) = llm {
        match check_llm_connectivity(provider.as_ref()).await {
            Ok(_) => eprintln!("  ✓ LLM extractor connected"),
            Err(e) => eprintln!("  ! Warning: LLM extractor check failed: {}.", e),
        }
    }

    eprintln!("--- Readiness Check Complete ---\n");

    let context = Arc::new(McpContext {
        db,
        embedder,
        llm,
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

fn load_model(model_dir: &std::path::PathBuf) -> Result<NomicModel> {
    let device = Device::Cpu;
    NomicModel::load(
        model_dir.join("config.json"),
        model_dir.join("tokenizer.json"),
        model_dir.join("model.safetensors"),
        &device,
    )
}
