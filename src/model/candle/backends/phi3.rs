use async_trait::async_trait;
use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::phi3::{Model as Phi3Model, Config as Phi3Config};
use candle_transformers::generation::LogitsProcessor;
use edgequake_llm::LlmError;
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

use crate::model::candle::backend::{ModelBackend, ModelRole};
use crate::model::candle::registry::GenerationConfig;

pub struct Phi3Backend {
    model: Mutex<Phi3Model>,
    gen_config: GenerationConfig,
}

impl Phi3Backend {
    pub fn load(model_dir: &Path, device: &Device, gen_config: GenerationConfig) -> Result<Self> {
        let config_str = std::fs::read_to_string(model_dir.join("config.json"))?;
        let config: Phi3Config = serde_json::from_str(&config_str)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[model_dir.join("model.safetensors")],
                candle_core::DType::F32,
                device,
            )?
        };
        let model = Phi3Model::new(&config, vb)?;
        Ok(Self {
            model: Mutex::new(model),
            gen_config,
        })
    }
}

#[async_trait]
impl ModelBackend for Phi3Backend {
    fn role(&self) -> ModelRole {
        ModelRole::Generation
    }

    async fn generate(
        &self,
        prompt: &str,
        tokenizer: &Tokenizer,
        device: &Device,
    ) -> Result<String, LlmError> {
        let vocab = tokenizer.get_vocab(true);
        let eos_token = self
            .gen_config
            .eos_tokens
            .iter()
            .find_map(|t| vocab.get(t.as_str()).cloned())
            .unwrap_or(self.gen_config.eos_fallback_id);

        let tokens = tokenizer
            .encode(prompt, true)
            .map_err(|e| LlmError::Unknown(e.to_string()))?;
        let mut token_ids = tokens.get_ids().to_vec();
        let mut generated = Vec::new();
        let mut logits_processor = LogitsProcessor::new(42, None, None);

        let mut model = self.model.lock().unwrap();
        for _ in 0..self.gen_config.max_new_tokens {
            let input = Tensor::new(token_ids.as_slice(), device)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .unsqueeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            // seqlen_offset for Phi3: offset into the sequence for KV-cache.
            let offset = token_ids.len() - generated.len();
            let logits = model
                .forward(&input, offset)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let logits = logits
                .squeeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let token = logits_processor
                .sample(&logits)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            if token == eos_token {
                break;
            }
            generated.push(token);
            token_ids.push(token);
        }

        tokenizer
            .decode(&generated, true)
            .map_err(|e| LlmError::Unknown(e.to_string()))
    }

    async fn embed_batch(
        &self,
        _token_ids: &[Vec<u32>],
        _type_ids: &[Vec<u32>],
        _device: &Device,
    ) -> Result<Vec<Vec<f32>>, LlmError> {
        Err(LlmError::Unknown(
            "Phi3Backend does not support embedding".to_string(),
        ))
    }
}
