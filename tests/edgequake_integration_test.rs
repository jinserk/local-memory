use anyhow::Result;
use local_memory::config::Config;
use local_memory::mcp::tools::{call_tool, McpContext};
use local_memory::model::nomic::MockEmbedder;
use local_memory::storage::sqlite::SqliteDatabase;
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;
use async_trait::async_trait;
use edgequake_llm::{LLMProvider, LLMResponse, ChatMessage, LlmError, CompletionOptions};

struct MockLLM;

#[async_trait]
impl LLMProvider for MockLLM {
    fn name(&self) -> &str { "mock" }
    fn model(&self) -> &str { "mock-model" }
    fn max_context_length(&self) -> usize { 4096 }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse, LlmError> {
        if prompt.contains("Alice works at Acme Corp") {
            Ok(LLMResponse {
                content: json!({
                    "entities": [
                        {"name": "Alice", "type": "Person", "description": "Software Engineer"},
                        {"name": "Acme Corp", "type": "Organization", "description": "A big company"}
                    ],
                    "relationships": [
                        {"source": "Alice", "target": "Acme Corp", "predicate": "WORKS_AT", "description": "Alice is employed by Acme Corp"}
                    ]
                }).to_string(),
                model: "mock-model".to_string(),
                prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
                finish_reason: Some("stop".to_string()), tool_calls: vec![],
                metadata: std::collections::HashMap::new(),
                cache_hit_tokens: Some(0), thinking_tokens: Some(0), thinking_content: None,
            })
        } else if prompt.contains("Acme Corp is located in San Francisco") {
            Ok(LLMResponse {
                content: json!({
                    "entities": [
                        {"name": "Acme Corp", "type": "Organization", "description": "A big company"},
                        {"name": "San Francisco", "type": "Location", "description": "A city in California"}
                    ],
                    "relationships": [
                        {"source": "Acme Corp", "target": "San Francisco", "predicate": "LOCATED_IN", "description": "Acme Corp headquarters are in SF"}
                    ]
                }).to_string(),
                model: "mock-model".to_string(),
                prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
                finish_reason: Some("stop".to_string()), tool_calls: vec![],
                metadata: std::collections::HashMap::new(),
                cache_hit_tokens: Some(0), thinking_tokens: Some(0), thinking_content: None,
            })
        } else {
             Ok(LLMResponse {
                content: "Mock response".to_string(),
                model: "mock-model".to_string(),
                prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
                finish_reason: Some("stop".to_string()), tool_calls: vec![],
                metadata: std::collections::HashMap::new(),
                cache_hit_tokens: Some(0), thinking_tokens: Some(0), thinking_content: None,
            })
        }
    }

    async fn complete_with_options(&self, prompt: &str, _options: &CompletionOptions) -> Result<LLMResponse, LlmError> {
        self.complete(prompt).await
    }

    async fn chat(&self, _messages: &[ChatMessage], _options: Option<&CompletionOptions>) -> Result<LLMResponse, LlmError> {
        self.complete("").await
    }
}

#[tokio::test]
async fn test_mcp_context_flow() -> Result<()> {
    let dir = tempdir()?;
    let db = Arc::new(SqliteDatabase::open(dir.path().join("test.db"))?);
    let context = McpContext {
        db: db.clone(),
        embedder: Arc::new(MockEmbedder),
        llm: Some(Arc::new(MockLLM)),
        config: Config::default(),
    };

    // 1. Ingestion
    context.get_pipeline().run("Alice works at Acme Corp.", json!({})).await?;
    
    // 2. Verification
    let count = db.count_entities()?;
    assert_eq!(count, 2);

    // 3. Tool call
    let args = json!({"entity_name": "Alice"});
    let tool_result = call_tool("graph_get_neighborhood", args, &context).await?;
    
    let content = tool_result["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Acme Corp"));

    Ok(())
}
