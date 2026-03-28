# MCP Integration Guide

This guide describes how to integrate Local Memory with MCP clients (OpenCode, Claude, etc.) to give them "Supermemory" capabilities.

## Protocol Details

Local Memory implements the Model Context Protocol (MCP) using JSON-RPC 2.0 over stdio.

| Aspect | Value |
|--------|-------|
| Server Name | `local-memory` |
| Capabilities | Tools |

## Tool Reference

### `memorize`
Ingests text into the vectorized database and Knowledge Graph.
- **Workflow**: Embed -> Extract -> Store.
- **Auto-Formatting**: If using a local `NuExtract` model, the server automatically formats the prompt into the required JSON template format.

### `recall`
The primary retrieval tool.
- **Logic**: Performs a 3-stage hybrid search.
- **Output**: Returns relevant text snippets along with their related Knowledge Graph entities and relationships.

### `explore`
Explores the graph directly.
- **Use Case**: When an agent already knows an entity (e.g., "Project EdgeQuake") and wants to see everything connected to it without doing a vector search.

---

## Best Practices for Agents

To act like a "Living Knowledge" system, agents should follow these patterns:

### 1. Proactive Memory Retrieval
Instead of asking the user, the agent should call `recall` at the start of a session to see if there is relevant history or previous architectural decisions.

### 2. Fact Consolidation
When an agent creates a new file or fixes a bug, it should call `memorize` with a brief summary:
*"I fixed the CORS issue in the worker by adding the correct headers to the response object."*

### 3. Multi-Hop Reasoning
Agents can use the output of `recall` to find entities, then call `explore` on those entities to discover deep connections that weren't in the initial text snippets.
