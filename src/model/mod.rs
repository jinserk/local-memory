use std::sync::Arc;
use std::collections::HashMap;
use edgequake_llm::{LLMProvider, OpenAIProvider, OllamaProvider, EmbeddingProvider};
use crate::config::{Config, ExtractorProvider, ModelProvider};
use anyhow::Result;

pub mod base;
pub mod candle;
pub mod ollama;
pub mod auth;

// Re-export common types
pub use base::{UnifiedModel, GenericUnifiedModel, check_llm_connectivity, check_embedding_connectivity};
pub use candle::CandleProvider;
pub use ollama::pull_ollama_model;

use async_trait::async_trait;
use edgequake_llm::LlmError;

struct GeminiEmbeddingProvider {
    api_key: String,
    model: String,
    dimension: usize,
}

#[async_trait]
impl EmbeddingProvider for GeminiEmbeddingProvider {
    fn name(&self) -> &str { "gemini" }
    fn model(&self) -> &str { &self.model }
    fn dimension(&self) -> usize { self.dimension }
    fn max_tokens(&self) -> usize { 2048 }
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, LlmError> {
        let mut results = Vec::new();
        let client = reqwest::Client::new();
        
        for text in texts {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
                self.model, self.api_key
            );
            
            let resp = client.post(&url)
                .json(&serde_json::json!({
                    "content": { "parts": [{ "text": text }] }
                }))
                .send()
                .await
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
                
            if !resp.status().is_success() {
                let err_body = resp.text().await.unwrap_or_else(|e| e.to_string());
                return Err(LlmError::Unknown(format!("Gemini API error: {}", err_body)));
            }
            
            let body: serde_json::Value = resp.json().await
                .map_err(|e| LlmError::Unknown(format!("Failed to parse Gemini response: {}", e)))?;
                
            let values = body["embedding"]["values"].as_array()
                .ok_or_else(|| LlmError::Unknown("Missing 'embedding.values' in Gemini response".to_string()))?;
                
            let vec: Vec<f32> = values.iter()
                .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            results.push(vec);
        }
        
        Ok(results)
    }
}

use edgequake_llm::{ChatMessage, CompletionOptions, LLMResponse};

struct GeminiLLMProvider {
    api_key: String,
    model: String,
}

