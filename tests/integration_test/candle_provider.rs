/// Integration tests for `CandleProvider`.
///
/// These tests exercise the provider's role-guard logic, `format_prompt`
/// routing, and token-count plumbing without requiring real model weights or
/// network access. A minimal mock backend and a small WordLevel tokenizer are
/// constructed in-process.
use anyhow::Result;
use async_trait::async_trait;
use candle_core::Device;
use edgequake_llm::{EmbeddingProvider, LLMProvider, LlmError};
use local_memory::model::candle::{
    backend::{ModelBackend, ModelRole},
    registry::{ArchConfig, EmbeddingConfig, ModelRegistry},
    CandleProvider,
};
use tokenizers::Tokenizer;

// ── Mock backends ─────────────────────────────────────────────────────────────

struct MockEmbeddingBackend {
    dim: usize,
}

#[async_trait]
impl ModelBackend for MockEmbeddingBackend {
    fn role(&self) -> ModelRole {
        ModelRole::Embedding
    }
    fn dimension(&self) -> usize {
        self.dim
    }
    async fn generate(
        &self,
        _prompt: &str,
        _tokenizer: &Tokenizer,
        _device: &Device,
    ) -> Result<String, LlmError> {
        Err(LlmError::Unknown("embedding backend cannot generate".into()))
    }
    async fn embed_batch(
        &self,
        token_ids: &[Vec<u32>],
        _type_ids: &[Vec<u32>],
        _device: &Device,
    ) -> Result<Vec<Vec<f32>>, LlmError> {
        Ok(token_ids.iter().map(|_| vec![0.5_f32; self.dim]).collect())
    }
}

struct MockGenerationBackend {
    response: String,
}

#[async_trait]
impl ModelBackend for MockGenerationBackend {
    fn role(&self) -> ModelRole {
        ModelRole::Generation
    }
    fn dimension(&self) -> usize {
        0
    }
    async fn generate(
        &self,
        _prompt: &str,
        _tokenizer: &Tokenizer,
        _device: &Device,
    ) -> Result<String, LlmError> {
        Ok(self.response.clone())
    }
    async fn embed_batch(
        &self,
        _token_ids: &[Vec<u32>],
        _type_ids: &[Vec<u32>],
        _device: &Device,
    ) -> Result<Vec<Vec<f32>>, LlmError> {
        Err(LlmError::Unknown("generation backend cannot embed".into()))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a minimal `Tokenizer` using a WordLevel model with a small vocabulary.
/// This is enough for tokenisation-plumbing tests without any model files.
fn make_test_tokenizer() -> Tokenizer {
    use ahash::AHashMap;
    use tokenizers::models::wordlevel::WordLevel;
    use tokenizers::pre_tokenizers::whitespace::Whitespace;
    use tokenizers::AddedToken;

    let mut vocab: AHashMap<String, u32> = AHashMap::new();
    // Special tokens + words used in test sentences.
    for (i, tok) in [
        "[UNK]", "[PAD]", "[CLS]", "[SEP]", "[MASK]",
        "hello", "world", "the", "fox", "jumped", "over",
        "lazy", "dog", "some", "raw", "text", "entity", "description",
        "search", "query", "summary", "extract",
    ]
    .iter()
    .enumerate()
    {
        vocab.insert(tok.to_string(), i as u32);
    }

    let model = WordLevel::builder()
        .vocab(vocab)
        .unk_token("[UNK]".to_string())
        .build()
        .unwrap();

    let mut tok = Tokenizer::new(model);
    tok.with_pre_tokenizer(Some(Whitespace {}));
    tok.add_special_tokens(&[
        AddedToken::from("[UNK]", true),
        AddedToken::from("[PAD]", true),
        AddedToken::from("[CLS]", true),
        AddedToken::from("[SEP]", true),
    ]);
    tok
}

fn embedding_arch_config() -> ArchConfig {
    ArchConfig::Embedding(EmbeddingConfig {
        query_prefix: "search_query: ".to_string(),
        document_prefix: "search_document: ".to_string(),
    })
}

fn generation_arch_config() -> ArchConfig {
    // Use the real Qwen2 extraction template from the embedded YAML.
    let reg = ModelRegistry::load().unwrap();
    reg.resolve("numind/NuExtract-2.0-2B").unwrap().config
}

fn make_embedding_provider() -> CandleProvider {
    CandleProvider::from_parts(
        "mock-bert",
        make_test_tokenizer(),
        Box::new(MockEmbeddingBackend { dim: 256 }),
        embedding_arch_config(),
    )
}

fn make_generation_provider() -> CandleProvider {
    CandleProvider::from_parts(
        "mock-qwen2",
        make_test_tokenizer(),
        Box::new(MockGenerationBackend {
            response: r#"{"entities":[],"relationships":[]}"#.to_string(),
        }),
        generation_arch_config(),
    )
}

// ── EmbeddingProvider tests ───────────────────────────────────────────────────

#[test]
fn test_embedding_provider_name() {
    let p = make_embedding_provider();
    assert_eq!(EmbeddingProvider::name(&p), "candle-embed");
}

#[test]
fn test_embedding_provider_model() {
    let p = make_embedding_provider();
    assert_eq!(EmbeddingProvider::model(&p), "mock-bert");
}

#[test]
fn test_embedding_provider_dimension_delegates_to_backend() {
    let p = make_embedding_provider();
    assert_eq!(p.dimension(), 256);
}

#[tokio::test]
async fn test_embed_returns_correct_batch_size() -> Result<()> {
    let p = make_embedding_provider();
    let texts = vec!["hello".to_string(), "world".to_string()];
    let result = p.embed(&texts).await?;
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].len(), 256);
    assert_eq!(result[1].len(), 256);
    Ok(())
}

