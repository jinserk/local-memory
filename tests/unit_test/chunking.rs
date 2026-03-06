use local_memory::engine::ingestion::IngestionPipeline;
use local_memory::storage::SqliteDatabase;
use tempfile::tempdir;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_explicit_chunking() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("chunking.db");
    let dim = 768;
    let db = Arc::new(SqliteDatabase::open(&db_path, dim)?);
    
    struct MockEmbedder;
    #[async_trait::async_trait]
    impl edgequake_llm::EmbeddingProvider for MockEmbedder {
        fn name(&self) -> &str { "mock" }
        fn model(&self) -> &str { "mock" }
        fn dimension(&self) -> usize { 768 }
        fn max_tokens(&self) -> usize { 512 }
        async fn embed(&self, texts: &[String]) -> std::result::Result<Vec<Vec<f32>>, edgequake_llm::LlmError> {
            Ok(texts.iter().map(|_| vec![0.0; 768]).collect())
        }
    }

    let pipeline = IngestionPipeline::new(Arc::new(MockEmbedder), db.clone(), None, false, None);
    
    let text = "Chunk 1 ---CHUNK--- Chunk 2";
    let id = pipeline.run(text, json!({"title": "ChunkTest"})).await?;
    
    // Use the helper method
    let doc_count = db.count_documents_by_parent(&id.to_string())?;
    assert_eq!(doc_count, 2);

    Ok(())
}
