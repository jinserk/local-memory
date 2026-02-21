use anyhow::Result;
use local_memory::config::Config;
use local_memory::engine::funnel::SearchFunnel;
use local_memory::engine::ingestion::IngestionPipeline;
use local_memory::mcp::tools::{call_tool, list_tools};
use local_memory::model::nomic::NomicModel;
use local_memory::storage::db::Database;
use candle_core::Device;
use serde_json::{json, Value};
use std::io::{self, BufRead};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load();
    

    if !config.storage_path.exists() {
        std::fs::create_dir_all(&config.storage_path)?;
    }
    
    let db = Arc::new(Database::open(&config.storage_path)?);

    let device = Device::Cpu;
    let model_dir = config.model_path.clone();
    

    let embedder = Arc::new(NomicModel::load(
        model_dir.join("config.json"),
        model_dir.join("tokenizer.json"),
        model_dir.join("model.safetensors"),
        &device,
    )?);

    let pipeline = IngestionPipeline::new(embedder.clone(), db.clone());
    let funnel = SearchFunnel::new(&db, &config);

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();

    eprintln!("Local Memory MCP Server starting...");

    while handle.read_line(&mut line)? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Failed to parse request: {}", e);
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                });
                println!("{}", serde_json::to_string(&error_response)?);
                line.clear();
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");

        match method {
            "initialize" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "tools": {
                                "listChanged": true
                            }
                        },
                        "serverInfo": {
                            "name": "local-memory",
                            "version": "0.1.0"
                        }
                    }
                });
                println!("{}", serde_json::to_string(&response)?);
            }
            "tools/list" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": list_tools()
                    }
                });
                println!("{}", serde_json::to_string(&response)?);
            }
            "tools/call" => {
                let name = request.get("params").and_then(|p| p.get("name")).and_then(|n| n.as_str()).unwrap_or("");
                let arguments = request.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));
                
                match call_tool(name, arguments, &pipeline, &funnel, embedder.as_ref()) {
                    Ok(result) => {
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": result
                        });
                        println!("{}", serde_json::to_string(&response)?);
                    }
                    Err(e) => {
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32603,
                                "message": format!("Internal error: {}", e)
                            }
                        });
                        println!("{}", serde_json::to_string(&response)?);
                    }
                }
            }
            _ => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "status": "ok",
                        "message": format!("Method '{}' received", method)
                    }
                });
                println!("{}", serde_json::to_string(&response)?);
            }
        }

        line.clear();
    }

    Ok(())
}
