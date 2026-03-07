# Core Specification: GraphRAG Memory System

## 1. System Overview
Local Memory is a high-performance local GraphRAG (Graph Retrieval-Augmented Generation) system. It enhances traditional vector retrieval by extracting a structured Knowledge Graph (entities and relationships) from ingested text, enabling multi-hop reasoning and high-context retrieval.

## 2. Technical Architecture

### Storage Layer
- **Engine**: SQLite 3
- **Vector Extension**: `sqlite-vec` (using `vec0` virtual tables)
- **Schema**:
  - `documents`: Raw text, titles, and metadata.
  - `entities`: Unique nodes with `decay_factor` and `last_recalled_at`.
  - `relationships`: Directed edges connecting entities.
  - `vec_documents`: Vector index for document embeddings.
  - `communities`: Clustered entities with summaries.

### Engine Layer
- **Knowledge Evolution**:
  - **Decay Factor**: Linear decay of entity relevance (1.0 -> 0.0 over 180 days).
  - **Recall**: Accessing an entity resets its decay factor to 1.0.
  - **Pruning**: Automated 24-hour maintenance to remove zero-factor entities, relationships, and orphaned communities.
  - **Ranking**: Entity listings and searches prioritize higher `decay_factor` for relevant results.
- **Ingestion Pipeline**:
  1. **Embedding**: Generate vectors.
  2. **KG Extraction**: Extract nodes and edges.
  3. **Atomic Commit**: Storing data with initial `decay_factor`.
- **Retrieval Pipeline (Hybrid Funnel)**:
  1. **Vector Search**: KNN search against document embeddings.
  2. **Graph Expansion**: Traversal of extracted entities to find related facts.
  3. **Context Fusion**: Merging and ranking text chunks and graph triplets.

## 3. MCP Interface
The system implements the Model Context Protocol (MCP) using JSON-RPC 2.0 over stdio.

### Tools
- `memorize`: Ingests text and updates the Knowledge Graph.
- `recall`: Performs hybrid retrieval for a given query.
- `explore`: Explores connections for a specific entity.
- `forget`: Resets an entity's decay factor to 0.0 for subsequent pruning.

## 4. CLI Interface (lmcli)
A diagnostic and exploration tool for the local database.
- `lmcli stats`: Database and Knowledge Graph statistics.
- `lmcli list-entities / list-relations`: Direct inspection of the graph.
- `lmcli search`: Testing hybrid retrieval from the command line.
- `lmcli test`: System self-verification and initialization.

## 5. Directory Structure
- `src/engine/`: Ingestion and retrieval logic.
- `src/storage/`: SQLite and `sqlite-vec` integration.
- `src/mcp/`: Tool definitions and RPC handling.
- `src/model/`: Local embedding model execution.
- `src/bin/`: CLI binary implementation.
