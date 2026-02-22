# Local Memory

A high-performance local semantic memory system with MCP (Model Context Protocol) integration. Store, search, and retrieve memories using vector embeddings, all running locally on your machine.

## Features

- **Semantic Search**: Find relevant memories using natural language queries
- **Multi-stage Search Funnel**: Optimized 3-stage search pipeline combining speed and accuracy
  - Stage 1: Binary quantization with SIMD-accelerated Hamming distance (~16x faster than scalar)
  - Stage 2: Matryoshka embedding refinement (256d vectors)
  - Stage 3: Full vector re-ranking (768d vectors)
- **MCP Protocol Support**: Works with OpenCode, Claude-code, and other MCP-compatible clients
- **Memory Tiering**: Episodic (temporary) and Semantic (permanent) memory storage
- **CLI Diagnostics**: Built-in tools for inspecting and testing your memory database
- **Local-first**: Everything runs on your machine, no cloud dependencies

## Installation

### Prerequisites

- Rust 1.75+ (2024 edition)
- Nomic Embed Text v1.5 model files

### Build from Source

```bash
git clone <repository-url>
cd local-memory
cargo build --release
```

### Download Model

The system requires Nomic Embed Text v1.5 model files. Download them to the `models/` directory:

```bash
mkdir -p models
# Download config.json, tokenizer.json, and model.safetensors to models/
```

## Quick Start

### Running the MCP Server

```bash
# Using default configuration
cargo run --release

# With custom config file
LOCAL_MEMORY_CONFIG=/path/to/config.json cargo run --release
```

The MCP server communicates via stdio using JSON-RPC 2.0. It's designed to be spawned by MCP clients.

### CLI Diagnostics

The `mem-diag` binary provides diagnostic tools:

```bash
# Show memory statistics
cargo run --release --bin mem-diag -- stats

# Search memories
cargo run --release --bin mem-diag -- search "your query here"

# Inspect a specific memory
cargo run --release --bin mem-diag -- inspect <uuid>

# Run diagnostic tests
cargo run --release --bin mem-diag -- test
```

## Usage

### MCP Integration

Local Memory implements the MCP v1.0 specification with two primary tools:

#### memory_insert

Insert a new memory into the database:

```json
{
  "name": "memory_insert",
  "arguments": {
    "text": "The user prefers dark mode in their editor",
    "metadata": {
      "category": "preference",
      "source": "conversation"
    }
  }
}
```

#### memory_search

Search for relevant memories:

```json
{
  "name": "memory_search",
  "arguments": {
    "query": "editor preferences",
    "top_k": 5
  }
}
```

### Configuration

Create a `config.json` file or set `LOCAL_MEMORY_CONFIG` environment variable:

```json
{
  "storage_path": "storage",
  "model_path": "models",
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

#### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `storage_path` | Directory for storing memories | `storage` |
| `model_path` | Directory containing model files | `models` |
| `search_stages.stage1_k` | Candidates from Hamming scan | `100` |
| `search_stages.stage2_k` | Candidates from Matryoshka refinement | `10` |
| `tier.default_tier` | Default memory tier (`Semantic` or `Episodic`) | `Semantic` |
| `tier.default_episodic_ttl_seconds` | TTL for episodic memories in seconds | `3600` |

### Memory Tiers

- **Semantic**: Permanent memories that persist until explicitly deleted
- **Episodic**: Temporary memories that expire after a configured TTL

Expired episodic memories are automatically filtered out during retrieval.

## Performance

Benchmarks on a typical development machine:

| Operation | Time |
|-----------|------|
| Ingestion (per document) | ~14 microseconds |
| Search (1000 vectors) | ~282 microseconds |
| SIMD Hamming (768-bit) | ~3.2 nanoseconds |
| Memory per document | ~3.2 KB |

The 3-stage search funnel provides excellent balance between speed and recall:
- Stage 1 scans all memories in microseconds using binary vectors
- Stage 2 refines with higher-precision Matryoshka embeddings
- Stage 3 provides final ranking with full cosine similarity

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed system architecture.

## MCP Integration Guide

See [docs/MCP_INTEGRATION.md](docs/MCP_INTEGRATION.md) for integration with OpenCode, Claude-code, and other MCP clients.

## Development

### Running Tests

```bash
# Unit tests
cargo test

# Benchmarks
cargo bench

# E2E tests (requires model files)
cargo test --test mcp_e2e_test
```

### Project Structure

```
src/
  main.rs           # MCP server entry point
  cli.rs            # CLI implementation
  config.rs         # Configuration handling
  engine/
    bq.rs           # Binary quantization
    funnel.rs       # Search funnel coordinator
    ingestion.rs    # Document ingestion pipeline
    matryoshka.rs   # Matryoshka embedding slicing
    search_stage1.rs # Hamming distance scan
    search_stage2.rs # Matryoshka refinement
    search_stage3.rs # Full re-ranking
  model/
    nomic.rs        # Nomic embedding model
  storage/
    db.rs           # Database operations
    schema.rs       # Data schemas
    tier.rs         # Memory tiering
  mcp/
    tools.rs        # MCP tool implementations
```

## License

MIT
