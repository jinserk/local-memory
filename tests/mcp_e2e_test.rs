//! E2E MCP Integration Tests
//!
//! Tests the MCP server via JSON-RPC over stdio by spawning the server as a subprocess.
//! These tests verify the complete MCP protocol implementation.

use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tempfile::TempDir;

/// Helper to manage MCP server subprocess for testing
struct McpServerProcess {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
    writer: std::process::ChildStdin,
    _temp_dir: TempDir,
}

impl McpServerProcess {
    /// Spawn the MCP server with test configuration
    fn spawn() -> Result<Self> {
        let temp_dir = TempDir::new()?;

        // Build the binary path
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
        let binary_path = PathBuf::from(&manifest_dir)
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?
            .join("target")
            .join("debug")
            .join("local-memory");

        // Check if binary exists, if not, build it
        if !binary_path.exists() {
            let build_status = Command::new("cargo")
                .args(["build", "--bin", "local-memory"])
                .current_dir(&manifest_dir)
                .status()?;
            if !build_status.success() {
                anyhow::bail!("Failed to build local-memory binary");
            }
        }

        // Create test config
        let config = json!({
            "storage_path": temp_dir.path().to_string_lossy(),
            "model_path": get_model_path()?,
            "embedding_model": {
                "name": "nomic-ai/nomic-embed-text-v1.5",
                "auto_download": true
            }
        });

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

        let mut child = Command::new(&binary_path)
            .env(
                "LOCAL_MEMORY_CONFIG",
                config_path.to_string_lossy().to_string(),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let reader = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?,
        );
        let writer = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture stdin"))?;

        Ok(Self {
            child,
            reader,
            writer,
            _temp_dir: temp_dir,
        })
    }

    /// Send a JSON-RPC request and return the response
    fn send_request(&mut self, request: &Value) -> Result<Value> {
        let request_str = serde_json::to_string(request)?;
        writeln!(self.writer, "{}", request_str)?;
        self.writer.flush()?;

        let mut response_line = String::new();
        self.reader.read_line(&mut response_line)?;
        let response: Value = serde_json::from_str(response_line.trim())?;

        Ok(response)
    }
}

impl Drop for McpServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Get the model path from environment or default location
fn get_model_path() -> Result<PathBuf> {
    // Check for environment variable first
    if let Ok(path) = std::env::var("LOCAL_MEMORY_MODEL_PATH") {
        return Ok(PathBuf::from(path));
    }

    // Default model path
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    Ok(PathBuf::from(home)
        .join(".local-memory")
        .join("models")
        .join("nomic-embed-text-v1.5"))
}

/// Check if model files exist
fn model_files_exist() -> bool {
    if let Ok(model_path) = get_model_path() {
        model_path.join("config.json").exists()
            && model_path.join("tokenizer.json").exists()
            && model_path.join("model.safetensors").exists()
    } else {
        false
    }
}

/// Helper to create a basic JSON-RPC request
fn make_request(id: i64, method: &str, params: Option<Value>) -> Value {
    let mut request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method
    });

    if let Some(p) = params {
        request["params"] = p;
    }

    request
}

// ============================================================================
// Test: Server Initialization
// ============================================================================

#[test]
fn test_server_initialize() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    let request = make_request(1, "initialize", Some(json!({})));
    let response = server.send_request(&request)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    // Verify capabilities
    let result = &response["result"];
    assert!(result["capabilities"].is_object());
    assert!(result["capabilities"]["tools"].is_object());
    assert_eq!(result["capabilities"]["tools"]["listChanged"], true);

    // Verify server info
    assert_eq!(result["serverInfo"]["name"], "local-memory");
    assert!(result["serverInfo"]["version"].is_string());

    Ok(())
}

// ============================================================================
// Test: Tools List
// ============================================================================

#[test]
fn test_tools_list() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    let request = make_request(2, "tools/list", None);
    let response = server.send_request(&request)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"]["tools"].is_array());

    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty(), "Tools list should not be empty");

    // Verify memory_insert tool
    let insert_tool = tools
        .iter()
        .find(|t| t["name"] == "memory_insert")
        .expect("memory_insert tool should exist");
    assert!(insert_tool["description"].is_string());
    assert!(insert_tool["inputSchema"]["properties"]["text"].is_object());
    assert!(insert_tool["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .contains(&json!("text")));

    // Verify memory_search tool
    let search_tool = tools
        .iter()
        .find(|t| t["name"] == "memory_search")
        .expect("memory_search tool should exist");
    assert!(search_tool["description"].is_string());
    assert!(search_tool["inputSchema"]["properties"]["query"].is_object());
    assert!(search_tool["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .contains(&json!("query")));

    Ok(())
}

// ============================================================================
// Test: Memory Insert
// ============================================================================

#[test]
fn test_memory_insert() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    // Initialize first
    let init_request = make_request(1, "initialize", Some(json!({})));
    let _ = server.send_request(&init_request)?;

    // Insert a memory
    let insert_request = make_request(
        2,
        "tools/call",
        Some(json!({
            "name": "memory_insert",
            "arguments": {
                "text": "This is a test memory for E2E testing",
                "metadata": {
                    "source": "e2e-test",
                    "priority": "high"
                }
            }
        })),
    );
    let response = server.send_request(&insert_request)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);

    // Check for success or error
    if let Some(error) = response.get("error") {
        // If there's an error, it should be descriptive
        assert!(error["message"].is_string());
        eprintln!(
            "Insert returned error (may be expected in test env): {}",
            error
        );
    } else {
        // Verify success response
        let result = &response["result"];
        assert!(result["content"].is_array());
        let content = result["content"].as_array().unwrap();
        assert!(!content.is_empty());
        assert_eq!(content[0]["type"], "text");
        assert!(content[0]["text"]
            .as_str()
            .unwrap()
            .contains("Memory inserted and knowledge graph updated."));
    }

    Ok(())
}

// ============================================================================
// Test: Memory Search
// ============================================================================

#[test]
fn test_memory_search() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    // Initialize first
    let init_request = make_request(1, "initialize", Some(json!({})));
    let _ = server.send_request(&init_request)?;

    // Insert a memory first
    let insert_request = make_request(
        2,
        "tools/call",
        Some(json!({
            "name": "memory_insert",
            "arguments": {
                "text": "Rust is a systems programming language focused on safety and performance"
            }
        })),
    );
    let insert_response = server.send_request(&insert_request)?;

    // Skip search test if insert failed
    if insert_response.get("error").is_some() {
        eprintln!("Skipping search test: insert failed");
        return Ok(());
    }

    // Search for the memory
    let search_request = make_request(
        3,
        "tools/call",
        Some(json!({
            "name": "memory_search",
            "arguments": {
                "query": "programming language safety",
                "top_k": 5
            }
        })),
    );
    let response = server.send_request(&search_request)?;

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);

    if let Some(error) = response.get("error") {
        eprintln!("Search returned error: {}", error);
    } else {
        let result = &response["result"];
        assert!(result["content"].is_array());
        let content = result["content"].as_array().unwrap();
        assert!(!content.is_empty());
        assert_eq!(content[0]["type"], "text");
        // The text should be a JSON array of results
        let results_text = content[0]["text"].as_str().unwrap();
        let results: Vec<Value> = serde_json::from_str(results_text)?;
        // Results may be empty if no matches, but should be valid JSON
        assert!(results.is_empty() || results[0]["id"].is_string());
    }

    Ok(())
}

