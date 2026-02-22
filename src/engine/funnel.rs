use crate::config::Config;
use crate::engine::bq::encode_bq;
use crate::engine::search_stage1::hamming_scan;
use crate::engine::search_stage2::matryoshka_refinement;
use crate::engine::search_stage3::full_rerank;
use crate::storage::db::Database;
use anyhow::Result;
use serde_json::Value;
use uuid::Uuid;

pub struct SearchFunnel<'a> {
    db: &'a Database,
    config: &'a Config,
}

#[derive(Debug, Clone)]
pub struct FunnelResult {
    pub id: Uuid,
    pub score: f32,
    pub metadata: Value,
}

impl<'a> SearchFunnel<'a> {
    pub fn new(db: &'a Database, config: &'a Config) -> Self {
        Self { db, config }
    }

    pub fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<FunnelResult>> {
        let query_bits = encode_bq(query_vector);
        let stage1_results =
            hamming_scan(self.db, &query_bits, self.config.search_stages.stage1_k)?;

        if stage1_results.is_empty() {
            return Ok(vec![]);
        }

        let stage1_ids: Vec<Uuid> = stage1_results.into_iter().map(|r| r.id).collect();

        let stage2_results = matryoshka_refinement(
            self.db,
            query_vector,
            &stage1_ids,
            self.config.search_stages.stage2_k,
        )?;

        if stage2_results.is_empty() {
            return Ok(vec![]);
        }

        let stage2_ids: Vec<Uuid> = stage2_results.into_iter().map(|r| r.0).collect();

        let stage3_results = full_rerank(self.db, query_vector, &stage2_ids, top_k)?;

        let final_results = stage3_results
            .into_iter()
            .map(|(id, score, metadata)| FunnelResult {
                id,
                score,
                metadata,
            })
            .collect();

        Ok(final_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Memory;
    use crate::storage::MemoryTier;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_search_funnel_integration() -> Result<()> {
        let dir = tempdir()?;
        let db = Database::open(dir.path())?;
        let config = Config::default();
        let funnel = SearchFunnel::new(&db, &config);

        let dim = 768;

        let mut v1 = vec![0.0; dim];
        v1[0] = 1.0;
        let id1 = Uuid::new_v4();
        db.insert_memory(&Memory {
            id: id1,
            metadata: json!({"text": "perfect match"}),
            vector: v1.clone(),
            bit_vector: encode_bq(&v1),
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        let mut v2 = vec![0.0; dim];
        v2[1] = 1.0;
        let id2 = Uuid::new_v4();
        db.insert_memory(&Memory {
            id: id2,
            metadata: json!({"text": "partial match"}),
            vector: v2.clone(),
            bit_vector: encode_bq(&v2),
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        let mut v3 = vec![0.0; dim];
        v3[dim - 1] = 1.0;
        let id3 = Uuid::new_v4();
        db.insert_memory(&Memory {
            id: id3,
            metadata: json!({"text": "no match"}),
            vector: v3.clone(),
            bit_vector: encode_bq(&v3),
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        let mut query = vec![0.0; dim];
        query[0] = 0.9;
        query[1] = 0.1;

        let results = funnel.search(&query, 2)?;

        assert!(results.len() >= 1);
        assert_eq!(results[0].id, id1);
        assert_eq!(results[0].metadata["text"], "perfect match");

        if results.len() > 1 {
            assert_eq!(results[1].id, id2);
        }

        Ok(())
    }
}
