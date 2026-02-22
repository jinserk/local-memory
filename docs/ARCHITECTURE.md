# Local Memory Architecture

This document describes the architecture of Local Memory, a high-performance semantic memory system with MCP integration.

## System Overview

```
+------------------+     +------------------+     +------------------+
|   MCP Client     |     |   CLI (mem-diag) |     |   External       |
|   (OpenCode,     |     |   Diagnostics    |     |   Applications   |
|   Claude-code)   |     |                  |     |                  |
+--------+---------+     +--------+---------+     +--------+---------+
         |                        |                        |
         | JSON-RPC 2.0           | Direct                 |
         | (stdio)                | Access                 |
         v                        v                        v
+--------+---------------------------------------------------------+
|                         MCP Server Layer                        |
|  +----------------------------------------------------------+   |
|  |                    MCP Tools (tools.rs)                   |   |
|  |    - memory_insert    - memory_search                     |   |
|  +----------------------------------------------------------+   |
+-----------------------------+-----------------------------------+
                              |
                              v
+-----------------------------+-----------------------------------+
|                        Engine Layer                             |
|  +-------------+  +------------------+  +------------------+   |
|  | Ingestion   |  |  Search Funnel   |  | Binary Quantize  |   |
|  | Pipeline    |  |  Coordinator     |  | (BQ Encoder)     |   |
|  +------+------+  +---------+--------+  +---------+--------+   |
|         |                  |                     |              |
|         v                  v                     v              |
|  +------+---------------------------------------------------+   |
|  |                    Matryoshka Slicer                     |   |
|  +----------------------------------------------------------+   |
+-----------------------------+-----------------------------------+
                              |
                              v
+-----------------------------+-----------------------------------+
|                        Model Layer                              |
|  +----------------------------------------------------------+   |
|  |                   Nomic Model                            |   |
|  |    - Text tokenization    - 768d embedding generation    |   |
|  |    - Mean pooling         - L2 normalization             |   |
|  +----------------------------------------------------------+   |
+-----------------------------+-----------------------------------+
                              |
                              v
+-----------------------------+-----------------------------------+
|                       Storage Layer                             |
|  +----------------------------------------------------------+   |
|  |              Fjall Database (LSM Tree)                   |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  |  | Metadata     |  | Vector      |  | Bit Vector      |  |   |
|  |  | Keyspace     |  | Keyspace    |  | Keyspace        |  |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  +----------------------------------------------------------+   |
+-----------------------------------------------------------------+
```

## Components

### 1. MCP Server Layer (`src/main.rs`, `src/mcp/`)

The entry point for all MCP communication. Implements JSON-RPC 2.0 over stdio.

**Responsibilities:**
- Parse incoming JSON-RPC requests
- Route method calls to appropriate handlers
- Format and send JSON-RPC responses
- Manage server lifecycle

**Supported Methods:**
- `initialize` - Server handshake
- `tools/list` - List available tools
- `tools/call` - Execute a tool

### 2. Engine Layer (`src/engine/`)

The core processing logic for memory operations.

#### Ingestion Pipeline (`ingestion.rs`)

Orchestrates the flow from text to stored memory:

```
Text Input
    |
    v
+-----------------+
| Tokenize Text   |
+-----------------+
    |
    v
+-----------------+
| Generate 768d   |
| Embedding       |
| (Nomic Model)   |
+-----------------+
    |
    v
+-----------------+
| Binary Quantize |
| (768 bits)      |
+-----------------+
    |
    v
+-----------------+
| Store to DB     |
| - Metadata      |
| - Full Vector   |
| - Bit Vector    |
+-----------------+
```

#### Search Funnel (`funnel.rs`)

The 3-stage search pipeline that balances speed and accuracy:

```
Query Text
    |
    v
+-----------------+
| Generate Query  |
| Embedding       |
+-----------------+
    |
    v
================================================================
|  STAGE 1: Hamming Scan (Binary Quantization)                 |
|  -----------------------------------------------------------  |
|  - Converts query to binary vector                           |
|  - Scans ALL memories using Hamming distance                 |
|  - SIMD-accelerated (~16x faster than scalar)                |
|  - Returns top stage1_k candidates (default: 100)            |
|  - Time: ~130 microseconds for 1000 vectors                  |
================================================================
    |
    v (100 candidates)
================================================================
|  STAGE 2: Matryoshka Refinement                              |
|  -----------------------------------------------------------  |
|  - Uses 256d truncated embeddings                            |
|  - Computes cosine similarity                                |
|  - Re-normalizes after truncation                            |
|  - Returns top stage2_k candidates (default: 10)             |
|  - Time: ~50 microseconds                                    |
================================================================
    |
    v (10 candidates)
================================================================
|  STAGE 3: Full Re-rank                                       |
|  -----------------------------------------------------------  |
|  - Uses full 768d embeddings                                 |
|  - Computes precise cosine similarity                        |
|  - Retrieves full metadata                                   |
|  - Returns top_k results (default: 5)                        |
|  - Time: ~100 microseconds                                   |
================================================================
    |
    v (5 results with metadata)
```

