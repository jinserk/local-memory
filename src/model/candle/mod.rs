use std::path::Path;
use anyhow::Result;
use async_trait::async_trait;
use candle_core::Device;
use edgequake_llm::{
    ChatMessage, CompletionOptions, EmbeddingProvider, LLMProvider, LLMResponse, LlmError,
};
use tokenizers::Tokenizer;

pub mod backend;
pub mod backends;
pub mod loader;
pub mod registry;

pub use loader::{ensure_model_files, get_model_dir, pub_test_missing_files, pub_test_model_exists};
pub use registry::{ArchConfig, ArchKind, ModelRegistry};
pub use backend::{ModelBackend, ModelRole};

use backends::{BertBackend, Phi3Backend, Qwen2Backend};
/// A unified local provider backed by the Candle framework.
///
/// Handles both embedding (BERT-family) and generation (Phi3/Qwen2-family)
/// through a single struct. Construction is complete after `load()` — no
/// separate `prepare()` call is needed.
pub struct CandleProvider {
    model_name: String,
    device: Device,
    tokenizer: Tokenizer,
    backend: Box<dyn ModelBackend>,
    arch_config: ArchConfig,
}

impl CandleProvider {
    /// Build a fully-initialised `CandleProvider`.
    ///
    /// Resolves the architecture from the registry, downloads model files if
    /// necessary, loads weights into `device`, and returns a ready-to-use
    /// provider.
    pub async fn load(
        model_name: &str,
        model_path: &Path,
        auto_download: bool,
        registry: &ModelRegistry,
    ) -> Result<Self> {
        let model_dir = ensure_model_files(model_name, model_path, auto_download).await?;
        let device = Device::Cpu;

        let resolved = registry.resolve(model_name)?;

        let tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))
            .map_err(anyhow::Error::msg)?;

        let backend: Box<dyn ModelBackend> = match resolved.arch {
            ArchKind::Bert => {
                Box::new(BertBackend::load(&model_dir, &device)?)
            }
            ArchKind::Phi3 => {
                let gen_cfg = match &resolved.config {
                    ArchConfig::Generation(g) => g.clone(),
                    _ => anyhow::bail!("Expected generation config for Phi3"),
                };
                Box::new(Phi3Backend::load(&model_dir, &device, gen_cfg)?)
            }
            ArchKind::Qwen2 => {
                let gen_cfg = match &resolved.config {
                    ArchConfig::Generation(g) => g.clone(),
                    _ => anyhow::bail!("Expected generation config for Qwen2"),
                };
                Box::new(Qwen2Backend::load(&model_dir, &device, gen_cfg)?)
            }
        };

        Ok(Self {
            model_name: model_name.to_string(),
            device,
            tokenizer,
            backend,
            arch_config: resolved.config,
        })
    }

    /// Test-only constructor that accepts a pre-built backend.
    /// Avoids the need for real model files or network access in tests.
    /// Do not use in production code.
    #[doc(hidden)]
    pub fn from_parts(
        model_name: &str,
        tokenizer: Tokenizer,
        backend: Box<dyn ModelBackend>,
        arch_config: ArchConfig,
    ) -> Self {
        Self {
            model_name: model_name.to_string(),
            device: Device::Cpu,
            tokenizer,
            backend,
            arch_config,
        }
    }
}

// ── LLMProvider ───────────────────────────────────────────────────────────────

#[async_trait]
impl LLMProvider for CandleProvider {
    fn name(&self) -> &str {
        "huggingface"
    }
    fn model(&self) -> &str {
        &self.model_name
    }
    fn max_context_length(&self) -> usize {
        4096
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse, LlmError> {
        if self.backend.role() != ModelRole::Generation {
            return Err(LlmError::Unknown(format!(
                "Model '{}' is an embedding model and cannot generate text",
                self.model_name
            )));
        }
        let final_prompt = self.arch_config.format_prompt(prompt);
        let content = self
            .backend
            .generate(&final_prompt, &self.tokenizer, &self.device)
            .await?;
        let prompt_tokens = self
            .tokenizer
            .encode(final_prompt.as_str(), false)
            .map(|e| e.len())
            .unwrap_or(0);
        let completion_tokens = self
            .tokenizer
            .encode(content.as_str(), false)
            .map(|e| e.len())
            .unwrap_or(0);
        Ok(LLMResponse {
            content,
            model: self.model_name.clone(),
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            finish_reason: Some("stop".to_string()),
            tool_calls: vec![],
            metadata: std::collections::HashMap::new(),
            cache_hit_tokens: Some(0),
            thinking_tokens: Some(0),
            thinking_content: None,
        })
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        _options: &CompletionOptions,
    ) -> Result<LLMResponse, LlmError> {
        self.complete(prompt).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        _options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse, LlmError> {
        let last = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        self.complete(last).await
    }
}

// ── EmbeddingProvider ─────────────────────────────────────────────────────────

#[async_trait]
impl EmbeddingProvider for CandleProvider {
    fn name(&self) -> &str {
        "candle-embed"
    }
    fn model(&self) -> &str {
        &self.model_name
    }
    fn dimension(&self) -> usize {
        self.backend.dimension()
    }
    fn max_tokens(&self) -> usize {
        2048
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LlmError> {
        if self.backend.role() != ModelRole::Embedding {
            return Err(LlmError::Unknown(format!(
                "Model '{}' is a generation model and cannot produce embeddings",
                self.model_name
            )));
        }
        let embed_cfg = match &self.arch_config {
            ArchConfig::Embedding(e) => e,
            _ => {
                return Err(LlmError::Unknown(
                    "Expected embedding arch config".to_string(),
                ))
            }
        };

        let mut token_ids_batch = Vec::with_capacity(texts.len());
        let mut type_ids_batch = Vec::with_capacity(texts.len());

        for (i, text) in texts.iter().enumerate() {
            // Single-text calls are queries; batches are documents.
            let prefix = if texts.len() == 1 {
                embed_cfg.query_prefix.as_str()
            } else {
                embed_cfg.document_prefix.as_str()
            };
            let prefixed = format!("{}{}", prefix, text);
            let tokens = self
                .tokenizer
                .encode(prefixed.as_str(), true)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            token_ids_batch.push(tokens.get_ids().to_vec());
            type_ids_batch.push(tokens.get_type_ids().to_vec());
            let _ = i; // suppress unused warning
        }

        self.backend
            .embed_batch(&token_ids_batch, &type_ids_batch, &self.device)
            .await
    }
}

// ── UnifiedModel ──────────────────────────────────────────────────────────────

#[async_trait]
impl crate::model::UnifiedModel for CandleProvider {
    /// No-op: loading is completed inside `CandleProvider::load()`.
    async fn prepare(&self) -> Result<()> {
        Ok(())
    }
}
