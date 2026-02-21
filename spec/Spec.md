# System Specification: MCP Interface

## Protocol
The system implements the Model Context Protocol (MCP) v1.0 to provide a standardized interface for AI agents.

## JSON-RPC Methods

### memory/insert
Adds a new memory entry to the engine.
- **Parameters**:
  - `text`: The raw text content to store.
  - `metadata`: Optional key-value pairs for filtering.
- **Returns**:
  - `id`: Unique identifier for the stored memory.

### memory/search
Queries the memory engine for relevant entries.
- **Parameters**:
  - `query`: The search string.
  - `top_k`: Number of results to return (default: 10).
  - `filters`: Optional metadata filters.
- **Returns**:
  - `results`: List of objects containing `text`, `metadata`, and `score`.

### memory/delete
Removes memory entries.
- **Parameters**:
  - `id`: Specific identifier to delete.
  - `filters`: Metadata filters to delete multiple entries.
- **Returns**:
  - `success`: Boolean indicating completion.

## Technical Requirements
- **Embedding Model**: `nomic-embed-text-v1.5` (768 dimensions).
- **Quantization**: 1-bit Binary Quantization for Stage 1.
- **Dimensionality Reduction**: 256-dimensional Matryoshka subset for Stage 2.
- **Consistency**: Eventual consistency for search results after insertion.
