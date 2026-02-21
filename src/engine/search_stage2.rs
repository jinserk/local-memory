use crate::engine::matryoshka::slice_vector;
use crate::storage::db::Database;
use anyhow::Result;
use simsimd::SpatialSimilarity;
use uuid::Uuid;

pub fn matryoshka_refinement(
    db: &Database,
    query_vector: &[f32],
    candidate_ids: &[Uuid],
    top_k: usize,
) -> Result<Vec<(Uuid, f32)>> {
    let target_dim = 256;
    let sliced_query = slice_vector(query_vector, target_dim).map_err(anyhow::Error::msg)?;

    let mut scores = Vec::with_capacity(candidate_ids.len());

    for &id in candidate_ids {
        if let Some(memory) = db.get_memory(id)? {
            let sliced_candidate = slice_vector(&memory.vector, target_dim).map_err(anyhow::Error::msg)?;

            let distance = SpatialSimilarity::cos(&sliced_query, &sliced_candidate).ok_or_else(|| {
                anyhow::anyhow!("Failed to calculate cosine distance for ID: {}", id)
            })?;

            let similarity = 1.0 - distance as f32;

            scores.push((id, similarity));
        }
    }

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if scores.len() > top_k {
        scores.truncate(top_k);
    }

    Ok(scores)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::{Database, Memory};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_matryoshka_refinement() -> Result<()> {
        let dir = tempdir()?;
        let db = Database::open(dir.path())?;

        let mut v1 = vec![0.0; 768];
        v1[0] = 1.0;
        let id1 = Uuid::new_v4();
        db.insert_memory(&Memory {
            id: id1,
            metadata: json!({}),
            vector: v1,
            bit_vector: vec![],
        })?;

        let mut v2 = vec![0.0; 768];
        v2[1] = 1.0;
        let id2 = Uuid::new_v4();
        db.insert_memory(&Memory {
            id: id2,
            metadata: json!({}),
            vector: v2,
            bit_vector: vec![],
        })?;

        let mut query = vec![0.0; 768];
        query[0] = 1.0;
        query[1] = 0.1;

        let candidates = vec![id1, id2];
        let results = matryoshka_refinement(&db, &query, &candidates, 2)?;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, id1);
        assert_eq!(results[1].0, id2);
        assert!(results[0].1 > results[1].1);

        Ok(())
    }
}
