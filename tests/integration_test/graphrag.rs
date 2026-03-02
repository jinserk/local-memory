use anyhow::Result;
use local_memory::config::Config;
use local_memory::mcp::tools::{call_tool, McpContext};
use local_memory::storage::SqliteDatabase;
use local_memory::model::UnifiedModel;
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;
use async_trait::async_trait;
use edgequake_llm::{LLMProvider, EmbeddingProvider, LLMResponse, ChatMessage, LlmError, CompletionOptions};

struct MockLLM;

#[async_trait]
impl LLMProvider for MockLLM {
    fn name(&self) -> &str { "mock" }
    fn model(&self) -> &str { "mock-model" }
    fn max_context_length(&self) -> usize { 4096 }

    async fn complete(&self, prompt: &str) -> std::result::Result<LLMResponse, LlmError> {
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

    async fn complete_with_options(&self, prompt: &str, _options: &CompletionOptions) -> std::result::Result<LLMResponse, LlmError> {
        self.complete(prompt).await
    }

    async fn chat(&self, messages: &[ChatMessage], _options: Option<&CompletionOptions>) -> std::result::Result<LLMResponse, LlmError> {
        let last_message = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        self.complete(last_message).await
    }
}

struct MockUnified {
    llm: MockLLM,
    dimension: usize,
}

#[async_trait]
impl LLMProvider for MockUnified {
    fn name(&self) -> &str { self.llm.name() }
    fn model(&self) -> &str { self.llm.model() }
    fn max_context_length(&self) -> usize { self.llm.max_context_length() }
    async fn complete(&self, prompt: &str) -> std::result::Result<LLMResponse, LlmError> { self.llm.complete(prompt).await }
    async fn complete_with_options(&self, prompt: &str, options: &CompletionOptions) -> std::result::Result<LLMResponse, LlmError> { self.llm.complete_with_options(prompt, options).await }
    async fn chat(&self, messages: &[ChatMessage], options: Option<&CompletionOptions>) -> std::result::Result<LLMResponse, LlmError> { self.llm.chat(messages, options).await }
}

#[async_trait]
impl EmbeddingProvider for MockUnified {
    fn name(&self) -> &str { "mock-embed" }
    fn model(&self) -> &str { "mock-model" }
    fn dimension(&self) -> usize { self.dimension }
    fn max_tokens(&self) -> usize { 2048 }
    async fn embed(&self, texts: &[String]) -> std::result::Result<Vec<Vec<f32>>, LlmError> {
        Ok(texts.iter().map(|_| vec![0.0; self.dimension]).collect())
    }
}

#[async_trait]
impl UnifiedModel for MockUnified {
    async fn prepare(&self) -> Result<()> { Ok(()) }
}

#[tokio::test]
async fn test_mcp_context_flow() -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test.db");
    let dimension = 768;
    let db = Arc::new(SqliteDatabase::open(&db_path, dimension)?);
    
    let model = Arc::new(MockUnified { llm: MockLLM, dimension });
    let context = McpContext {
        db: db.clone(),
        model: model.clone(),
        config: Config::default(),
    };

    // 1. Ingestion
    context.get_pipeline().run("Alice works at Acme Corp.", json!({})).await?;
    
    // 2. Verification
    let count = db.count_entities()?;
    assert_eq!(count, 2);

    // 3. Tool call (Neighborhood)
    let args = json!({"entity_name": "Alice"});
    let tool_result = call_tool("graph_get_neighborhood", args, &context).await?;
    
    let content = tool_result["content"][0]["text"].as_str().unwrap();
    assert!(content.contains("Acme Corp"));

    // 4. Search
    let search_args = json!({"query": "Who is Alice?"});
    let search_result = call_tool("memory_search", search_args, &context).await?;
    let search_content = search_result["content"][0]["text"].as_str().unwrap();
    assert!(search_content.contains("Alice"));

    Ok(())
}
