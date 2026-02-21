
## 2026-02-21: Project Initialization
- Initialized OpenSpec SDD, Spec, and Artifacts files in the `spec/` directory.
- Established the 3-stage funnel architecture:
  1. Binary Quantization (BQ) for rapid filtering (1-bit, 768 bits).
  2. Matryoshka Embeddings for refinement (256-dimensional float32).
  3. Full Embeddings for final reranking (768-dimensional float32).
- Defined the MCP v1.0 interface with tools for memory insertion, search, and deletion.
- Mapped the internal project structure to ensure clear separation of concerns between engine logic, data models, storage, and the MCP interface.
- Confirmed the use of `swiftide` for orchestration, `fjall` for storage, and `simsimd` for performance.

## 2026-02-21: Project Scaffolding
- Initialized Rust project with `cargo init`.
- Created directory structure: `src/engine`, `src/model`, `src/storage`, `src/mcp`, and `tests`.
- Configured `Cargo.toml` with key dependencies: `tokio`, `serde`, `fjall`, `candle-core`, `simsimd`, `bitvec`, `uuid`, and `anyhow`.
- Implemented a basic MCP server loop in `src/main.rs` using stdio JSON-RPC.
- Verified the project structure and dependencies with `cargo check`.

## 2026-02-21: JSON Configuration Module
- Implemented `Config` and `SearchStages` structs in `src/config.rs`.
- Used `serde` for JSON deserialization with sensible defaults.
- Added `load()` method supporting `LOCAL_MEMORY_CONFIG` environment variable and `config.json` fallback.
- Note: In Rust 2024 edition, `env::set_var` and `env::remove_var` are unsafe and require `unsafe` blocks in tests.
- Verified configuration loading and default fallback with unit tests.

## Fjall Storage Setup (2026-02-21)
- Fjall 3.0.2 has significant API changes compared to 2.x.
- `Keyspace` is now `Database`, and `Partition` is now `Keyspace`.
- `Database::builder(path).open()?` is the entry point.
- `db.keyspace(name, || KeyspaceCreateOptions::default())?` is used to open/create a keyspace.
- Write batches are available via `db.batch()` and are useful for atomic updates across multiple keyspaces.
- `uuid` crate needs the `serde` feature to be used with `serde_json` or `bincode`.
- `bincode` is efficient for serializing `Vec<f32>` for storage.

## Matryoshka Slicing Logic (2026-02-21)
- Matryoshka embeddings (like `nomic-embed-text-v1.5`) allow for truncation while preserving representational power.
- Re-normalization (L2) is CRITICAL after slicing to ensure the vector remains on the unit hypersphere for cosine similarity.
- Implementation uses `&[f32]` for flexibility and returns `Result<Vec<f32>, String>` to handle dimension mismatches.
- L2 norm calculation: `sqrt(sum(x_i^2))`.
- Re-normalization: `v_i / norm`.
- Handled edge case where norm is zero (all-zero vector) to avoid division by zero.

## Nomic Model Integration (Candle)
- Implemented `NomicModel` using `candle-transformers` BERT implementation.
- Nomic Embed Text v1.5 requires mean pooling and L2 normalization for optimal performance.
- Used `tokenizers` crate for text processing.
- Device selection (CPU/GPU) is handled via `candle_core::Device`.
- Weights are loaded using `mmaped_safetensors` for efficiency.
