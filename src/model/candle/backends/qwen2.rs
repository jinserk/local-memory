use async_trait::async_trait;
use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::qwen2::{ModelForCausalLM as Qwen2Model, Config as Qwen2Config};
use candle_transformers::generation::LogitsProcessor;
use edgequake_llm::LlmError;
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

use crate::model::candle::backend::{ModelBackend, ModelRole};
use crate::model::candle::registry::GenerationConfig;

pub struct Qwen2Backend {
    model: Mutex<Qwen2Model>,
    gen_config: GenerationConfig,
}

impl Qwen2Backend {
    pub fn load(model_dir: &Path, device: &Device, gen_config: GenerationConfig) -> Result<Self> {
        let config_str = std::fs::read_to_string(model_dir.join("config.json"))?;
        // Qwen2Config only deserializes the text-tower fields; serde ignores vision_config etc.
        let config: Qwen2Config = serde_json::from_str(&config_str)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[model_dir.join("model.safetensors")],
                candle_core::DType::F32,
                device,
            )?
        };
        let model = Qwen2Model::new(&config, vb)?;
        Ok(Self {
            model: Mutex::new(model),
            gen_config,
        })
    }
}

#[async_trait]
impl ModelBackend for Qwen2Backend {
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

        let encoding = tokenizer
            .encode(prompt, true)
            .map_err(|e| LlmError::Unknown(e.to_string()))?;
        let prompt_ids: Vec<u32> = encoding.get_ids().to_vec();
        let prompt_len = prompt_ids.len();
        let mut generated: Vec<u32> = Vec::new();
        let mut logits_processor = LogitsProcessor::new(42, None, None);

        let mut model = self.model.lock().unwrap();

        // Clear KV cache once before prefill (fix: previously called twice).
        model.clear_kv_cache();

        // Prefill: feed the entire prompt to populate KV cache.
        let prompt_tensor = Tensor::new(prompt_ids.as_slice(), device)
            .map_err(|e| LlmError::Unknown(e.to_string()))?
            .unsqueeze(0)
            .map_err(|e| LlmError::Unknown(e.to_string()))?;
        let logits = model
            .forward(&prompt_tensor, 0)
            .map_err(|e| LlmError::Unknown(e.to_string()))?;
        let logits = logits
            .squeeze(0)
            .map_err(|e| LlmError::Unknown(e.to_string()))?
            .squeeze(0)
            .map_err(|e| LlmError::Unknown(e.to_string()))?;
        let next_token = logits_processor
            .sample(&logits)
            .map_err(|e| LlmError::Unknown(e.to_string()))?;
        if next_token != eos_token {
            generated.push(next_token);
        }

        // Decode: one token at a time; KV cache accumulates.
        for _ in 0..(self.gen_config.max_new_tokens.saturating_sub(1)) {
            if generated.is_empty() {
                break;
            }
            let last_token = *generated.last().unwrap();
            if last_token == eos_token {
                generated.pop();
                break;
            }
            let input = Tensor::new(&[last_token], device)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .unsqueeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            // seqlen_offset = tokens already in the KV cache (prompt + generated so far − 1).
            let seqlen_offset = prompt_len + generated.len() - 1;
            let logits = model
                .forward(&input, seqlen_offset)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let logits = logits
                .squeeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .squeeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let token = logits_processor
                .sample(&logits)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            if token == eos_token {
                break;
            }
            generated.push(token);
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
            "Qwen2Backend does not support embedding".to_string(),
        ))
    }
}
