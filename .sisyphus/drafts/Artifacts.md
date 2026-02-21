# Artifacts: Rust Local Memory Plugin (OpenSpec)

## 1. Project Structure
```text
.
├── Cargo.toml
├── src/
│   ├── main.rs          # MCP Server Entry Point
│   ├── engine/          # Search Funnel & Indexing
│   │   ├── mod.rs
│   │   ├── bit_wise.rs  # Stage 1: Hamming distance
│   │   ├── matryoshka.rs # Stage 2: Sliced Cosine
│   │   └── rerank.rs    # Stage 3: Full Float
│   ├── model/           # Embedding Logic (Candle)
│   │   ├── mod.rs
│   │   └── nomic.rs     # Nomic v1.5 implementation
│   ├── storage/         # Fjall KV Store Wrapper
│   │   ├── mod.rs
│   │   └── lsm.rs
│   └── mcp/             # JSON-RPC / MCP Handlers
│       ├── mod.rs
│       ├── handlers.rs
│       └── types.rs
├── tests/               # TDD Test Suite
│   ├── integration.rs
│   └── search_recall.rs
└── spec/                # OpenSpec Definitions
    ├── SDD.md
    └── Spec.md
```

## 2. Key Modules & Responsibilities

### `engine::bit_wise`
- **Goal**: Blazing fast initial filter.
- **Tools**: `bitvec` for storage, `simsimd` for XOR+Popcount.
- **Logic**: Performs linear scan over all binary vectors in memory.

### `engine::matryoshka`
- **Goal**: Refine candidates using reduced dimensionality.
- **Logic**: Slices 768d vectors to 256d, re-normalizes, and computes Cosine similarity.

### `model::nomic`
- **Goal**: High-quality local embeddings.
- **Implementation**: Port of `nomic-embed-text-v1.5` using `Candle`.

### `storage::lsm`
- **Goal**: ACID storage for metadata and raw vectors.
- **Backend**: `Fjall`.

## 3. TDD Milestone Markers
1. **M1**: MCP Server boilerplate (Standard tools register).
2. **M2**: `bit_wise` scan logic (Verified against synthetic data).
3. **M3**: `Candle` model integration (Embedding generation).
4. **M4**: `Fjall` persistence (Storage & recovery).
5. **M5**: Full Search Funnel integration (Multi-stage verification).
