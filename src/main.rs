use anyhow::Result;
use local_memory::config::Config;
use local_memory::engine::funnel::SearchFunnel;
use local_memory::engine::ingestion::IngestionPipeline;
use local_memory::mcp::tools::{call_tool, list_tools};
use local_memory::model::nomic::NomicModel;
use local_memory::model::downloader::ensure_model_files;
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

    // Ensure model files are available (download if needed)
    if config.model.auto_download {
        if let Err(e) = ensure_model_files(&config.model.name, &config.model_path, config.model.auto_download) {
            eprintln!("Warning: Failed to ensure model files: {}", e);
        }
    }

    // Try to load the model, but don't crash if it fails
    let embedder = match load_model(&config.model_path, &config.model.name) {
        Ok(model) => Some(Arc::new(model)),
        Err(e) => {
            eprintln!("Warning: Failed to load model: {}", e);
            eprintln!("The server will start, but memory_insert/memory_search will not work until model files are available.");
            None
        }
    };

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
                        "protocolVersion": "2024-11-05",
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

                // Check if model is available for tools that need it
                match name {
                    "memory_insert" | "memory_search" if embedder.is_none() => {
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": {
                                "code": -32603,
                                "message": "Model not loaded. Please ensure model files are in the 'models/' directory."
                            }
                        });
                        println!("{}", serde_json::to_string(&response)?);
                    }
                    _ => {
                        // For memory_insert/memory_search, we need embedder
                        if name == "memory_insert" || name == "memory_search" {
                            let embedder = match embedder.as_ref() {
                                Some(e) => e,
                                None => {
                                    let response = json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {
                                            "code": -32603,
                                            "message": "Model not loaded. Please ensure model files are in the 'models/' directory."
                                        }
                                    });
                                    println!("{}", serde_json::to_string(&response)?);
                                    continue;
                                }
                            };
                            let pipeline = IngestionPipeline::new(embedder.clone(), db.clone());
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
                        } else {
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -32601,
                                    "message": format!("Unknown tool: {}", name)
                                }
                            });
                            println!("{}", serde_json::to_string(&response)?);
                        }
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

fn load_model(model_dir: &std::path::PathBuf, _model_name: &str) -> Result<NomicModel> {
    let device = Device::Cpu;
    NomicModel::load(
        model_dir.join("config.json"),
        model_dir.join("tokenizer.json"),
        model_dir.join("model.safetensors"),
        &device,
    )
}