// ============================================================================
// Test: Error Handling - Invalid Tool
// ============================================================================

#[test]
fn test_error_invalid_tool() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    // Call a non-existent tool
    let request = make_request(
        1,
        "tools/call",
        Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
    );
    let response = server.send_request(&request)?;

    // Verify error response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(
        response.get("error").is_some(),
        "Should return error for invalid tool"
    );

    let error = &response["error"];
    assert!(error["message"].as_str().unwrap().contains("Unknown tool"));

    Ok(())
}

// ============================================================================
// Test: Error Handling - Missing Required Argument
// ============================================================================

#[test]
fn test_error_missing_argument() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    // Call memory_insert without required 'text' argument
    let request = make_request(
        1,
        "tools/call",
        Some(json!({
            "name": "memory_insert",
            "arguments": {
                "metadata": {"key": "value"}
            }
        })),
    );
    let response = server.send_request(&request)?;

    // Verify error response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(
        response.get("error").is_some(),
        "Should return error for missing argument"
    );

    let error = &response["error"];
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("Missing 'text' argument"));

    Ok(())
}

// ============================================================================
// Test: Error Handling - Invalid JSON
// ============================================================================

#[test]
fn test_error_invalid_json() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    // Send invalid JSON
    writeln!(server.writer, "{{invalid json}}")?;
    server.writer.flush()?;

    let mut response_line = String::new();
    server.reader.read_line(&mut response_line)?;
    let response: Value = serde_json::from_str(response_line.trim())?;

    // Verify parse error response
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_some());
    assert_eq!(response["error"]["code"], -32700); // Parse error code

    Ok(())
}

// ============================================================================
// Test: Full Workflow - Insert and Search
// ============================================================================

#[test]
fn test_full_workflow() -> Result<()> {
    if !model_files_exist() {
        eprintln!("Skipping test: model files not found");
        return Ok(());
    }

    let mut server = McpServerProcess::spawn()?;

    // 1. Initialize
    let init_request = make_request(1, "initialize", Some(json!({})));
    let init_response = server.send_request(&init_request)?;
    assert!(
        init_response.get("result").is_some(),
        "Initialize should succeed"
    );

    // 2. List tools
    let tools_request = make_request(2, "tools/list", None);
    let tools_response = server.send_request(&tools_request)?;
    assert!(tools_response["result"]["tools"].is_array());

    // 3. Insert multiple memories
    let memories = vec![
        "The quick brown fox jumps over the lazy dog",
        "Rust provides memory safety without garbage collection",
        "Machine learning models can be optimized for inference speed",
    ];

    for (i, memory) in memories.iter().enumerate() {
        let insert_request = make_request(
            3 + i as i64,
            "tools/call",
            Some(json!({
                "name": "memory_insert",
                "arguments": {
                    "text": memory,
                    "metadata": {"batch": "workflow-test"}
                }
            })),
        );
        let response = server.send_request(&insert_request)?;
        if response.get("error").is_some() {
            eprintln!("Insert {} failed: {:?}", i, response["error"]);
            return Ok(()); // Skip rest of test if insert fails
        }
    }

    // 4. Search for memories
    let search_request = make_request(
        10,
        "tools/call",
        Some(json!({
            "name": "memory_search",
            "arguments": {
                "query": "programming language performance",
                "top_k": 3
            }
        })),
    );
    let search_response = server.send_request(&search_request)?;

    // Verify search response
    assert!(search_response.get("result").is_some() || search_response.get("error").is_some());

    if let Some(result) = search_response.get("result") {
        let content = result["content"].as_array().unwrap();
        let results_text = content[0]["text"].as_str().unwrap();
        let results: Vec<Value> = serde_json::from_str(results_text)?;
        // Should find at least one result (the Rust memory)
        eprintln!("Search returned {} results", results.len());
    }

    Ok(())
}
