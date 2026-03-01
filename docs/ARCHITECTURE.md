# Local Memory Architecture

This document describes the architecture of Local Memory, a high-performance GraphRAG (Graph Retrieval-Augmented Generation) memory system with MCP integration.

## System Overview

```
+------------------+     +------------------+     +------------------+
|   MCP Client     |     |   CLI (lmcli)    |     |   External       |
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
|  |    - memory_insert           - memory_search              |   |
|  |    - graph_get_neighborhood                               |   |
|  +----------------------------------------------------------+   |
+-----------------------------+-----------------------------------+
                              |
                              v
+-----------------------------+-----------------------------------+
|                        Engine Layer                             |
|  +-------------+  +------------------+  +------------------+   |
|  | Ingestion   |  |  Search Funnel   |  | LLM Graph        |   |
|  | Pipeline    |  |  Coordinator     |  | Extractor        |   |
|  +------+------+  +---------+--------+  +---------+--------+   |
|         |                  |                     |              |
|         v                  v                     v              |
|  +------+---------------------------------------------------+   |
|  |                    Hybrid Retriever                      |   |
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
|  |              SQLite Database (sqlite-vec)                |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  |  | Documents    |  | Entities    |  | Relationships   |  |   |
|  |  | (Table)      |  | (Table)     |  | (Table)         |  |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  |  | Vec Docs     |  | Vec Entity  |  | Bit Vectors     |  |   |
|  |  | (Virtual)    |  | (Virtual)   |  | (Virtual)       |  |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  +----------------------------------------------------------+   |
+-----------------------------------------------------------------+
```

## Components

### 1. MCP Server Layer (`src/main.rs`, `src/mcp/`)

The entry point for all MCP communication. Implements JSON-RPC 2.0 over stdio.

**Supported Tools:**
- `memory_insert`: Ingests text and extracts graph data.
- `memory_search`: Hybrid retrieval combining vector and graph context.
- `graph_get_neighborhood`: Direct exploration of entity connections.

### 2. Engine Layer (`src/engine/`)

#### Ingestion Pipeline (`ingestion.rs`)

Orchestrates the flow from raw text to structured Knowledge Graph:
1. **Embedding**: Generates 768d vector via local Nomic model.
2. **Extraction**: Uses `edgequake-llm` to identify entities and relationships.
3. **Storage**: Atomically commits documents, entities, and edges to SQLite.

#### Search Funnel (`funnel.rs`)

Implements **Hybrid Retrieval**:
1. **Vector Search**: Identifies the top-k most similar document chunks using `sqlite-vec`.
2. **Graph Traversal**: Expands relevant entities to find connected facts (the "neighborhood").
3. **Context Fusion**: Combines text snippets with graph metadata for the final prompt.

### 3. Model Layer (`src/model/`)

#### Nomic Embed Text v1.5 (`nomic.rs`)
Local embedding generation using `candle-transformers`.

### 4. Storage Layer (`src/storage/sqlite.rs`)

Uses SQLite with the `sqlite-vec` extension for unified storage.

**Primary Tables:**
- `documents`: Stores raw content and metadata.
- `entities`: Stores unique concepts, people, and objects.
- `relationships`: Stores directed edges between entities (triplets).
- `vec_documents`: Virtual table for sub-millisecond vector similarity search.

## Data Flow

### Hybrid Search Flow

```
Client                    MCP Server                Engine                 Storage
  |                           |                        |                      |
  |---- memory_search ------->|                        |                      |
  |                           |---- encode query ----->|                      |
  |                           |                        |---- vec search ----->|
  |                           |                        |<--- top chunks ------|
  |                           |---- extract context -->|                      |
  |                           |                        |---- traverse graph ->|
  |                           |                        |<--- related facts ---|
  |                           |<--- fused results -----|                      |
  |<--- JSON results ---------|                        |                      |
```

## Performance

- **Vector Search**: ~100-200 microseconds for 1000 documents via `sqlite-vec`.
- **Graph Traversal**: O(1) lookups for immediate neighborhood.
- **Storage Footprint**: Efficient relational storage with minimal overhead.
