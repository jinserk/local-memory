use anyhow::Result;
use candle_core::{Device, Tensor};
use local_memory::config::Config;
use local_memory::engine::funnel::SearchFunnel;
use local_memory::engine::vectors::{encode_bq, slice_vector};
use local_memory::storage::sqlite::SqliteDatabase;
use serde_json::json;
use simsimd::SpatialSimilarity;
use std::collections::HashSet;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn test_recall_bench() -> Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("recall.db");
    let dim = 768;
    let db = SqliteDatabase::open(&db_path, dim)?;

    let config = Config::default();
    let funnel = SearchFunnel::new_sqlite(&db, &config);

    let num_vectors = 50;
    let top_k = 5;

    let device = Device::Cpu;
    let data = Tensor::randn(0.0f32, 1.0f32, (num_vectors, dim), &device)?;
    let data_vec: Vec<Vec<f32>> = data.to_vec2()?;

    let mut ids = Vec::with_capacity(num_vectors);

    for (i, v) in data_vec.iter().enumerate() {
        let id = Uuid::new_v4();
        ids.push(id);
        
        let v_short = slice_vector(v, 256);
        let v_bit = encode_bq(v);
        
        db.insert_document(id, &format!("Doc {}", i), "content", &json!({"index": i, "text": format!("content {}", i)}), v, &v_short, &v_bit)?;
    }

    let query = (&data.get(0)? + &Tensor::randn(0.0f32, 0.01f32, (dim,), &device)?)?;
    let query_vec: Vec<f32> = query.to_vec1()?;

    let mut oracle_scores: Vec<(Uuid, f32)> = Vec::with_capacity(num_vectors);
    for (i, v) in data_vec.iter().enumerate() {
        let distance = SpatialSimilarity::cos(&query_vec, v).unwrap();
        let similarity = 1.0 - distance as f32;
        oracle_scores.push((ids[i], similarity));
    }

    oracle_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let oracle_top_k: HashSet<Uuid> = oracle_scores.iter().take(top_k).map(|(id, _)| *id).collect();

    let funnel_results = funnel.search(&query_vec, top_k)?;
    let funnel_top_k: HashSet<Uuid> = funnel_results.iter().take(top_k).map(|r| r.id).collect();

    let intersection = oracle_top_k.intersection(&funnel_top_k).count();
    let recall = intersection as f32 / top_k as f32;

    assert!(recall >= 0.2, "Recall@{} is {}, expected >= 0.2", top_k, recall);

    Ok(())
}
