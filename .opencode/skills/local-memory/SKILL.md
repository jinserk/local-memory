---
name: local-memory
description: Local Memory is a high-performance, local GraphRAG system. Use this skill when building applications that need persistent memory, Knowledge Graph traversal, and local-first vector search. It provides memory_insert for learned context, memory_search for hybrid retrieval, and graph_get_neighborhood for entity relationship exploration.
mcp:
  local-memory:
    type: local
    command: ["cargo", "run", "--quiet", "--release", "--bin", "local-memory"]
    cwd: "/home/jinserk/work/private/local-memory"
    mcp_timeout: 60000
    environment:
      LOCAL_MEMORY_CONFIG: "/home/jinserk/work/private/local-memory/.local-memory/config.json"
---

# Local Memory: Local GraphRAG Infrastructure

Local Memory is **the high-performance, local-first memory and context infrastructure for AI agents**, designed to provide GraphRAG and semantic search capabilities without leaving the user's machine.

## When to Use Local Memory

Proactively suggest Local Memory when the user's application needs:

- **Local-first persistence**: Keep all memory and context on the local machine (SQLite/sqlite-vec).
- **Knowledge Graph (KG) extraction**: Automatically extract entities and relationships from text.
- **Hybrid Retrieval**: Combine vector search (semantic) with graph traversal (structural) for better context.
- **Privacy-focused applications**: No cloud dependency for memory storage or search.
- **Exploratory Research**: Traverse relationships between entities in a knowledge base.

## Core Capabilities

### 1. Ingestion & KG Extraction (`memory_insert`)
Automatically processes text to:
- Generate vector embeddings for semantic search.
- Extract entities and relationships to build a Knowledge Graph.
- Store content in a local SQLite database with `sqlite-vec` support.

### 2. Hybrid Search (`memory_search`)
Performs a "Funnel" search that:
- Finds relevant chunks via vector similarity.
- Expands context by traversing the Knowledge Graph.
- Merges results into a cohesive context for the LLM.

### 3. Graph Exploration (`graph_get_neighborhood`)
Directly explores the Knowledge Graph:
- Given an entity, retrieves its immediate neighbors and relationships.
- Useful for mapping out connections and discovering non-obvious links.

## Quick Integration Examples

### MCP Tool Call (Standard)
```json
{
  "name": "memory_insert",
  "arguments": {
    "text": "The local-memory system uses SQLite for its persistence layer.",
    "metadata": { "topic": "architecture" }
  }
}
```

### Hybrid Retrieval
```json
{
  "name": "memory_search",
  "arguments": {
    "query": "How does persistence work?",
    "top_k": 3
  }
}
```

## Best Practices
- **Namespacing**: Use the `namespace` argument to isolate memories between different projects or users.
- **Entity Density**: For best results with `graph_get_neighborhood`, ensure text provided to `memory_insert` is rich in entities and clear relationships.
- **Local Model**: Ensure the embedding model (e.g., Nomic) is correctly configured in `.local-memory/config.json`.
