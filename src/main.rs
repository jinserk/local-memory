use anyhow::Result;
use serde_json::{json, Value};
use std::io::{self, BufRead};

#[tokio::main]
async fn main() -> Result<()> {
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
