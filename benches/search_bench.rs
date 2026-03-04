use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use local_memory::config::Config;
use local_memory::engine::vectors::{encode_bq, slice_vector};
use local_memory::engine::funnel::SearchFunnel;
use local_memory::storage::sqlite::SqliteDatabase;
use simsimd::{SpatialSimilarity, BinarySimilarity};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;
use std::collections::HashSet;
use candle_core::{Device, Tensor};

/// Scalar (non-SIMD) Hamming distance for comparison
fn hamming_distance_scalar(a: &[u8], b: &[u8]) -> u64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x ^ y).count_ones() as u64)
        .sum()
}

/// SIMD-accelerated Hamming distance using simsimd
fn hamming_distance_simd(a: &[u8], b: &[u8]) -> Option<f64> {
    u8::hamming(a, b)
}

// =============================================================================
// Benchmark 1: Hamming Distance SIMD vs Scalar
// =============================================================================

fn bench_hamming_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_distance");

    // Test with different vector sizes (in bits)
    let sizes = [128, 256, 512, 768, 1024, 2048];

    for &bits in &sizes {
        let bytes = bits / 8;
        let a: Vec<u8> = (0..bytes).map(|i| (i % 256) as u8).collect();
        let b: Vec<u8> = (0..bytes).map(|i| ((i + 128) % 256) as u8).collect();

        group.throughput(Throughput::Bytes(bytes as u64));

        group.bench_with_input(BenchmarkId::new("scalar", bits), &bits, |bencher, _| {
            bencher.iter(|| black_box(hamming_distance_scalar(black_box(&a), black_box(&b))));
        });

        group.bench_with_input(BenchmarkId::new("simd", bits), &bits, |bencher, _| {
            bencher.iter(|| black_box(hamming_distance_simd(black_box(&a), black_box(&b))));
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark 2: Full Search Funnel & Recall
// =============================================================================

fn bench_search_funnel(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_funnel");

    let vector_dim = 768;
    let db_sizes = [100, 500, 1000];
    let device = Device::Cpu;

    for &num_vectors in &db_sizes {
        let dir = tempdir().expect("Failed to create temp dir");
        let db_path = dir.path().join("bench.db");
        let db = Arc::new(SqliteDatabase::open(&db_path, vector_dim).expect("Failed to open database"));

        // Generate random vectors using candle
        let data = Tensor::randn(0.0f32, 1.0f32, (num_vectors, vector_dim), &device).expect("Failed to generate random vectors");
        let mut data_vec: Vec<Vec<f32>> = data.to_vec2().expect("Failed to convert tensor to vec");
        
        // Normalize each vector
        for v in &mut data_vec {
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in v { *x /= norm; }
            }
        }



        let mut all_ids = Vec::with_capacity(num_vectors);

        // Populate database
        for (i, vector) in data_vec.iter().enumerate() {
            let id = Uuid::new_v4();
            
            let v_short = slice_vector(vector, 256);
            let v_bit = encode_bq(vector);

            db.insert_document_with_namespace(
                id,
                &format!("Doc {}", i),
                &format!("Content for document {}", i),
                &serde_json::json!({"index": i, "text": format!("Content for document {}", i), "created_at": 1740000000}),
                vector,
                &v_short,
                &v_bit,
                "default"
            ).expect("Failed to insert");
            
            all_ids.push(id);
        }

        let mut config = Config::default();
        config.stage1_candidates = 1000;
        config.stage2_candidates = 1000;
        let funnel = SearchFunnel::new_sqlite(&db, &config);




        // Create a query vector (no perturbation for baseline check)
        let query = data_vec[0].clone();


        // Calculate Recall@10 against brute-force
        let top_k = 10;
        let mut oracle_scores: Vec<(Uuid, f32)> = Vec::with_capacity(num_vectors);
        for (i, v) in data_vec.iter().enumerate() {
            let distance = SpatialSimilarity::cos(&query, v).unwrap();
            let similarity = 1.0 - distance as f32;
            oracle_scores.push((all_ids[i], similarity));
        }
        oracle_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let oracle_top_k_list: Vec<Uuid> = oracle_scores.iter().take(top_k).map(|(id, _)| *id).collect();
        let oracle_top_k: HashSet<Uuid> = oracle_top_k_list.iter().cloned().collect();

        let funnel_results = funnel.search(&query, top_k).expect("Search failed");
        let funnel_top_k_ids: Vec<Uuid> = funnel_results.iter().take(top_k).map(|r| r.id).collect();
        let funnel_top_k: HashSet<Uuid> = funnel_top_k_ids.iter().cloned().collect();

        let intersection = oracle_top_k.intersection(&funnel_top_k).count();
        let recall = intersection as f32 / top_k as f32;
        
        println!("\n[Recall@10 for {} vectors]: {:.4}", num_vectors, recall);
        println!("  Oracle top 5: {:?}", &oracle_top_k_list[..5]);
        println!("  Funnel top 5: {:?}", &funnel_top_k_ids[..std::cmp::min(5, funnel_top_k_ids.len())]);

        if !funnel_results.is_empty() {
            println!("  Top result score: {:.4}", funnel_results[0].score);
        } else {
            println!("  No results found!");
        }

        group.throughput(Throughput::Elements(num_vectors as u64));

        group.bench_with_input(
            BenchmarkId::new("end_to_end", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(funnel.search(black_box(&query), 10).expect("Search failed"))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Benchmark 3: Document Ingestion (excluding embedding time)
// =============================================================================

fn bench_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion");

    let vector_dim = 768;
    let dir = tempdir().expect("Failed to create temp dir");
    let db_path = dir.path().join("ingest.db");
    let db = Arc::new(SqliteDatabase::open(&db_path, vector_dim).expect("Failed to open database"));

    group.bench_function("full_ingestion_no_embedding", |bencher| {
        let mut counter = 0u64;

        bencher.iter(|| {
            let id = Uuid::new_v4();
            // Simulate pre-computed embedding
            let vector = vec![0.5f32; vector_dim];
            let v_short = slice_vector(&vector, 256);
            let v_bit = encode_bq(&vector);

            db.insert_document_with_namespace(
                id,
                &format!("Doc {}", counter),
                &format!("Content {}", counter),
                &serde_json::json!({"counter": counter, "created_at": 1740000000}),
                &vector,
                &v_short,
                &v_bit,
                "default"
            ).expect("Failed to insert");

            counter += 1;
            black_box(id)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hamming_distance,
    bench_search_funnel,
    bench_ingestion,
);

criterion_main!(benches);
