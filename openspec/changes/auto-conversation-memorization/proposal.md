## Why

Currently, conversation history in OpenCode is ephemeral to the `local-memory` system unless manually ingested via the `memorize` tool. By automatically observing and memorizing Q&A pairs, we create a persistent, self-evolving knowledge base. This allows the agent to `recall` previous context, decisions, and explanations across different sessions, significantly improving the "long-term memory" of the system.

## What Changes

- **Conversation Observer**: Introduce a new background observer (similar to Git and Shell observers) that monitors the OpenCode SQLite database (`opencode.db`).
- **Automatic Ingestion**: New messages (user and assistant) from the OpenCode database will be automatically processed, embedded, and extracted into the Knowledge Graph.
- **Namespaced Memory**: All conversation-derived memories will be stored under a dedicated `conversation` namespace to allow for scoped retrieval.
- **Configuration**: Add `enable_conversation_observer` and `opencode_db_path` to the project configuration.

## Capabilities

### New Capabilities
- `conversation-memorization`: Automatically observes OpenCode's SQLite database to ingest and summarize Q&A pairs into the local GraphRAG system.

### Modified Capabilities
- `proactive-automation`: Extend the existing proactive automation framework to include conversation history as a first-class observed source alongside Git and Shell.

## Impact

- **Storage**: Increased storage usage in `local-memory.db` due to automated ingestion of all conversation steps.
- **Performance**: Minor background CPU/IO impact during polling and extraction of new messages.
- **Dependencies**: Adds `rusqlite` dependency usage for the observer (already present in the project).
- **Configuration**: New fields in `Config` and `openspec/config.yaml`.
