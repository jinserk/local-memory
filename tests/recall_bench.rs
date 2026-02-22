use anyhow::Result;
use candle_core::{Device, Tensor};
use local_memory::config::{Config, SearchStages};
use local_memory::engine::bq::encode_bq;
use local_memory::engine::funnel::SearchFunnel;
use local_memory::storage::db::{Database, Memory};
use local_memory::storage::MemoryTier;
use serde_json::json;
use simsimd::SpatialSimilarity;
use std::collections::HashSet;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn test_recall_bench() -> Result<()> {
    let dir = tempdir()?;
    let db = Database::open(dir.path())?;

    let config = Config {
        search_stages: SearchStages {
            stage1_k: 1000,
            stage2_k: 1000,
            ..SearchStages::default()
        },
        ..Config::default()
    };

    let funnel = SearchFunnel::new(&db, &config);

    let num_vectors = 1000;
    let dim = 768;
    let top_k = 10;

    let device = Device::Cpu;
    let data = Tensor::randn(0.0f32, 1.0f32, (num_vectors, dim), &device)?;
    let data_vec: Vec<Vec<f32>> = data.to_vec2()?;

    let mut ids = Vec::with_capacity(num_vectors);

    println!("Inserting {} vectors...", num_vectors);
    for (i, v) in data_vec.iter().enumerate() {
        let id = Uuid::new_v4();
        ids.push(id);
        db.insert_memory(&Memory {
            id,
            metadata: json!({"index": i}),
            vector: v.clone(),
            bit_vector: encode_bq(v),
            tier: MemoryTier::default(),
            expires_at: None,
        })?;
    }

    let query = (&data.get(0)? + &Tensor::randn(0.0f32, 0.01f32, (dim,), &device)?)?;
    let query_vec: Vec<f32> = query.to_vec1()?;

    println!("Running Oracle search...");
    let mut oracle_scores: Vec<(Uuid, f32)> = Vec::with_capacity(num_vectors);
    for (i, v) in data_vec.iter().enumerate() {
        let distance = SpatialSimilarity::cos(&query_vec, v).unwrap();
        let similarity = 1.0 - distance as f32;
        oracle_scores.push((ids[i], similarity));
    }

    oracle_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let oracle_top_k: HashSet<Uuid> = oracle_scores
        .iter()
        .take(top_k)
        .map(|(id, _)| *id)
        .collect();

    println!("Running Funnel search...");
    let funnel_results = funnel.search(&query_vec, top_k)?;
    let funnel_top_k: HashSet<Uuid> = funnel_results.iter().take(top_k).map(|r| r.id).collect();

    let intersection = oracle_top_k.intersection(&funnel_top_k).count();
    let recall = intersection as f32 / top_k as f32;

    println!("Recall@{}: {}", top_k, recall);
    println!("Oracle top-10 IDs: {:?}", oracle_top_k);
    println!("Funnel top-10 IDs: {:?}", funnel_top_k);

    assert!(
        recall > 0.9,
        "Recall@{} is {}, which is not > 0.9",
        top_k,
        recall
    );

    Ok(())
}
