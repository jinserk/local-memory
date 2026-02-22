use crate::storage::db::Database;
use anyhow::Result;
use serde_json::Value;
use simsimd::SpatialSimilarity;
use uuid::Uuid;

pub fn full_rerank(
    db: &Database,
    query_vector: &[f32],
    candidate_ids: &[Uuid],
    top_k: usize,
) -> Result<Vec<(Uuid, f32, Value)>> {
    let mut results = Vec::with_capacity(candidate_ids.len());

    for &id in candidate_ids {
        if let Some(memory) = db.get_memory(id)? {
            let distance =
                SpatialSimilarity::cos(query_vector, &memory.vector).ok_or_else(|| {
                    anyhow::anyhow!("Failed to calculate cosine distance for ID: {}", id)
                })?;

            let similarity = 1.0 - distance as f32;

            results.push((id, similarity, memory.metadata));
        }
    }

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if results.len() > top_k {
        results.truncate(top_k);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::{Database, Memory};
    use crate::storage::MemoryTier;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_full_rerank() -> Result<()> {
        let dir = tempdir()?;
        let db = Database::open(dir.path())?;

        let v1 = vec![1.0, 0.0, 0.0];
        let id1 = Uuid::new_v4();
        let meta1 = json!({"text": "first"});
        db.insert_memory(&Memory {
            id: id1,
            metadata: meta1.clone(),
            vector: v1.clone(),
            bit_vector: vec![],
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        let v2 = vec![0.0, 1.0, 0.0];
        let id2 = Uuid::new_v4();
        let meta2 = json!({"text": "second"});
        db.insert_memory(&Memory {
            id: id2,
            metadata: meta2.clone(),
            vector: v2.clone(),
            bit_vector: vec![],
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        let query = vec![1.0, 0.1, 0.0];
        let candidates = vec![id1, id2];
        let results = full_rerank(&db, &query, &candidates, 2)?;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, id1);
        assert_eq!(results[0].2, meta1);
        assert_eq!(results[1].0, id2);
        assert_eq!(results[1].2, meta2);
        assert!(results[0].1 > results[1].1);

        Ok(())
    }
}
