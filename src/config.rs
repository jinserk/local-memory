use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryTier {
    Episodic,
    Semantic,
}

impl Default for MemoryTier {
    fn default() -> Self {
        MemoryTier::Semantic
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TierConfig {
    pub default_tier: MemoryTier,
    pub default_episodic_ttl_seconds: Option<u64>,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            default_tier: MemoryTier::Semantic,
            default_episodic_ttl_seconds: Some(3600),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModelProvider {
    HuggingFace,
    Local,
}

impl Default for ModelProvider {
    fn default() -> Self {
        Self::HuggingFace
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ModelConfig {
    pub name: String,
    #[serde(default)]
    pub provider: ModelProvider,
    #[serde(default = "default_auto_download")]
    pub auto_download: bool,
}

fn default_auto_download() -> bool {
    true
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "nomic-ai/nomic-embed-text-v1.5".to_string(),
            provider: ModelProvider::HuggingFace,
            auto_download: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExtractorProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ExtractorConfig {
    pub provider: ExtractorProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default = "default_storage_path")]
    pub storage_path: PathBuf,
    #[serde(default = "default_model_path")]
    pub model_path: PathBuf,
    #[serde(default)]
    pub tier: TierConfig,
    
    #[serde(default, alias = "model")]
    pub embedding_model: ModelConfig,
    
    #[serde(default)]
    pub llm_extractor: Option<ExtractorConfig>,
}

fn default_storage_path() -> PathBuf { PathBuf::from(".local-memory/storage") }
fn default_model_path() -> PathBuf { PathBuf::from(".local-memory/models") }

impl Default for Config {
    fn default() -> Self {
        Self {
            storage_path: default_storage_path(),
            model_path: default_model_path(),
            tier: TierConfig::default(),
            embedding_model: ModelConfig::default(),
            llm_extractor: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = env::var("LOCAL_MEMORY_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".local-memory/config.json"));

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }

        Config::default()
    }
}
