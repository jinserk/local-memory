use crate::storage::db::Database;
use anyhow::Result;
use simsimd::BinarySimilarity;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: Uuid,
    pub distance: f64,
}

pub fn hamming_scan(db: &Database, query_bits: &[u8], k: usize) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    for kv_res in db.bit_index_iter() {
        let (key, value) = kv_res?;
        let id = Uuid::from_slice(&key)?;

        if let Some(dist) = u8::hamming(query_bits, &value) {
            results.push(SearchResult { id, distance: dist });
        }
    }

    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    results.truncate(k);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Memory;
    use crate::storage::MemoryTier;
    use tempfile::tempdir;

    #[test]
    fn test_hamming_scan_basic() -> Result<()> {
        let dir = tempdir()?;
        let db = Database::open(dir.path())?;

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        db.insert_memory(&Memory {
            id: id1,
            metadata: serde_json::json!({}),
            vector: vec![1.0, 1.0],
            bit_vector: vec![0b11110000],
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        db.insert_memory(&Memory {
            id: id2,
            metadata: serde_json::json!({}),
            vector: vec![-1.0, -1.0],
            bit_vector: vec![0b00001111],
            tier: MemoryTier::default(),
            expires_at: None,
        })?;

        let query = vec![0b11110000];
        let results = hamming_scan(&db, &query, 10)?;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, id1);
        assert_eq!(results[0].distance, 0.0);
        assert_eq!(results[1].id, id2);
        assert!(results[1].distance > 0.0);

        Ok(())
    }

    #[test]
    fn test_hamming_scan_benchmark() -> Result<()> {
        let dir = tempdir()?;
        let db = Database::open(dir.path())?;

        let vector_dim = 1024;
        let num_vectors = 1000;
        let bit_vector_len = vector_dim / 8;

        for i in 0..num_vectors {
            let id = Uuid::new_v4();
            let bit_vector: Vec<u8> = (0..bit_vector_len).map(|j| ((i + j) % 256) as u8).collect();
            db.insert_memory(&Memory {
                id,
                metadata: serde_json::json!({}),
                vector: vec![0.0; vector_dim],
                bit_vector,
                tier: MemoryTier::default(),
                expires_at: None,
            })?;
        }

        let query: Vec<u8> = (0..bit_vector_len).map(|j| (j % 256) as u8).collect();

        let start = std::time::Instant::now();
        let results = hamming_scan(&db, &query, 10)?;
        let duration = start.elapsed();

        println!("Scanned {} vectors in {:?}", num_vectors, duration);
        assert_eq!(results.len(), 10);

        Ok(())
    }
}
