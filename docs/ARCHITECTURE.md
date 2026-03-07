# Local Memory: EdgeQuake Architecture

This document describes the high-performance GraphRAG architecture of Local Memory, inspired by the "Living Knowledge" philosophy of Supermemory.

## System Overview

Local Memory operates as a local-first, zero-latency sidecar for AI agents. It combines a high-speed vector retrieval funnel with a relational knowledge graph.

```
+------------------+     +------------------+     +------------------+
|   MCP Client     |     |   CLI (lmcli)    |     |   IDE / Agent    |
| (Claude, Gemini) |     |   Setup & Diag   |     |   Extensions     |
+--------+---------+     +--------+---------+     +--------+---------+
         |                        |                        |
         | JSON-RPC 2.0           | Binary                 | Direct
         | (stdio)                | Access                 | Call
         v                        v                        v
+--------+---------------------------------------------------------+
|                         MCP Server Layer                        |
|  +----------------------------------------------------------+   |
|  |                    MCP Tools & Resources                  |   |
|  |    - memory_insert           - memory_search              |   |
|  |    - graph_get_neighborhood  - [FUTURE] context_resource  |   |
|  +----------------------------------------------------------+   |
+-----------------------------+-----------------------------------+
                              |
                              v
+-----------------------------+-----------------------------------+
|                        Engine Layer                             |
|  +-------------+  +------------------+  +------------------+   |
|  | Ingestion   |  |  Search Funnel   |  | LLM Structured   |   |
|  | Pipeline    |  |  (3-Stage)       |  | Extractor        |   |
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
|  +--------------------------+  +----------------------------+   |
|  |      Embedder (Vector)   |  |      Reasoning (Graph)     |   |
|  |  - Nomic 1.5 (Local)     |  |  - NuExtract 1.5 (Local)    |   |
|  |  - Nomic v2-moe (Ollama) |  |  - NuExtract 2.0 (Ollama)   |   |
|  +--------------------------+  +----------------------------+   |
+-----------------------------+-----------------------------------+
                              |
                              v
+-----------------------------+-----------------------------------+
|                       Storage Layer                             |
|  +----------------------------------------------------------+   |
|  |              SQLite Database (sqlite-vec)                |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  |  | Documents    |  | Entities    |  | Relationships   |  |   |
|  |  | [Temporal]   |  | [Graph]     |  | [Triples]       |  |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  |  | Vec_Bit      |  | Vec_Short   |  | Vec_Full        |  |   |
|  |  | (Hamming)    |  | (Matryoshka)|  | (Cosine)        |  |   |
|  |  +--------------+  +-------------+  +-----------------+  |   |
|  +----------------------------------------------------------+   |
+-----------------------------------------------------------------+
```

## Core Components

### 1. 3-Stage Search Funnel (`src/engine/funnel.rs`)
To achieve sub-millisecond retrieval over thousands of documents, we use a tiered funnel:
1.  **Stage 1: Binary Quantization (BQ)**: Fast Hamming distance search over packed bit-vectors. Filters top 50 candidates.
2.  **Stage 2: Matryoshka Slicing**: Re-ranks candidates using a smaller "short" vector (typically dimension/3). Filters top 20.
3.  **Stage 3: Full Precision + Graph**: Final Cosine similarity using the full 768d vector, fused with Knowledge Graph context.

### 2. Living Knowledge Graph (`src/engine/ingestion.rs`)
Unlike static databases, Local Memory tracks the evolution of facts:
*   **Temporal Ingestion**: [IN PROGRESS] Tracks versioning so newer facts (e.g., today's weather) supersede historical ones.
*   **Structured Extraction**: Uses `NuExtract` to turn natural language into JSON triples (`Source` -> `Predicate` -> `Target`).
*   **Knowledge Decay & Forgetting**: To keep the graph relevant and prevent stale information from polluting retrieval, we implement a linear decay mechanism:
    *   **Decay Factor**: Each entity starts with a `decay_factor` of 1.0.
    *   **Daily Decay**: A background service reduces the factor daily, reaching 0.0 after 180 days (6 months) of inactivity.
    *   **Recall & Survival**: Every time an entity is recalled (via search or exploration), its `decay_factor` resets to 1.0.
    *   **Pruning**: Entities with a 0.0 factor are automatically removed along with their relationships and orphaned communities.
    *   **Manual Forgetting**: The `forget` operation allows explicit removal of a fact by setting its decay factor to 0.0 immediately.

### 3. Unified Model Provider (`src/model/`)
We use an **Asymmetric Model Factory**:
*   **`CandleProvider`**: High-speed local BERT/Phi-3 execution using Rust.
*   **`GenericUnifiedModel`**: Allows mixing cloud APIs (OpenAI) with local servers (Ollama) for different tasks in the same session.

## Data Lifecycle

1.  **Ingest**: Raw text is passed to `memory_insert`.
2.  **Embed**: The text is prefixed with `search_document: ` and vectorized.
3.  **Extract**: The model identifies entities (e.g., `Boston`) and links (e.g., `IN` -> `Massachusetts`).
4.  **Store**: All data is committed to a single SQLite file, including 3 tiers of vectors.
5.  **Retrieve**: `memory_search` query is prefixed with `search_query: `, run through the funnel, and returned with Graph context.
