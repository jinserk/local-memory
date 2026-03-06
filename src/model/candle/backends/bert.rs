use async_trait::async_trait;
use anyhow::Result;
use candle_core::{Device, DType, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use edgequake_llm::LlmError;
use serde_json::json;
use std::path::Path;
use tokenizers::Tokenizer;

use crate::model::candle::backend::{ModelBackend, ModelRole};

pub struct BertBackend {
    model: BertModel,
    dimension: usize,
}

impl BertBackend {
    pub fn load(model_dir: &Path, device: &Device) -> Result<Self> {
        let config_str = std::fs::read_to_string(model_dir.join("config.json"))?;
        let mut config_val: serde_json::Value = serde_json::from_str(&config_str)?;
        let map = config_val
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("Invalid config.json"))?;

        // Field aliases used by some BERT variants.
        if !map.contains_key("hidden_size") && map.contains_key("n_embd") {
            map.insert("hidden_size".to_string(), map["n_embd"].clone());
        }
        if !map.contains_key("num_attention_heads") && map.contains_key("n_head") {
            map.insert("num_attention_heads".to_string(), map["n_head"].clone());
        }
        if !map.contains_key("num_hidden_layers") && map.contains_key("n_layer") {
            map.insert("num_hidden_layers".to_string(), map["n_layer"].clone());
        }
        if !map.contains_key("intermediate_size") && map.contains_key("n_inner") {
            map.insert("intermediate_size".to_string(), map["n_inner"].clone());
        }
        map.entry("hidden_act".to_string()).or_insert(json!("gelu"));
        map.entry("type_vocab_size".to_string()).or_insert(json!(2));
        map.entry("layer_norm_eps".to_string()).or_insert(json!(1e-12));
        map.entry("pad_token_id".to_string()).or_insert(json!(0));
        map.entry("position_embedding_type".to_string())
            .or_insert(json!("absolute"));
        map.entry("hidden_dropout_prob".to_string())
            .or_insert(json!(0.1));
        map.entry("attention_probs_dropout_prob".to_string())
            .or_insert(json!(0.1));
        let max_pos = map
            .get("n_positions")
            .and_then(|v| v.as_u64())
            .unwrap_or(512) as usize;
        map.entry("max_position_embeddings".to_string())
            .or_insert(json!(max_pos));

        let config: BertConfig = serde_json::from_value(config_val)?;
        let hidden_size = config.hidden_size;
        let intermediate_size = config.intermediate_size;

        // Load and remap tensors from the NomicBERT safetensors layout.
        let raw_tensors =
            candle_core::safetensors::load(model_dir.join("model.safetensors"), device)?;
        let mut tensors = std::collections::HashMap::new();

        for (name, tensor) in raw_tensors {
            let mut mapped = name.clone();
            if mapped.starts_with("encoder.layers.") {
                mapped = mapped.replace("encoder.layers.", "encoder.layer.");
            }
            if mapped.contains(".attn.Wqkv.weight") {
                let prefix = mapped.replace(".attn.Wqkv.weight", "");
                tensors.insert(
                    format!("{}.attention.self.query.weight", prefix),
                    tensor.narrow(0, 0, hidden_size)?,
                );
                tensors.insert(
                    format!("{}.attention.self.key.weight", prefix),
                    tensor.narrow(0, hidden_size, hidden_size)?,
                );
                tensors.insert(
                    format!("{}.attention.self.value.weight", prefix),
                    tensor.narrow(0, 2 * hidden_size, hidden_size)?,
                );
                continue;
            }
            if mapped.contains(".attn.out_proj.weight") {
                mapped = mapped.replace(".attn.out_proj.weight", ".attention.output.dense.weight");
            } else if mapped.contains(".mlp.fc11.weight") {
                mapped = mapped.replace(".mlp.fc11.weight", ".intermediate.dense.weight");
            } else if mapped.contains(".mlp.fc2.weight") {
                mapped = mapped.replace(".mlp.fc2.weight", ".output.dense.weight");
            } else if mapped.contains(".norm1.weight") {
                mapped = mapped.replace(".norm1.weight", ".attention.output.LayerNorm.weight");
            } else if mapped.contains(".norm1.bias") {
                mapped = mapped.replace(".norm1.bias", ".attention.output.LayerNorm.bias");
            } else if mapped.contains(".norm2.weight") {
                mapped = mapped.replace(".norm2.weight", ".output.LayerNorm.weight");
            } else if mapped.contains(".norm2.bias") {
                mapped = mapped.replace(".norm2.bias", ".output.LayerNorm.bias");
            } else if mapped == "emb_ln.weight" {
                mapped = "embeddings.LayerNorm.weight".to_string();
            } else if mapped == "emb_ln.bias" {
                mapped = "embeddings.LayerNorm.bias".to_string();
            }
            tensors.insert(mapped, tensor);
        }

        // Synthesise missing positional embeddings (NomicBERT uses rotary, not learned).
        if !tensors.contains_key("embeddings.position_embeddings.weight") {
            let dummy = Tensor::zeros(
                (config.max_position_embeddings, hidden_size),
                DType::F32,
                device,
            )?;
            tensors.insert("embeddings.position_embeddings.weight".to_string(), dummy);
        }

        // Fill in zero biases for any attention/ffn biases not present in the checkpoint.
        for i in 0..config.num_hidden_layers {
            let prefix = format!("encoder.layer.{}", i);
            for suffix in ["query", "key", "value"] {
                let name = format!("{}.attention.self.{}.bias", prefix, suffix);
                if !tensors.contains_key(&name) {
                    tensors.insert(name, Tensor::zeros(hidden_size, DType::F32, device)?);
                }
            }
            let other_biases = [
                (
                    format!("{}.attention.output.dense.bias", prefix),
                    hidden_size,
                ),
                (
                    format!("{}.intermediate.dense.bias", prefix),
                    intermediate_size,
                ),
                (format!("{}.output.dense.bias", prefix), hidden_size),
            ];
            for (name, size) in other_biases {
                if !tensors.contains_key(&name) {
                    tensors.insert(name, Tensor::zeros(size, DType::F32, device)?);
                }
            }
        }

        let vb = VarBuilder::from_tensors(tensors, DType::F32, device);
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            dimension: hidden_size,
        })
    }
}

