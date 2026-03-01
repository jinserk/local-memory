# Local Memory (GraphRAG)

A high-performance local GraphRAG (Graph Retrieval-Augmented Generation) system with MCP (Model Context Protocol) integration. Inspired by the **EdgeQuake** project and the **LightRAG** algorithm, this system extracts knowledge graphs from your documents to enable multi-hop reasoning and sophisticated memory retrieval, all running locally on your machine.

## Features

- **GraphRAG Engine**: Beyond simple vector similarity, it extracts entities and relationships to build a structured Knowledge Graph.
- **Hybrid Search**: Combines traditional vector similarity with graph traversal for context-rich retrieval.
- **SQLite Storage**: Uses SQLite with the `sqlite-vec` extension for unified, efficient storage of documents, entities, relationships, and embeddings.
- **Local LLM Integration**: Uses `edgequake-llm` to interface with local providers (like Ollama) or cloud APIs (OpenAI) for entity extraction.
- **MCP Protocol Support**: Native support for Model Context Protocol, making it compatible with OpenCode, Claude Desktop, Cursor, and other AI agents.
- **lmcli Tool**: A powerful CLI for inspecting, testing, and exploring your local knowledge graph.
- **Local-first**: Privacy-focused design where everything runs on your hardware.

## Installation

### Prerequisites

- Rust 1.80+ (2024 edition)
- Nomic Embed Text v1.5 model files (for local embeddings)
- `sqlite3` installed on your system

### Build from Source

```bash
git clone <repository-url>
cd local-memory
cargo build --release
```

## Quick Start

### 1. Initialize and Test
Run the diagnostic test to create your local database and verify the system:
```bash
./target/release/lmcli test
```

### 2. Running the MCP Server
Spawning the server for use with an AI agent:
```bash
# Using default configuration
cargo run --release

# With an LLM provider enabled for entity extraction
OPENAI_API_KEY=your_key_here cargo run --release
```

The MCP server communicates via stdio using JSON-RPC 2.0.

### 3. CLI Exploration
The `lmcli` binary provides tools to inspect your memory:

```bash
# Show database statistics
./target/release/lmcli stats

# List extracted entities
./target/release/lmcli list-entities

# List knowledge graph relationships
./target/release/lmcli list-relations

# Perform a hybrid search
./target/release/lmcli search "How does X relate to Y?"
```

## MCP Tools

Local Memory exposes several tools to AI agents:

#### `memory_insert`
Ingests text, generates embeddings, and extracts knowledge graph entities/relationships.
```json
{
  "name": "memory_insert",
  "arguments": {
    "text": "Alice is a software engineer at Acme Corp.",
    "metadata": { "title": "Employee Directory" }
  }
}
```

#### `memory_search`
Performs a hybrid search across vectors and the knowledge graph.
```json
{
  "name": "memory_search",
  "arguments": {
    "query": "Who works at Acme Corp?",
    "top_k": 5
  }
}
```

#### `graph_get_neighborhood`
Explores the immediate connections of a specific entity in the graph.
```json
{
  "name": "graph_get_neighborhood",
  "arguments": { "entity_name": "Acme Corp" }
}
```

## Architecture

Local Memory uses a modular architecture:
- **Storage Layer**: SQLite + `sqlite-vec` for relational and vector data.
- **Ingestion Pipeline**: Text -> Embedding -> LLM Entity Extraction -> Graph Storage.
- **Search Funnel**: Query -> Vector Search -> Graph Traversal -> Context Fusion.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for more details.

## Development

### Running Tests
```bash
# Run GraphRAG integration tests
cargo test --test edgequake_integration_test
```

## License

MIT
