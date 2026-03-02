//! E2E MCP Integration Tests
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use tempfile::TempDir;

struct McpTestServer {
    child: Child,
    #[allow(dead_code)]
    temp_dir: TempDir,
}

impl McpTestServer {
    fn spawn() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let cwd = std::env::current_dir()?;
        
        let mut bin_path = cwd.clone();
        bin_path.push("target");
        bin_path.push("debug");
        bin_path.push("local-memory");

        if !bin_path.exists() {
            bin_path.pop();
            bin_path.push("release");
            bin_path.push("local-memory");
        }

        if !bin_path.exists() {
            anyhow::bail!("Could not find local-memory binary. Please build the project first.");
        }

        // Use real models dir structure
        let mut base_models_path = cwd.clone();
        base_models_path.push(".local-memory");
        base_models_path.push("models");

        // Create storage directory explicitly
        let storage_path = temp_dir.path().join("storage");
        std::fs::create_dir_all(&storage_path)?;

        // Create test config using Nomic 1.5
        let config = json!({
            "storage_path": storage_path.to_string_lossy(),
            "model_path": base_models_path.to_string_lossy(),
            "embedding": {
                "name": "nomic-ai/nomic-embed-text-v1.5",
                "auto_download": true, 
                "dimension": 768
            },
            "llm_extractor": {
                "provider": "huggingface",
                "name": "phi-3-mini-4k-instruct"
            }
        });

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

        let child = Command::new(&bin_path)
            .current_dir(&cwd)
            .env("LOCAL_MEMORY_CONFIG", config_path.to_string_lossy().as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(Self { child, temp_dir })
    }

    fn send_request(&mut self, request: Value) -> Result<Value> {
        let stdin = self.child.stdin.as_mut().unwrap();
        let stdout = self.child.stdout.as_mut().unwrap();
        let mut reader = BufReader::new(stdout);

        let request_str = serde_json::to_string(&request)?;
        writeln!(stdin, "{}", request_str)?;
        stdin.flush()?;

        let mut response_line = String::new();
        loop {
            response_line.clear();
            let bytes_read = reader.read_line(&mut response_line)?;
            if bytes_read == 0 {
                anyhow::bail!("Server closed connection unexpectedly");
            }
            
            let trimmed = response_line.trim();
            if trimmed.is_empty() {
                continue;
            }
            
            if trimmed.starts_with('{') {
                return Ok(serde_json::from_str(trimmed)?);
            }
        }
    }
}

impl Drop for McpTestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn test_server_initialize() -> Result<()> {
    let mut server = McpTestServer::spawn()?;
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "1.0.0" }
        }
    });

    let response = server.send_request(request)?;
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["serverInfo"]["name"], "local-memory");
    Ok(())
}

#[test]
fn test_tools_list() -> Result<()> {
    let mut server = McpTestServer::spawn()?;
    
    server.send_request(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "t", "version": "1" } }
    }))?;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let response = server.send_request(request)?;
    let tools = response["result"]["tools"].as_array().expect("tools field missing or not an array");
    assert!(tools.iter().any(|t| t["name"] == "memory_insert"));
    assert!(tools.iter().any(|t| t["name"] == "memory_search"));
    Ok(())
}