### 3. Model Layer (`src/model/`)

#### Nomic Embed Text v1.5 (`nomic.rs`)

Implements the embedding model using Candle transformers.

**Features:**
- 768-dimensional embeddings
- Mean pooling over token embeddings
- L2 normalization for cosine similarity
- Supports Matryoshka representation learning

**Model Files Required:**
- `config.json` - Model configuration
- `tokenizer.json` - BPE tokenizer
- `model.safetensors` - Model weights

### 4. Storage Layer (`src/storage/`)

#### Fjall Database (`db.rs`)

LSM-tree based persistent storage using Fjall 3.0.

**Keyspaces:**

| Keyspace | Key | Value | Description |
|----------|-----|-------|-------------|
| `metadata` | UUID | JSON | Memory metadata + tier info |
| `vectors` | UUID | bincode | Full 768d float vector |
| `bit_vectors` | UUID | bytes | Binary quantized vector |

**Memory Entry Structure:**

```rust
struct Memory {
    id: Uuid,                  // 16 bytes
    metadata: Value,           // Variable (JSON)
    vector: Vec<f32>,          // 3072 bytes (768 * 4)
    bit_vector: Vec<u8>,       // 96 bytes (768 / 8)
    tier: MemoryTier,          // 1 byte
    expires_at: Option<u64>,   // 8 bytes (if episodic)
}
```

**Memory Footprint:** ~3.2 KB per document

#### Memory Tiering (`tier.rs`)

Two memory types with different persistence characteristics:

| Tier | Persistence | Use Case |
|------|-------------|----------|
| **Semantic** | Permanent | User preferences, learned facts |
| **Episodic** | TTL-based | Temporary context, session data |

Expired episodic memories are filtered on read, not actively garbage collected.

## Data Flow

### Insert Flow

```
Client                    MCP Server                Engine                 Storage
  |                           |                        |                      |
  |---- memory_insert ------->|                        |                      |
  |                           |---- run pipeline ----->|                      |
  |                           |                        |---- encode text ---> Model
  |                           |                        |<--- 768d vector -----|
  |                           |                        |---- quantize ------->|
  |                           |                        |<--- 96 bytes --------|
  |                           |                        |---- insert --------->|
  |                           |                        |                      |-- store
  |                           |<--- UUID --------------|                      |
  |<--- "Memory inserted" ----|                        |                      |
```

### Search Flow

```
Client                    MCP Server                Engine                 Storage
  |                           |                        |                      |
  |---- memory_search ------->|                        |                      |
  |                           |---- encode query ----->|                      |
  |                           |                        |---- encode text ---> Model
  |                           |                        |<--- 768d vector -----|
  |                           |---- funnel.search() -->|                      |
  |                           |                        |                      |
  |                           |                        |== STAGE 1: Hamming ==|
  |                           |                        |---- scan bit_vec ---->|
  |                           |                        |<--- 100 candidates ---|
  |                           |                        |                      |
  |                           |                        |== STAGE 2: Matryoshka
  |                           |                        |---- get 256d vec ---->|
  |                           |                        |<--- cosine dists -----|
  |                           |                        |                      |
  |                           |                        |== STAGE 3: Full =====|
  |                           |                        |---- get 768d vec ---->|
  |                           |                        |---- get metadata ----->|
  |                           |                        |<--- results ----------|
  |                           |<--- results ----------|                      |
  |<--- JSON results ---------|                        |                      |
```

## Performance Characteristics

### SIMD Acceleration

Binary quantization enables SIMD-accelerated Hamming distance:

```
768-bit Hamming Distance:
- Scalar:  ~52 nanoseconds
- SIMD:    ~3 nanoseconds
- Speedup: ~16x
```

### Memory Layout

```
Per Document:
+------------------+------------------+------------------+
|  Full Vector     |  Bit Vector      |  Metadata        |
|  3072 bytes      |  96 bytes        |  ~64 bytes       |
|  (768 x f32)     |  (768 bits)      |  (JSON)          |
+------------------+------------------+------------------+
Total: ~3.2 KB
```

### Search Latency

| Database Size | Stage 1 | Full Funnel | 
|---------------|---------|-------------|
| 100 docs | ~13 us | ~170 us |
| 500 docs | ~64 us | ~217 us |
| 1000 docs | ~127 us | ~282 us |
| 10000 docs | ~1.3 ms | ~2 ms (est.) |

## Configuration

See `config.json` schema:

```json
{
  "storage_path": "./storage",
  "model_path": "./models",
  "search_stages": {
    "stage1_k": 100,
    "stage2_k": 10
  },
  "tier": {
    "default_tier": "Semantic",
    "default_episodic_ttl_seconds": 3600
  }
}
```

### Tuning Guidelines

- **Higher recall**: Increase `stage1_k` (e.g., 200-500)
- **Lower latency**: Decrease `stage1_k` and `stage2_k`
- **More results**: Increase `top_k` in search calls
- **Temporary data**: Set `default_tier` to `Episodic`
