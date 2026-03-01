# Core Specification: GraphRAG Memory System

## 1. System Overview
Local Memory is a high-performance local GraphRAG (Graph Retrieval-Augmented Generation) system. It enhances traditional vector retrieval by extracting a structured Knowledge Graph (entities and relationships) from ingested text, enabling multi-hop reasoning and high-context retrieval.

## 2. Technical Architecture

### Storage Layer
- **Engine**: SQLite 3
- **Vector Extension**: `sqlite-vec` (using `vec0` virtual tables)
- **Schema**:
  - `documents`: Raw text, titles, and metadata.
  - `entities`: Unique nodes (People, Organizations, Concepts, etc.).
  - `relationships`: Directed edges (triplets) connecting entities.
  - `vec_documents`: High-performance vector index for document embeddings.

### Engine Layer
- **Ingestion Pipeline**:
  1. **Embedding**: Generate 768d vectors via Nomic Embed Text v1.5.
  2. **KG Extraction**: LLM-powered extraction of nodes and edges via `edgequake-llm`.
  3. **Atomic Commit**: Storing both vector and graph data in a single SQLite transaction.
- **Retrieval Pipeline (Hybrid Funnel)**:
  1. **Vector Search**: KNN search against document embeddings.
  2. **Graph Expansion**: Traversal of extracted entities to find related facts.
  3. **Context Fusion**: Merging and ranking text chunks and graph triplets.

## 3. MCP Interface
The system implements the Model Context Protocol (MCP) using JSON-RPC 2.0 over stdio.

### Tools
- `memory_insert`: Ingests text and updates the Knowledge Graph.
- `memory_search`: Performs hybrid retrieval for a given query.
- `graph_get_neighborhood`: Explores connections for a specific entity.

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
