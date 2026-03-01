use crate::config::Config;
use crate::storage::sqlite::SqliteDatabase;
use anyhow::Result;
use serde_json::Value;
use uuid::Uuid;

pub struct SearchFunnel<'a> {
    db: &'a SqliteDatabase,
    _config: &'a Config,
}

#[derive(Debug, Clone)]
pub struct FunnelResult {
    pub id: Uuid,
    pub score: f32,
    pub metadata: Value,
    pub context: Option<Value>, // Additional context from graph
}

impl<'a> SearchFunnel<'a> {
    pub fn new_sqlite(db: &'a SqliteDatabase, config: &'a Config) -> Self {
        Self { db, _config: config }
    }

    pub fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<FunnelResult>> {
        let sqlite_results = self.db.search_documents(query_vector, top_k)?;

        let final_results = sqlite_results
            .into_iter()
            .map(|(id, score, metadata)| FunnelResult {
                id,
                score,
                metadata,
                context: None,
            })
            .collect();

        Ok(final_results)
    }

    pub fn hybrid_search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<FunnelResult>> {
        // 1. Vector Search for documents
        let results = self.search(query_vector, top_k)?;

        // 2. Enhance with graph context (simple version)
        // In a real LightRAG implementation, we would extract keywords from query
        // and search for entities. For now, we'll just return the documents.
        
        Ok(results)
    }
}
