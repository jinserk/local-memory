# MCP Integration Guide

This guide explains how to integrate Local Memory with MCP-compatible clients like OpenCode, Claude-code, and custom applications.

## MCP Protocol Overview

The Model Context Protocol (MCP) is a standard for connecting AI models to external tools and data sources. Local Memory implements MCP v1.0 using JSON-RPC 2.0 over stdio.

### Protocol Details

| Aspect | Value |
|--------|-------|
| Protocol Version | MCP v1.0 |
| Transport | stdio (JSON-RPC 2.0) |
| Server Name | `local-memory` |
| Server Version | `0.1.0` |

### Server Capabilities

```json
{
  "capabilities": {
    "tools": {
      "listChanged": true
    }
  },
  "serverInfo": {
    "name": "local-memory",
    "version": "0.1.0"
  }
}
```

## Tool Reference

### memory_insert

Insert a new memory into the local database.

#### Input Schema

```json
{
  "type": "object",
  "properties": {
    "text": {
      "type": "string",
      "description": "The text content to remember"
    },
    "metadata": {
      "type": "object",
      "description": "Optional metadata associated with the memory"
    }
  },
  "required": ["text"]
}
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `text` | string | Yes | The text content to store |
| `metadata` | object | No | Additional metadata (category, source, tags, etc.) |

#### Example Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "memory_insert",
    "arguments": {
      "text": "The user prefers vim keybindings in their editor",
      "metadata": {
        "category": "preference",
        "source": "conversation",
        "importance": "high"
      }
    }
  }
}
```

#### Example Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Memory inserted with ID: 550e8400-e29b-41d4-a716-446655440000"
      }
    ]
  }
}
```

### memory_search

Search for relevant memories using semantic similarity.

#### Input Schema

```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "The search query"
    },
    "top_k": {
      "type": "integer",
      "description": "The number of results to return",
      "default": 5
    }
  },
  "required": ["query"]
}
```

#### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | - | Natural language search query |
| `top_k` | integer | No | 5 | Number of results to return |

#### Example Request

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "memory_search",
    "arguments": {
      "query": "editor preferences and keybindings",
      "top_k": 3
    }
  }
}
```

#### Example Response

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "[\n  {\n    \"id\": \"550e8400-e29b-41d4-a716-446655440000\",\n    \"score\": 0.8923,\n    \"metadata\": {\n      \"text\": \"The user prefers vim keybindings in their editor\",\n      \"category\": \"preference\",\n      \"source\": \"conversation\",\n      \"importance\": \"high\"\n    }\n  },\n  {\n    \"id\": \"660e8400-e29b-41d4-a716-446655440001\",\n    \"score\": 0.7845,\n    \"metadata\": {\n      \"text\": \"User uses dark mode for reduced eye strain\",\n      \"category\": \"preference\"\n    }\n  }\n]"
      }
    ]
  }
}
```

## Integration Guides

### OpenCode

Add Local Memory to your OpenCode configuration:

**Method 1: Configuration File**

Edit your OpenCode config file (typically `~/.config/opencode/config.json`):

```json
{
  "mcpServers": {
    "local-memory": {
      "command": "/path/to/local-memory/target/release/local-memory",
      "args": [],
      "env": {
        "LOCAL_MEMORY_CONFIG": "/path/to/config.json"
      }
    }
  }
}
```

**Method 2: Project-local Configuration**

Create `.opencode/mcp.json` in your project:

```json
{
  "servers": {
    "local-memory": {
      "command": "cargo",
      "args": ["run", "--release", "--manifest-path", "../local-memory/Cargo.toml"],
      "cwd": "../local-memory"
    }
  }
}
```

### Claude-code

Configure Claude-code to use Local Memory:

**Desktop App Configuration**

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or equivalent on other platforms:

```json
{
  "mcpServers": {
    "local-memory": {
      "command": "/path/to/local-memory/target/release/local-memory",
      "env": {
        "LOCAL_MEMORY_CONFIG": "/path/to/local-memory/config.json"
      }
    }
  }
}
```

**With Custom Storage Path**

```json
{
  "mcpServers": {
    "local-memory": {
      "command": "/path/to/local-memory/target/release/local-memory",
      "env": {
        "LOCAL_MEMORY_CONFIG": "/path/to/local-memory/config.json"
      },
      "storage": {
        "path": "/custom/storage/path"
      }
    }
  }
}
```

### Custom Integration

For custom applications, spawn the Local Memory server as a subprocess:

#### Python Example

```python
import subprocess
import json

