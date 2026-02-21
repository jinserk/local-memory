use crate::engine::bq::encode_bq;
use crate::model::nomic::Embedder;
use crate::storage::db::{Database, Memory};
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

pub struct IngestionPipeline {
    model: Arc<dyn Embedder + Send + Sync>,
    db: Arc<Database>,
}

impl IngestionPipeline {
    pub fn new(model: Arc<dyn Embedder + Send + Sync>, db: Arc<Database>) -> Self {
        Self { model, db }
    }

    pub fn run(&self, text: &str, metadata: serde_json::Value) -> Result<Uuid> {
        let id = Uuid::new_v4();


        let vector = self.model.encode(text)?;


        let bit_vector = encode_bq(&vector);


        let memory = Memory {
            id,
            metadata,
            vector,
            bit_vector,
        };

        self.db.insert_memory(&memory)?;

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use tempfile::tempdir;
    use std::sync::Arc;
    use serde_json::json;

    struct MockEmbedder;
    impl crate::model::nomic::Embedder for MockEmbedder {
        fn encode(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![1.0, -1.0, 0.5, -0.5])
        }
    }

    #[test]
    fn test_ingestion_pipeline() -> Result<()> {
        let dir = tempdir()?;
        let db = Arc::new(Database::open(dir.path())?);
        let model = Arc::new(MockEmbedder);
        let pipeline = IngestionPipeline::new(model, db.clone());

        let text = "hello world";
        let metadata = json!({"source": "test"});
        let id = pipeline.run(text, metadata.clone())?;

        let retrieved = db.get_memory(id)?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.metadata, metadata);
        assert_eq!(retrieved.vector, vec![1.0, -1.0, 0.5, -0.5]);
        // 1.0 -> 1, -1.0 -> 0, 0.5 -> 1, -0.5 -> 0 => 1010... -> 0xA0
        assert_eq!(retrieved.bit_vector, vec![0xA0]);

        Ok(())
    }
}
