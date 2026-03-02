use anyhow::Result;
use std::sync::Arc;
use async_trait::async_trait;
use edgequake_llm::{LLMProvider, EmbeddingProvider, LLMResponse, LlmError, ChatMessage, CompletionOptions};

/// A unified interface for models that can perform both text generation and embedding.
#[async_trait]
pub trait UnifiedModel: LLMProvider + EmbeddingProvider {
    /// Prepare the model (download files, pull images, etc.)
    async fn prepare(&self) -> Result<()>;
}

/// A generic wrapper that combines any LLMProvider and EmbeddingProvider into a UnifiedModel.
pub struct GenericUnifiedModel {
    pub llm: Arc<dyn LLMProvider>,
    pub embedder: Arc<dyn EmbeddingProvider>,
    /// List of (model_name, base_url) to pull via Ollama during prepare()
    pub prepare_list: Vec<(String, String)>,
}

#[async_trait]
impl LLMProvider for GenericUnifiedModel {
    fn name(&self) -> &str { self.llm.name() }
    fn model(&self) -> &str { self.llm.model() }
    fn max_context_length(&self) -> usize { self.llm.max_context_length() }
    async fn complete(&self, prompt: &str) -> Result<LLMResponse, LlmError> {
        self.llm.complete(prompt).await
    }
    async fn complete_with_options(&self, prompt: &str, options: &CompletionOptions) -> Result<LLMResponse, LlmError> {
        self.llm.complete_with_options(prompt, options).await
    }
    async fn chat(&self, messages: &[ChatMessage], options: Option<&CompletionOptions>) -> Result<LLMResponse, LlmError> {
        self.llm.chat(messages, options).await
    }
}

#[async_trait]
impl EmbeddingProvider for GenericUnifiedModel {
    fn name(&self) -> &str { self.embedder.name() }
    fn model(&self) -> &str { self.embedder.model() }
    fn dimension(&self) -> usize { self.embedder.dimension() }
    fn max_tokens(&self) -> usize { self.embedder.max_tokens() }
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LlmError> {
        self.embedder.embed(texts).await
    }
}

#[async_trait]
impl UnifiedModel for GenericUnifiedModel {
    async fn prepare(&self) -> Result<()> {
        for (model_name, host) in &self.prepare_list {
            crate::model::ollama::pull_ollama_model(host, model_name).await?;
        }
        Ok(())
    }
}

/// Utility to check if an LLM provider is reachable
pub async fn check_llm_connectivity(provider: &dyn LLMProvider) -> Result<()> {
    if provider.name() == "huggingface" { return Ok(()); }
    provider.complete("ping").await
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("LLM connectivity check failed: {}", e))
}
