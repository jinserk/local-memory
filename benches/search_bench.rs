use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use local_memory::config::Config;
use local_memory::engine::bq::encode_bq;
use local_memory::engine::funnel::SearchFunnel;
use local_memory::engine::search_stage1::hamming_scan;
use local_memory::storage::db::{Database, Memory};
use local_memory::storage::MemoryTier;
use simsimd::BinarySimilarity;
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

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
// Benchmark 2: Hamming Scan (Stage 1 Search)
// =============================================================================

fn bench_hamming_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_scan");

    let vector_dim = 768;
    let bit_vector_len = vector_dim / 8;

    // Populate database with different sizes
    let db_sizes = [100, 500, 1000, 2000];

    for &num_vectors in &db_sizes {
        // Clear previous data by creating a new database
        let dir = tempdir().expect("Failed to create temp dir");
        let db = Database::open(dir.path()).expect("Failed to open database");

        for i in 0..num_vectors {
            let id = Uuid::new_v4();
            let bit_vector: Vec<u8> = (0..bit_vector_len).map(|j| ((i + j) % 256) as u8).collect();
            let vector: Vec<f32> = (0..vector_dim)
                .map(|j| ((i + j) as f32 % 2.0) - 1.0)
                .collect();

            db.insert_memory(&Memory {
                id,
                metadata: serde_json::json!({"index": i}),
                vector,
                bit_vector,
                tier: MemoryTier::default(),
                expires_at: None,
            })
            .expect("Failed to insert memory");
        }

        let query: Vec<u8> = (0..bit_vector_len).map(|j| (j % 256) as u8).collect();

        group.throughput(Throughput::Elements(num_vectors as u64));

        group.bench_with_input(
            BenchmarkId::new("scan_k10", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(hamming_scan(&db, black_box(&query), 10).expect("Scan failed"))
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("scan_k100", num_vectors),
            &num_vectors,
            |bencher, _| {
                bencher.iter(|| {
                    black_box(hamming_scan(&db, black_box(&query), 100).expect("Scan failed"))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Benchmark 3: Full Search Funnel
// =============================================================================

fn bench_search_funnel(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_funnel");

    let vector_dim = 768;

    let db_sizes = [100, 500, 1000];

    for &num_vectors in &db_sizes {
        let dir = tempdir().expect("Failed to create temp dir");
        let db = Arc::new(Database::open(dir.path()).expect("Failed to open database"));

        // Populate database
        for i in 0..num_vectors {
            let id = Uuid::new_v4();
            let mut vector = vec![0.0f32; vector_dim];
            // Create semi-random vectors
            for j in 0..vector_dim {
                vector[j] = ((i * j) as f32 % 2.0) - 1.0;
            }
            let bit_vector = encode_bq(&vector);

            db.insert_memory(&Memory {
                id,
                metadata: serde_json::json!({"index": i, "text": format!("Document {}", i)}),
                vector,
                bit_vector,
                tier: MemoryTier::default(),
                expires_at: None,
            })
            .expect("Failed to insert memory");
        }

        let config = Config::default();
        let funnel = SearchFunnel::new(&db, &config);

        // Create a query vector
        let mut query = vec![0.0f32; vector_dim];
        query[0] = 0.9;
        query[1] = 0.1;

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
// Benchmark 4: Document Ingestion (excluding embedding time)
// =============================================================================

fn bench_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion");

    let vector_dim = 768;

    let dir = tempdir().expect("Failed to create temp dir");
    let db = Arc::new(Database::open(dir.path()).expect("Failed to open database"));

    // Benchmark the ingestion pipeline components (excluding embedding)
    // This measures: BQ encoding + database write

    group.bench_function("bq_encode_768d", |bencher| {
        let vector: Vec<f32> = (0..vector_dim).map(|i| (i as f32 % 2.0) - 1.0).collect();

        bencher.iter(|| black_box(encode_bq(black_box(&vector))));
    });

    group.bench_function("db_insert_single", |bencher| {
        let mut counter = 0u64;

        bencher.iter(|| {
            let id = Uuid::new_v4();
            let vector: Vec<f32> = (0..vector_dim)
                .map(|i| ((i + counter as usize) as f32 % 2.0) - 1.0)
                .collect();
            let bit_vector = encode_bq(&vector);

            db.insert_memory(&Memory {
                id,
                metadata: serde_json::json!({"counter": counter}),
                vector,
                bit_vector,
                tier: MemoryTier::default(),
                expires_at: None,
            })
            .expect("Failed to insert");

            counter += 1;
            black_box(id)
        });
    });

    // Full ingestion pipeline (BQ + DB write)
    group.bench_function("full_ingestion_no_embedding", |bencher| {
        let mut counter = 0u64;

        bencher.iter(|| {
            let id = Uuid::new_v4();
            // Simulate pre-computed embedding
            let vector: Vec<f32> = (0..vector_dim)
                .map(|i| ((i + counter as usize) as f32 % 2.0) - 1.0)
                .collect();
            let bit_vector = encode_bq(&vector);

            let memory = Memory {
                id,
                metadata: serde_json::json!({"text": format!("Document {}", counter)}),
                vector,
                bit_vector,
                tier: MemoryTier::default(),
                expires_at: None,
            };

            db.insert_memory(&memory).expect("Failed to insert");
            counter += 1;
            black_box(id)
        });
    });

    group.finish();
}

// =============================================================================
// Benchmark 5: SIMD Verification (Assembly Check)
// =============================================================================

fn bench_simd_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_verification");

    // This benchmark is designed to verify SIMD is being used
    // by comparing performance characteristics

    let bytes = 96; // 768 bits = 96 bytes
    let a: Vec<u8> = (0..bytes).map(|i| (i % 256) as u8).collect();
    let b: Vec<u8> = (0..bytes).map(|i| ((i + 128) % 256) as u8).collect();

    // Run many iterations to get stable measurements
    group.sample_size(1000);

    group.bench_function("scalar_768bit", |bencher| {
        bencher.iter(|| {
            for _ in 0..100 {
                black_box(hamming_distance_scalar(black_box(&a), black_box(&b)));
            }
        });
    });

    group.bench_function("simd_768bit", |bencher| {
        bencher.iter(|| {
            for _ in 0..100 {
                black_box(hamming_distance_simd(black_box(&a), black_box(&b)));
            }
        });
    });

    group.finish();
}

// =============================================================================
// Benchmark 6: Memory Usage Estimation
// =============================================================================

fn bench_memory_overhead(c: &mut Criterion) {
    let group = c.benchmark_group("memory_overhead");

    // Measure memory footprint of stored data

    let vector_dim = 768;
    let bit_vector_len = vector_dim / 8;

    // Estimate per-document memory usage
    let vector_bytes = vector_dim * 4; // f32 = 4 bytes
    let bit_vector_bytes = bit_vector_len;
    let uuid_bytes = 16;
    let metadata_overhead = 64; // Estimated JSON overhead

    let total_per_doc = vector_bytes + bit_vector_bytes + uuid_bytes + metadata_overhead;

    println!("\n=== Memory Usage Estimation ===");
    println!("Vector (768d f32):     {} bytes", vector_bytes);
    println!("Bit vector (768 bits): {} bytes", bit_vector_bytes);
    println!("UUID:                  {} bytes", uuid_bytes);
    println!("Metadata overhead:     ~{} bytes", metadata_overhead);
    println!(
        "Total per document:    {} bytes ({:.2} KB)",
        total_per_doc,
        total_per_doc as f64 / 1024.0
    );
    println!(
        "1000 documents:        {} KB",
        (total_per_doc * 1000) / 1024
    );
    println!(
        "10000 documents:       {} MB",
        (total_per_doc * 10000) / 1024 / 1024
    );
    println!("================================\n");

    group.finish();
}

criterion_group!(
    benches,
    bench_hamming_distance,
    bench_hamming_scan,
    bench_search_funnel,
    bench_ingestion,
    bench_simd_verification,
    bench_memory_overhead,
);

criterion_main!(benches);
