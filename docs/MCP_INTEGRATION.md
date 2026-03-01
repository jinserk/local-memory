# MCP Integration Guide

This guide explains how to integrate Local Memory (GraphRAG) with MCP-compatible clients like OpenCode, Claude Desktop, Cursor, and custom applications.

## MCP Protocol Overview

Local Memory implements the Model Context Protocol (MCP) using JSON-RPC 2.0 over stdio. It allows AI agents to store structured memories as a Knowledge Graph and retrieve them using hybrid search.

### Protocol Details

| Aspect | Value |
|--------|-------|
| Protocol Version | 2024-11-05 |
| Transport | stdio (JSON-RPC 2.0) |
| Server Name | `local-memory` |
| Server Version | `0.2.0-edgequake` |

## Tool Reference

### `memory_insert`

Ingests text, generates vector embeddings, and performs LLM-based entity/relationship extraction to build the knowledge graph.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "text": { "type": "string", "description": "The text content to remember" },
    "metadata": { "type": "object", "description": "Optional metadata (title, source, etc.)" }
  },
  "required": ["text"]
}
```

### `memory_search`

Performs a **Hybrid Search** combining vector similarity (via `sqlite-vec`) and Knowledge Graph traversal.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string", "description": "The search query" },
    "top_k": { "type": "integer", "description": "Number of results", "default": 5 }
  },
  "required": ["query"]
}
```

### `graph_get_neighborhood`

Retrieves all entities and relationships directly connected to a specific entity. This is useful for "multi-hop" reasoning where an agent needs to explore context around a known concept.

#### Input Schema
```json
{
  "type": "object",
  "properties": {
    "entity_name": { "type": "string", "description": "Entity name to explore" }
  },
  "required": ["entity_name"]
}
```

## Integration Examples

### Claude Desktop

Edit your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "local-memory": {
      "command": "/path/to/local-memory/target/release/local-memory",
      "env": {
        "OPENAI_API_KEY": "your-key-for-graph-extraction"
      }
    }
  }
}
```

### Cursor

Add a new MCP server in Cursor settings:
- **Name**: `local-memory`
- **Type**: `command`
- **Command**: `/path/to/local-memory/target/release/local-memory`

## Advanced: Entity Extraction

To enable Knowledge Graph extraction, ensure an LLM provider is configured via environment variables when starting the server:

| Provider | Variable | Note |
|----------|----------|------|
| OpenAI | `OPENAI_API_KEY` | Used for extracting nodes/edges from text |

If no LLM is configured, `memory_insert` will still store the text and vector for semantic search, but the Knowledge Graph will not be updated.

## Best Practices for Agents

1. **Use `memory_insert` for facts**: "Remember that project X uses React."
2. **Use `memory_search` for open questions**: "What do I know about the frontend tech stack?"
3. **Use `graph_get_neighborhood` for exploration**: "Tell me more about 'React' and what else it's connected to in my memory."
