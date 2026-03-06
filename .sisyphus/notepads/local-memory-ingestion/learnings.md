## Learnings
- Interacting with the `local-memory` MCP server via `skill_mcp` failed with a specific error: `The "file" argument must be of type string. Received an instance of Array`. This might be due to how the MCP client handles the `cargo run` command with arguments.
- A workaround was to use a Python script to handle the JSON-RPC handshake and tool calls by piping to the binary directly.
- Splitting large files into logical sections for `memorize` helps avoid timeouts and ensures better Knowledge Graph extraction.
- The `local-memory` namespace was used as requested to isolate these memories.
