use std::sync::Arc;
use edgequake_llm::{LLMProvider, OpenAIProvider, OllamaProvider, EmbeddingProvider};
use crate::config::{Config, ExtractorProvider, ModelProvider};
use anyhow::Result;

pub mod base;
pub mod candle;
pub mod ollama;

// Re-export common types
pub use base::{UnifiedModel, GenericUnifiedModel, check_llm_connectivity};
pub use candle::CandleProvider;
pub use ollama::pull_ollama_model;

/// Unified factory to get a complete UnifiedModel (Embedding + LLM)
pub async fn get_unified_model(config: &Config) -> Result<Arc<dyn UnifiedModel>> {
    let mut prepare_list = Vec::new();
    let registry = candle::ModelRegistry::load()?;

    // 1. Resolve Embedder
    let embedder: Arc<dyn EmbeddingProvider> = match config.embedding.provider {
        ModelProvider::HuggingFace => {
            let p = CandleProvider::load(
                &config.embedding.name,
                &config.model_path,
                config.embedding.auto_download,
                &registry,
            )
            .await?;
            Arc::new(p)
        }
        ModelProvider::Ollama => {
            let host = config
                .embedding
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            if config.embedding.auto_download {
                prepare_list.push((config.embedding.name.clone(), host.clone()));
            }
            Arc::new(
                OllamaProvider::builder()
                    .host(host)
                    .embedding_model(&config.embedding.name)
                    .build()?,
            )
        }
        ModelProvider::Local => {
            anyhow::bail!("Local provider not yet implemented for standalone embedding");
        }
        ModelProvider::OpenAI => {
            let api_key = config
                .embedding
                .api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| anyhow::anyhow!("Missing API key for OpenAI embedding"))?;
            let base_url = config
                .embedding
                .base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            Arc::new(
                OpenAIProvider::compatible(api_key, base_url)
                    .with_embedding_model(&config.embedding.name),
            )
        }
    };

    // 2. Resolve LLM Extractor
    let llm: Arc<dyn LLMProvider> = if let Some(ext_config) = &config.llm_extractor {
        match ext_config.provider {
            ExtractorProvider::Ollama => {
                let host = ext_config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                if ext_config.auto_download {
                    prepare_list.push((ext_config.name.clone(), host.clone()));
                }
                Arc::new(
                    OllamaProvider::builder()
                        .host(host)
                        .model(&ext_config.name)
                        .build()?,
                )
            }
            ExtractorProvider::OpenAI => {
                let api_key = ext_config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| anyhow::anyhow!("Missing OpenAI API key"))?;
                let provider = OpenAIProvider::new(api_key.clone()).with_model(&ext_config.name);
                let provider = if let Some(base_url) = &ext_config.base_url {
                    OpenAIProvider::compatible(api_key, base_url.clone()).with_model(&ext_config.name)
                } else {
                    provider
                };
                Arc::new(provider)
            }
            ExtractorProvider::HuggingFace => {
                let p = CandleProvider::load(
                    &ext_config.name,
                    &config.model_path,
                    ext_config.auto_download,
                    &registry,
                )
                .await?;
                Arc::new(p)
            }
            _ => {
                anyhow::bail!("Unsupported extractor provider: {:?}", ext_config.provider);
            }
        }
    } else {
        // Default LLM: NuExtract-1.5 local
        let p = CandleProvider::load(
            "numind/NuExtract-1.5",
            &config.model_path,
            true,
            &registry,
        )
        .await?;
        Arc::new(p)
    };

    // 3. Return a Unified Model wrapper
    Ok(Arc::new(GenericUnifiedModel {
        llm,
        embedder,
        prepare_list,
        override_dimension: if config.embedding.provider == ModelProvider::OpenAI {
            Some(config.embedding.dimension)
        } else {
            None
        },
    }))
}

/// Unified factory to get just an LLM provider based on configuration.
pub fn get_llm_provider(config: &Config) -> Option<Arc<dyn LLMProvider + Send + Sync>> {
    if let Some(ext_config) = &config.llm_extractor {
        match ext_config.provider {
            ExtractorProvider::OpenAI => {
                let key = ext_config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())?;
                let mut p = OpenAIProvider::new(key);
                p = p.with_model(&ext_config.name);
                return Some(Arc::new(p));
            }
            ExtractorProvider::Ollama => {
                let host = ext_config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let p = edgequake_llm::OllamaProvider::builder()
                    .host(host)
                    .model(&ext_config.name)
                    .build()
                    .ok()?;
                return Some(Arc::new(p));
            }
            ExtractorProvider::HuggingFace => {
                // get_llm_provider is a sync function; CandleProvider::load is async.
                // Callers that need a sync handle should use get_unified_model instead.
                // Return None to signal "use get_unified_model".
                return None;
            }
            _ => {}
        }
    }
    None
}
