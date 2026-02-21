# Software Design Document: Local Memory Engine

## Overview
This document describes the architecture for a local memory engine built in pure Rust. The system provides fast, reliable vector search without the complexity of HNSW or other graph-based indices. It uses a 3-stage funnel approach to balance speed and accuracy.

## Design Goals
- Pure Rust implementation.
- No HNSW or complex graph indices.
- High recall (Recall@10 > 0.9).
- Low memory footprint for local execution.

## 3-Stage Search Funnel
The engine processes queries through three increasingly precise stages to filter the candidate set.

### Stage 1: Binary Quantization (BQ)
- **Input**: 768-dimensional query embedding.
- **Process**: Convert query and stored embeddings to 1-bit representations (768 bits total).
- **Metric**: Hamming distance using bitwise XOR and POPCNT.
- **Goal**: Rapidly narrow down millions of vectors to a few thousand candidates.
- **Acceleration**: Uses `simsimd` for hardware-accelerated bitwise operations.

### Stage 2: Matryoshka Embeddings
- **Input**: Top-K candidates from Stage 1.
- **Process**: Use a 256-dimensional float32 subset of the original 768-dimensional embedding.
- **Metric**: Cosine similarity.
- **Goal**: Refine the candidate list to a smaller set (e.g., top 100) using reduced dimensionality.

### Stage 3: Full Embeddings
- **Input**: Top candidates from Stage 2.
- **Process**: Use the full 768-dimensional float32 embeddings.
- **Metric**: Cosine similarity.
- **Goal**: Final reranking to ensure maximum precision and meet the Recall@10 > 0.9 target.

## Storage Layer
- **Engine**: Fjall (LSM-tree based storage).
- **Data Layout**:
  - Metadata and text stored in primary keys.
  - Quantized vectors stored for fast Stage 1 access.
  - Full embeddings stored for Stage 2 and 3 reranking.

## Embedding Model
- **Model**: `nomic-embed-text-v1.5`.
- **Dimensions**: 768.
- **Integration**: Orchestrated via `swiftide`.
