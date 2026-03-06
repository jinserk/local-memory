# Integration QA Learnings

- The MCP server correctly implements the protocol and handles tool calls.
- `memorize` successfully extracts entities and relationships using the configured LLM (NuExtract via Ollama).
- `recall` provides hybrid results combining vector similarity and graph context.
- The Git observer successfully monitors the repository and ingests commit summaries.
- The 3-stage funnel is partially implemented: Stage 1 (BQ) and Stage 2 (Short) are active, but Stage 3 (Full Precision) is missing from the search coordinator.
