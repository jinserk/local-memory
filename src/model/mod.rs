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

    // 1. Resolve Embedder
    let embedder: Arc<dyn EmbeddingProvider> = match config.embedding.provider {
        ModelProvider::HuggingFace => {
            Arc::new(CandleProvider::new(
                &config.embedding.name,
                config.model_path.clone(),
                config.embedding.auto_download
            ))
        }
        ModelProvider::Ollama => {
            let host = config.embedding.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
            if config.embedding.auto_download {
                prepare_list.push((config.embedding.name.clone(), host.clone()));
            }
            Arc::new(OllamaProvider::builder()
                .host(host)
                .embedding_model(&config.embedding.name)
                .build()?)
        }
        ModelProvider::Local => {
            anyhow::bail!("Local provider not yet implemented for standalone embedding");
        }
    };

    // 2. Resolve LLM Extractor
    let llm: Arc<dyn LLMProvider> = if let Some(ext_config) = &config.llm_extractor {
        match ext_config.provider {
            ExtractorProvider::Ollama => {
                let host = ext_config.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                if ext_config.auto_download {
                    prepare_list.push((ext_config.name.clone(), host.clone()));
                }
                Arc::new(OllamaProvider::builder()
                    .host(host)
                    .model(&ext_config.name)
                    .build()?)
            },
            ExtractorProvider::OpenAI => {
                let api_key = ext_config.api_key.clone().or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| anyhow::anyhow!("Missing OpenAI API key"))?;
                Arc::new(OpenAIProvider::new(api_key).with_model(&ext_config.name))
            },
            ExtractorProvider::HuggingFace => {
                Arc::new(CandleProvider::new(
                    &ext_config.name,
                    config.model_path.clone(),
                    ext_config.auto_download
                ))
            },
            _ => {
                anyhow::bail!("Unsupported extractor provider: {:?}", ext_config.provider);
            }
        }
    } else {
        // Default LLM: NuExtract-1.5 local
        Arc::new(CandleProvider::new(
            "numind/NuExtract-1.5",
            config.model_path.clone(),
            true
        ))
    };

    // 3. Return a Unified Model wrapper
    Ok(Arc::new(GenericUnifiedModel {
        llm,
        embedder,
        prepare_list,
    }))
}

/// Unified factory to get just an LLM provider based on configuration.
pub fn get_llm_provider(config: &Config) -> Option<Arc<dyn LLMProvider + Send + Sync>> {
    if let Some(ext_config) = &config.llm_extractor {
        match ext_config.provider {
            ExtractorProvider::OpenAI => {
                let key = ext_config.api_key.clone().or_else(|| std::env::var("OPENAI_API_KEY").ok())?;
                let mut p = OpenAIProvider::new(key);
                p = p.with_model(&ext_config.name);
                return Some(Arc::new(p));
            }
            ExtractorProvider::Ollama => {
                let host = ext_config.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                let p = edgequake_llm::OllamaProvider::builder().host(host).model(&ext_config.name).build().ok()?;
                return Some(Arc::new(p));
            }
            ExtractorProvider::HuggingFace => {
                let provider = CandleProvider::new(
                    &ext_config.name,
                    config.model_path.clone(),
                    ext_config.auto_download
                );
                return Some(Arc::new(provider));
            }
            _ => {}
        }
    }
    None
}
