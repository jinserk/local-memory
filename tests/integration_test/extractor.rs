use anyhow::Result;
use local_memory::config::Config;
use local_memory::mcp::tools::McpContext;
use local_memory::storage::SqliteDatabase;
use local_memory::model::UnifiedModel;
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;
use async_trait::async_trait;
use edgequake_llm::{LLMProvider, EmbeddingProvider, LLMResponse, ChatMessage, LlmError, CompletionOptions};

struct MockExtractor;

#[async_trait]
impl LLMProvider for MockExtractor {
    fn name(&self) -> &str { "mock-extractor" }
    fn model(&self) -> &str { "mock-model" }
    fn max_context_length(&self) -> usize { 4096 }

    async fn complete(&self, _prompt: &str) -> std::result::Result<LLMResponse, LlmError> {
        // Verify NuExtract formatting was applied if we use the real pipeline
        // but here we just return a valid JSON response
        Ok(LLMResponse {
            content: json!({
                "entities": [
                    {"name": "Apple", "type": "Company", "description": "Tech giant"},
                    {"name": "Cupertino", "type": "Location", "description": "City in California"}
                ],
                "relationships": [
                    {"source": "Apple", "target": "Cupertino", "predicate": "BASED_IN", "description": "Headquarters location"}
                ]
            }).to_string(),
            model: "mock-model".to_string(),
            prompt_tokens: 0, completion_tokens: 0, total_tokens: 0,
            finish_reason: Some("stop".to_string()), tool_calls: vec![],
            metadata: std::collections::HashMap::new(),
            cache_hit_tokens: Some(0), thinking_tokens: Some(0), thinking_content: None,
        })
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
    extractor: MockExtractor,
    dimension: usize,
}

#[async_trait]
impl LLMProvider for MockUnified {
    fn name(&self) -> &str { self.extractor.name() }
    fn model(&self) -> &str { self.extractor.model() }
    fn max_context_length(&self) -> usize { self.extractor.max_context_length() }
    async fn complete(&self, prompt: &str) -> std::result::Result<LLMResponse, LlmError> { self.extractor.complete(prompt).await }
    async fn complete_with_options(&self, prompt: &str, options: &CompletionOptions) -> std::result::Result<LLMResponse, LlmError> { self.extractor.complete_with_options(prompt, options).await }
    async fn chat(&self, messages: &[ChatMessage], options: Option<&CompletionOptions>) -> std::result::Result<LLMResponse, LlmError> { self.extractor.chat(messages, options).await }
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
async fn test_ingestion_with_llm_extractor() -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("extractor_test.db");
    let dimension = 768;
    let db = Arc::new(SqliteDatabase::open(&db_path, dimension)?);
    
    let model = Arc::new(MockUnified { extractor: MockExtractor, dimension });
    let context = McpContext {
        db: db.clone(),
        model: model.clone(),
        config: Config::default(),
    };

    // Run ingestion
    let pipeline = context.get_pipeline();
    pipeline.run("Apple is based in Cupertino.", json!({})).await?;
    
    // Verify entities were extracted by our Mock LLM
    let entities = db.list_entities(10)?;
    let entity_names: Vec<String> = entities.into_iter().map(|(name, _, _)| name).collect();
    assert!(entity_names.contains(&"Apple".to_string()));
    assert!(entity_names.contains(&"Cupertino".to_string()));

    // Verify relationship was extracted
    let relations = db.list_relationships(10)?;
    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].0, "Apple");
    assert_eq!(relations[0].1, "BASED_IN");
    assert_eq!(relations[0].2, "Cupertino");

    Ok(())
}
