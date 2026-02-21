# SDD: Rust Local Memory Plugin (OpenSpec)

## 1. Introduction
This document defines the technical design for a local memory plugin for OpenCode and Claude-code. The primary goal is to provide a high-performance, low-resource long-term memory system using Matryoshka Embeddings and a multi-stage bit-wise search funnel.

## 2. System Architecture
The system follows the **Model Context Protocol (MCP)**, allowing it to interface with AI clients via JSON-RPC over stdio.

### 2.1 Component Overview
- **MCP Server**: Handles the JSON-RPC interface, tool registration, and request routing.
- **Embedding Engine (Candle)**: Executes the `nomic-embed-text-v1.5` model locally to transform text into 768-dimensional float vectors.
- **Search Funnel**:
    - **Stage 1 (Coarse Filter)**: 1-bit Binary Quantization (BQ) of the full 768d vector. Uses Hamming distance via `Simsimd`. Filters top ~1000 candidates.
    - **Stage 2 (Refinement)**: 256-dimension Matryoshka slice of the float vector. Uses Cosine similarity. Filters top ~100 candidates.
    - **Stage 3 (Precision)**: Full 768-dimension re-ranking of the top 100 candidates.
- **Storage Engine (Fjall)**: LSM-tree based KV store.
    - `vectors`: Key = UUID, Value = Full f32 vector blob.
    - `metadata`: Key = UUID, Value = JSON blob (text, source, timestamp).
    - `bit_index`: In-memory packed bit-vectors for Stage 1 scanning.

## 3. Detailed Funnel Logic
### 3.1 Binary Quantization (Stage 1)
- **Quantization**: `bit = (value > 0.0) ? 1 : 0`.
- **Storage**: 768 bits = 96 bytes per vector.
- **Search**: XOR followed by Popcount (Hamming distance). SIMD optimized.

### 3.2 Matryoshka Slicing (Stage 2)
- Uses the first 256 components of the 768-dimensional vector.
- **Normalization**: Sliced vectors must be re-normalized to unit length for Cosine similarity.

## 4. Performance Targets
- **Ingestion**: < 50ms per document (excluding embedding time).
- **Search Latency**: < 10ms for 1M documents (Stage 1 scan).
- **Recall@10**: > 0.9 compared to brute-force float search.

## 5. Persistence Strategy
- `Fjall` ensures ACID compliance for metadata and vectors.
- The `bit_index` is lazily loaded from `Fjall` on startup and kept in memory for maximum throughput.
