use async_trait::async_trait;
use candle_core::Device;
use edgequake_llm::LlmError;
use tokenizers::Tokenizer;

/// Discriminates which role a loaded backend serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelRole {
    Embedding,
    Generation,
}

/// Unified trait implemented by every architecture-specific backend.
///
/// A backend owns the loaded model weights and device, but borrows the
/// tokenizer from `CandleProvider` (which holds it as a plain field, not
/// behind a lock, after construction is complete).
#[async_trait]
pub trait ModelBackend: Send + Sync {
    /// Which role this backend fulfils.
    fn role(&self) -> ModelRole;

    /// Embedding dimension. Only meaningful when `role() == Embedding`.
    fn dimension(&self) -> usize {
        0
    }

    /// Run text generation and return the decoded output string.
    /// Returns an error when `role() != Generation`.
    async fn generate(
        &self,
        prompt: &str,
        tokenizer: &Tokenizer,
        device: &Device,
    ) -> Result<String, LlmError>;

    /// Embed a batch of already-tokenised sequences.
    /// Returns an error when `role() != Embedding`.
    async fn embed_batch(
        &self,
        token_ids: &[Vec<u32>],
        type_ids: &[Vec<u32>],
        device: &Device,
    ) -> Result<Vec<Vec<f32>>, LlmError>;
}
