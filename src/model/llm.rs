use std::sync::Arc;
use edgequake_llm::{OpenAIProvider, LLMProvider, AnthropicProvider, GeminiProvider, OllamaProvider};
use crate::config::{Config, ExtractorProvider};
use anyhow::Result;

pub async fn check_and_pull_llm(provider: &dyn LLMProvider) -> Result<()> {
    // 1. Check connectivity/existence
    if let Err(_) = provider.complete("ping").await {
        eprintln!("  ! LLM model not found or provider unreachable. Attempting to pull/verify...");
        
        // 2. Try to pull the model if it's Ollama
        // Note: edgequake-llm might not have pull_model exposed yet in 0.2.9, 
        // we'll check if we can simulate it or just log it.
        // For now, let's just do a second check.
        match provider.complete("ping").await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("LLM check failed after retry: {}", e)),
        }
    } else {
        Ok(())
    }
}

pub async fn check_llm_connectivity(provider: &dyn LLMProvider) -> Result<()> {
    match provider.complete("ping").await {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("LLM connectivity check failed: {}", e)),
    }
}

pub fn get_llm_provider(config: &Config) -> Option<Arc<dyn LLMProvider + Send + Sync>> {
    if let Some(ext_config) = &config.llm_extractor {
        match ext_config.provider {
            ExtractorProvider::OpenAI => {
                let api_key = ext_config.api_key.clone().or_else(|| std::env::var("OPENAI_API_KEY").ok());
                if let Some(key) = api_key {
                    let mut provider = if let Some(base_url) = &ext_config.base_url {
                        OpenAIProvider::compatible(key, base_url)
                    } else {
                        OpenAIProvider::new(key)
                    };
                    provider = provider.with_model(&ext_config.model);
                    return Some(Arc::new(provider));
                }
            }
            ExtractorProvider::Anthropic => {
                let api_key = ext_config.api_key.clone().or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());
                if let Some(key) = api_key {
                    let mut provider = AnthropicProvider::new(key);
                    provider = provider.with_model(&ext_config.model);
                    return Some(Arc::new(provider));
                }
            }
            ExtractorProvider::Gemini => {
                let api_key = ext_config.api_key.clone().or_else(|| std::env::var("GEMINI_API_KEY").ok());
                if let Some(key) = api_key {
                    let mut provider = GeminiProvider::new(key);
                    provider = provider.with_model(&ext_config.model);
                    return Some(Arc::new(provider));
                }
            }
            ExtractorProvider::Ollama => {
                let host = ext_config.base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                let provider = OllamaProvider::builder()
                    .host(host)
                    .model(&ext_config.model)
                    .build();
                
                if let Ok(p) = provider {
                    return Some(Arc::new(p));
                }
            }
        }
    }

    // Fallback to environment variables if no config is present
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        return Some(Arc::new(OpenAIProvider::new(api_key)));
    }

    None
}
