use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use std::path::Path;
use tokenizers::Tokenizer;
pub trait Embedder {
    fn encode(&self, text: &str) -> Result<Vec<f32>>;
}

impl Embedder for NomicModel {
    fn encode(&self, text: &str) -> Result<Vec<f32>> {
        self.encode(text)
    }
}

pub struct NomicModel {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl NomicModel {
    pub fn load(
        config_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        weights_path: impl AsRef<Path>,
        device: &Device,
    ) -> Result<Self> {
        let config_str = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, device)?
        };
        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device: device.clone(),
        })
    }

    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        let tokens = self
            .tokenizer
            .encode(text, true)
            .map_err(anyhow::Error::msg)?;
        let token_ids = tokens.get_ids();
        let input_ids = Tensor::new(token_ids, &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

        let embeddings = self.model.forward(&input_ids, &token_type_ids, None)?;

        let (_n_batch, n_tokens, _hidden_size) = embeddings.dims3()?;
        let sum_embeddings = embeddings.sum(1)?;
        let mean_embeddings = (sum_embeddings / (n_tokens as f64))?;

        let norm = mean_embeddings.sqr()?.sum_keepdim(1)?.sqrt()?;
        let normalized_embeddings = mean_embeddings.broadcast_div(&norm)?;

        let result = normalized_embeddings.squeeze(0)?.to_vec1::<f32>()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nomic_model_struct_exists() {}

    #[test]
    fn test_pooling_and_normalization() -> Result<()> {
        let device = Device::Cpu;
        let embeddings = Tensor::randn(0f32, 1f32, (1, 10, 768), &device)?;

        let (_n_batch, n_tokens, _hidden_size) = embeddings.dims3()?;
        let sum_embeddings = embeddings.sum(1)?;
        let mean_embeddings = (sum_embeddings / (n_tokens as f64))?;

        let norm = mean_embeddings.sqr()?.sum_keepdim(1)?.sqrt()?;
        let normalized_embeddings = mean_embeddings.broadcast_div(&norm)?;

        let result = normalized_embeddings.squeeze(0)?.to_vec1::<f32>()?;
        assert_eq!(result.len(), 768);

        let mut sum_sq = 0.0;
        for x in result {
            sum_sq += x * x;
        }
        assert!((sum_sq - 1.0).abs() < 1e-5);

        Ok(())
    }
}