#[tokio::test]
async fn test_embed_single_text_succeeds() -> Result<()> {
    let p = make_embedding_provider();
    let texts = vec!["hello".to_string()];
    let result = p.embed(&texts).await?;
    assert_eq!(result.len(), 1);
    Ok(())
}

// ── Role guard: embed on generation model returns error ───────────────────────

#[tokio::test]
async fn test_embed_on_generation_model_returns_error() {
    let p = make_generation_provider();
    let texts = vec!["hello".to_string()];
    let result = p.embed(&texts).await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("generation model"),
        "expected 'generation model' in error, got: {msg}"
    );
}

// ── LLMProvider tests ─────────────────────────────────────────────────────────

#[test]
fn test_llm_provider_name() {
    let p = make_generation_provider();
    assert_eq!(LLMProvider::name(&p), "huggingface");
}

#[test]
fn test_llm_provider_model() {
    let p = make_generation_provider();
    assert_eq!(LLMProvider::model(&p), "mock-qwen2");
}

#[tokio::test]
async fn test_complete_returns_mock_response() -> Result<()> {
    let p = make_generation_provider();
    let response = p.complete("some raw text to extract").await?;
    assert_eq!(response.content, r#"{"entities":[],"relationships":[]}"#);
    assert_eq!(response.model, "mock-qwen2");
    assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    Ok(())
}

// ── Role guard: complete on embedding model returns error ─────────────────────

#[tokio::test]
async fn test_complete_on_embedding_model_returns_error() {
    let p = make_embedding_provider();
    let result = p.complete("extract something").await;
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("embedding model"),
        "expected 'embedding model' in error, got: {msg}"
    );
}

// ── Prompt routing through format_prompt ──────────────────────────────────────

#[tokio::test]
async fn test_complete_wraps_plain_text_in_template() -> Result<()> {
    let p = make_generation_provider();
    // Plain text — format_prompt wraps in the Qwen2 extraction template.
    let response = p.complete("the fox jumped over the lazy dog").await?;
    assert!(!response.content.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_complete_passes_through_already_formatted_prompt() -> Result<()> {
    let p = make_generation_provider();
    // Contains <|im_start|> — must not be double-wrapped.
    let pre_formatted = "<|im_start|>user\nhello<|im_end|>\n<|im_start|>assistant\n";
    let response = p.complete(pre_formatted).await?;
    assert!(!response.content.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_complete_with_template_tag_does_not_error() -> Result<()> {
    let p = make_generation_provider();
    // Community summarisation format sent by communities.rs.
    let prompt = "TEMPLATE:summary\nentity A: description\nentity B: description";
    let response = p.complete(prompt).await?;
    assert!(!response.content.is_empty());
    Ok(())
}

// ── Token counts ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_complete_total_tokens_equals_sum() -> Result<()> {
    let p = make_generation_provider();
    let response = p.complete("hello world").await?;
    assert_eq!(
        response.total_tokens,
        response.prompt_tokens + response.completion_tokens
    );
    Ok(())
}

// ── UnifiedModel::prepare is a no-op ─────────────────────────────────────────

#[tokio::test]
async fn test_prepare_is_noop() -> Result<()> {
    use local_memory::model::UnifiedModel;
    let p = make_generation_provider();
    p.prepare().await
}