class LocalMemoryClient:
    def __init__(self, executable_path, config_path=None):
        env = {}
        if config_path:
            env["LOCAL_MEMORY_CONFIG"] = config_path
        
        self.process = subprocess.Popen(
            [executable_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            text=True
        )
        self.request_id = 0
    
    def _send_request(self, method, params=None):
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params or {}
        }
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        response = self.process.stdout.readline()
        return json.loads(response)
    
    def initialize(self):
        return self._send_request("initialize")
    
    def list_tools(self):
        return self._send_request("tools/list")
    
    def insert_memory(self, text, metadata=None):
        args = {"text": text}
        if metadata:
            args["metadata"] = metadata
        return self._send_request("tools/call", {
            "name": "memory_insert",
            "arguments": args
        })
    
    def search_memories(self, query, top_k=5):
        return self._send_request("tools/call", {
            "name": "memory_search",
            "arguments": {
                "query": query,
                "top_k": top_k
            }
        })
    
    def close(self):
        self.process.terminate()
        self.process.wait()

# Usage
client = LocalMemoryClient("./target/release/local-memory")
client.initialize()

# Insert a memory
result = client.insert_memory(
    "User prefers TypeScript over JavaScript",
    {"category": "preference", "source": "interview"}
)
print(result)

# Search memories
results = client.search_memories("programming language preferences")
print(results)

client.close()
```

#### Node.js Example

```javascript
const { spawn } = require('child_process');

class LocalMemoryClient {
  constructor(executablePath, configPath = null) {
    const env = { ...process.env };
    if (configPath) {
      env.LOCAL_MEMORY_CONFIG = configPath;
    }
    
    this.process = spawn(executablePath, [], { env });
    this.requestId = 0;
    this.pendingRequests = new Map();
    
    let buffer = '';
    this.process.stdout.on('data', (data) => {
      buffer += data;
      const lines = buffer.split('\n');
      buffer = lines.pop();
      
      for (const line of lines) {
        if (line.trim()) {
          const response = JSON.parse(line);
          const resolver = this.pendingRequests.get(response.id);
          if (resolver) {
            this.pendingRequests.delete(response.id);
            resolver(response);
          }
        }
      }
    });
  }
  
  sendRequest(method, params = {}) {
    return new Promise((resolve) => {
      const id = ++this.requestId;
      this.pendingRequests.set(id, resolve);
      
      const request = {
        jsonrpc: '2.0',
        id,
        method,
        params
      };
      
      this.process.stdin.write(JSON.stringify(request) + '\n');
    });
  }
  
  async initialize() {
    return this.sendRequest('initialize');
  }
  
  async listTools() {
    return this.sendRequest('tools/list');
  }
  
  async insertMemory(text, metadata = null) {
    const args = { text };
    if (metadata) args.metadata = metadata;
    
    return this.sendRequest('tools/call', {
      name: 'memory_insert',
      arguments: args
    });
  }
  
  async searchMemories(query, topK = 5) {
    return this.sendRequest('tools/call', {
      name: 'memory_search',
      arguments: {
        query,
        top_k: topK
      }
    });
  }
  
  close() {
    this.process.kill();
  }
}

// Usage
async function main() {
  const client = new LocalMemoryClient('./target/release/local-memory');
  await client.initialize();
  
  // Insert a memory
  const insertResult = await client.insertMemory(
    'User works remotely from Pacific timezone',
    { category: 'work-info' }
  );
  console.log(insertResult);
  
  // Search memories
  const searchResults = await client.searchMemories('user location timezone');
  console.log(searchResults);
  
  client.close();
}

main();
```

## Best Practices

### Memory Organization

Use consistent metadata schemas for better organization:

```json
{
  "text": "User prefers React over Vue for frontend development",
  "metadata": {
    "category": "technical-preference",
    "domain": "frontend",
    "confidence": "high",
    "source": "explicit-statement",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

### Query Formulation

- Use natural language queries that describe what you're looking for
- Include context in queries for better semantic matching
- Use `top_k` appropriate to your use case (5-10 for most cases)

### Memory Types

Choose the appropriate tier for different types of memories:

| Memory Type | Tier | TTL | Example |
|-------------|------|-----|---------|
| User preferences | Semantic | Permanent | "User prefers dark mode" |
| Learned facts | Semantic | Permanent | "User's name is Alice" |
| Session context | Episodic | 1 hour | "Currently discussing project X" |
| Temporary state | Episodic | 5 minutes | "Just mentioned feeling tired" |

## Error Handling

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Internal error: Missing 'text' argument"
  }
}
```

### Common Error Codes

| Code | Meaning | Cause |
|------|---------|-------|
| -32700 | Parse error | Invalid JSON in request |
| -32600 | Invalid Request | Missing required fields |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Missing or invalid arguments |
| -32603 | Internal error | Server-side error |

## Troubleshooting

### Server Won't Start

1. Check that model files exist in `models/` directory
2. Verify `config.json` is valid JSON
3. Check storage path is writable

### Poor Search Results

1. Increase `stage1_k` in configuration for higher recall
2. Ensure queries are semantically similar to stored content
3. Check that memories were inserted successfully

### Memory Not Found

1. Verify the memory was inserted (check insert response)
2. Check if the memory is episodic and may have expired
3. Use CLI `mem-diag inspect <uuid>` to verify storage