#[async_trait]
impl LLMProvider for GeminiLLMProvider {
    fn name(&self) -> &str { "gemini" }
    fn model(&self) -> &str { &self.model }
    fn max_context_length(&self) -> usize { 1048576 } // 1M tokens for Gemini 1.5/2.0
    async fn complete(&self, prompt: &str) -> Result<LLMResponse, LlmError> {
        self.complete_with_options(prompt, &CompletionOptions::default()).await
    }
    async fn complete_with_options(&self, prompt: &str, _options: &CompletionOptions) -> Result<LLMResponse, LlmError> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let resp = client.post(&url)
            .json(&serde_json::json!({
                "contents": [{ "parts": [{ "text": prompt }] }]
            }))
            .send()
            .await
            .map_err(|e| LlmError::Unknown(e.to_string()))?;

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_else(|e| e.to_string());
            return Err(LlmError::Unknown(format!("Gemini API error: {}", err_body)));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| LlmError::Unknown(format!("Failed to parse Gemini response: {}", e)))?;

        let text = body["candidates"][0]["content"]["parts"][0]["text"].as_str()
            .ok_or_else(|| LlmError::Unknown("Missing 'text' in Gemini response".to_string()))?
            .to_string();

        let prompt_tokens = body["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = body["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as usize;
        let total_tokens = body["usageMetadata"]["totalTokenCount"].as_u64().unwrap_or(0) as usize;

        let mut metadata = HashMap::new();
        metadata.insert("raw_response".to_string(), body);

        Ok(LLMResponse {
            content: text,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            model: self.model.clone(),
            finish_reason: Some("stop".to_string()),
            tool_calls: Vec::new(),
            metadata,
            cache_hit_tokens: None,
            thinking_tokens: None,
            thinking_content: None,
        })
    }
    async fn chat(&self, messages: &[ChatMessage], options: Option<&CompletionOptions>) -> Result<LLMResponse, LlmError> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let mut contents = Vec::new();
        for msg in messages {
            use edgequake_llm::ChatRole;
            let role = match msg.role {
                ChatRole::User => "user",
                ChatRole::Assistant => "model",
                ChatRole::System => "user", 
                _ => "user",
            };

            let mut parts = Vec::new();
            if !msg.content.is_empty() {
                parts.push(serde_json::json!({ "text": msg.content }));
            }

            if let Some(images) = &msg.images {
                for img in images {
                    parts.push(serde_json::json!({
                        "inline_data": {
                            "mime_type": img.mime_type,
                            "data": img.data
                        }
                    }));
                }
            }

            if !parts.is_empty() {
                contents.push(serde_json::json!({
                    "role": role,
                    "parts": parts
                }));
            }
        }

        let mut body_json = serde_json::json!({
            "contents": contents,
        });

        if let Some(opts) = options {
            let mut config = serde_json::json!({});
            if let Some(t) = opts.temperature { config["temperature"] = serde_json::json!(t); }
            if let Some(m) = opts.max_tokens { config["maxOutputTokens"] = serde_json::json!(m); }
            body_json["generationConfig"] = config;
        }

        eprintln!("DEBUG: [Gemini chat] Requesting: {}", url);

        let resp = client.post(&url)
            .json(&body_json)
            .send()
            .await
            .map_err(|e| LlmError::Unknown(e.to_string()))?;

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_else(|e| e.to_string());
            return Err(LlmError::Unknown(format!("Gemini API error: {}", err_body)));
        }

        let body: serde_json::Value = resp.json().await
            .map_err(|e| LlmError::Unknown(format!("Failed to parse Gemini response: {}", e)))?;

        let text = body["candidates"][0]["content"]["parts"][0]["text"].as_str()
            .ok_or_else(|| LlmError::Unknown("Missing 'text' in Gemini response".to_string()))?
            .to_string();

        let prompt_tokens = body["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as usize;
        let completion_tokens = body["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as usize;
        let total_tokens = body["usageMetadata"]["totalTokenCount"].as_u64().unwrap_or(0) as usize;

        let mut metadata = HashMap::new();
        metadata.insert("raw_response".to_string(), body);

        Ok(LLMResponse {
            content: text,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            model: self.model.clone(),
            finish_reason: Some("stop".to_string()),
            tool_calls: Vec::new(),
            metadata,
            cache_hit_tokens: None,
            thinking_tokens: None,
            thinking_content: None,
        })
    }
}

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
                .or_else(|| auth::get_opencode_key("opencode"))
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
        ModelProvider::Gemini => {
            let api_key = config
                .embedding
                .api_key
                .clone()
                .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                .or_else(|| auth::get_google_token())
                .ok_or_else(|| anyhow::anyhow!("Missing Google API key for Gemini embedding. (Checked config api_key, GOOGLE_API_KEY, and OpenCode auth.json)"))?;
            
            if api_key.starts_with("AIza") {
                Arc::new(GeminiEmbeddingProvider {
                    api_key,
                    model: config.embedding.name.clone(),
                    dimension: config.embedding.dimension,
                })
            } else {
                let base_url = config
                    .embedding
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta/openai".to_string());
                Arc::new(
                    OpenAIProvider::compatible(api_key, base_url)
                        .with_embedding_model(&config.embedding.name),
                )
            }
        }
    };

    // 2. Resolve LLM Extractor
    let llm: Arc<dyn LLMProvider> = if let Some(ext_config) = &config.llm_extractor {
        eprintln!("DEBUG: Resolving LLM: {:?} ({})", ext_config.provider, ext_config.name);
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
                    .or_else(|| auth::get_opencode_key("opencode"))
                    .ok_or_else(|| anyhow::anyhow!("Missing OpenAI API key"))?;
                let provider = OpenAIProvider::new(api_key.clone()).with_model(&ext_config.name);
                let provider = if let Some(base_url) = &ext_config.base_url {
                    OpenAIProvider::compatible(api_key, base_url.clone()).with_model(&ext_config.name)
                } else {
                    provider
                };
                Arc::new(provider)
            }
            ExtractorProvider::Gemini => {
                let api_key = ext_config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                    .or_else(|| auth::get_google_token())
                    .ok_or_else(|| anyhow::anyhow!("Missing Google API key for Gemini LLM. (Checked config api_key, GOOGLE_API_KEY, and OpenCode auth.json)"))?;
                
                if api_key.starts_with("AIza") {
                    Arc::new(GeminiLLMProvider {
                        api_key,
                        model: ext_config.name.clone(),
                    })
                } else {
                    let base_url = ext_config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta/openai".to_string());
                    Arc::new(
                        OpenAIProvider::compatible(api_key, base_url).with_model(&ext_config.name),
                    )
                }
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
        override_dimension: if config.embedding.provider == ModelProvider::OpenAI || config.embedding.provider == ModelProvider::Gemini {
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
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .or_else(|| auth::get_opencode_key("opencode"));

                let key = match key {
                    Some(k) => k,
                    None => return None,
                };

                let mut p = OpenAIProvider::new(key);
                p = p.with_model(&ext_config.name);
                return Some(Arc::new(p));
            }
            ExtractorProvider::Gemini => {
                let key = ext_config
                    .api_key
                    .clone()
                    .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
                    .or_else(|| auth::get_google_token());
                
                let key = match key {
                    Some(k) => k,
                    None => return None,
                };

                if key.starts_with("AIza") {
                    return Some(Arc::new(GeminiLLMProvider {
                        api_key: key,
                        model: ext_config.name.clone(),
                    }));
                }

                let base_url = ext_config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta/openai".to_string());
                let p = OpenAIProvider::compatible(key, base_url).with_model(&ext_config.name);
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