#[async_trait]
impl ModelBackend for BertBackend {
    fn role(&self) -> ModelRole {
        ModelRole::Embedding
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn generate(
        &self,
        _prompt: &str,
        _tokenizer: &Tokenizer,
        _device: &Device,
    ) -> Result<String, LlmError> {
        Err(LlmError::Unknown(
            "BertBackend does not support text generation".to_string(),
        ))
    }

    async fn embed_batch(
        &self,
        token_ids: &[Vec<u32>],
        type_ids: &[Vec<u32>],
        device: &Device,
    ) -> Result<Vec<Vec<f32>>, LlmError> {
        let mut results = Vec::with_capacity(token_ids.len());
        for (ids, tids) in token_ids.iter().zip(type_ids.iter()) {
            let input_ids = Tensor::new(ids.as_slice(), device)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .unsqueeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let token_type_ids = Tensor::new(tids.as_slice(), device)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .unsqueeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;

            let embeddings = self
                .model
                .forward(&input_ids, &token_type_ids, None)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;

            let (_n_batch, n_tokens, _hidden) = embeddings
                .dims3()
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let sum = embeddings
                .sum(1)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let mean = (sum / (n_tokens as f64))
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let norm = mean
                .sqr()
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .sum_keepdim(1)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .sqrt()
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let normalized = mean
                .broadcast_div(&norm)
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            let vec = normalized
                .squeeze(0)
                .map_err(|e| LlmError::Unknown(e.to_string()))?
                .to_vec1::<f32>()
                .map_err(|e| LlmError::Unknown(e.to_string()))?;
            results.push(vec);
        }
        Ok(results)
    }
}
