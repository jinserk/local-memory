# Project Artifacts and Structure

## Directory Map

### src/engine
Contains the core logic for the 3-stage funnel and search orchestration.
- `funnel.rs`: Implementation of BQ, Matryoshka, and Full reranking.
- `search.rs`: High-level search API.

### src/model
Defines data structures for embeddings and quantization logic.
- `vector.rs`: Embedding types and distance metrics.
- `quantization.rs`: Logic for 1-bit BQ and Matryoshka slicing.

### src/storage
Integration with the Fjall storage engine.
- `db.rs`: Database initialization and CRUD operations.
- `schema.rs`: Key-value layout for vectors and metadata.

### src/mcp
Implementation of the Model Context Protocol interface.
- `server.rs`: JSON-RPC server setup.
- `handlers.rs`: Logic for `memory/insert`, `memory/search`, and `memory/delete`.

## External Dependencies
- `swiftide`: RAG orchestration.
- `fjall`: LSM-tree storage.
- `simsimd`: Accelerated vector operations.
- `nomic-embed-text-v1.5`: Embedding model.
