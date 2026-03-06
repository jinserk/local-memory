## Context

`local-memory` currently observes Git commits and Shell commands. The user wants to bridge the gap between their active development session (OpenCode) and the GraphRAG memory system. OpenCode stores session data in a local SQLite database at `~/.local/share/opencode/opencode.db`.

## Goals / Non-Goals

**Goals:**
- Implement a `ConversationObserver` that polls OpenCode's SQLite database.
- Reconstruct Q&A pairs from the `message` and `part` tables.
- Automatically ingest new conversation steps into the `conversation` namespace.
- Ensure zero-configuration for the standard OpenCode path.

**Non-Goals:**
- Real-time ingestion (polling is sufficient).
- Memorizing system prompts or aborted messages.
- Handling multi-agent session synchronization beyond the local database.

## Decisions

### 1. Polling Mechanism
**Decision**: Use a polling loop (every 60 seconds) instead of file-system watchers.
**Rationale**: SQLite writes can be frequent and atomic; file-system watchers (`inotify`) would trigger on every write, while polling allows for batching new messages and simplifies the implementation.
**Alternatives**: Using `notify` crate, but it often yields false positives or missed events on locked SQLite files.

### 2. Tracking Progress
**Decision**: Store the maximum `time_created` or `id` of the last processed message in the `ConversationObserver` state.
**Rationale**: This ensures we only process new messages since the last poll.
**Alternatives**: Adding a `processed` flag to the OpenCode database (not possible as it's an external database we shouldn't modify).

### 3. Content Reconstruction
**Decision**: Merge `text` parts for a given `message_id`. For `tool` parts, extract the tool name and simplified input/output if relevant.
**Rationale**: Conversations often consist of multiple parts (text, tool calls, tool results). To provide good context for GraphRAG, we need a cohesive summary of the step.
**Alternatives**: Ingesting every part as a separate document (would fragment context).

### 4. Integration Point
**Decision**: Add `spawn_conversation_observer` to `src/engine/mod.rs` and call it from `src/main.rs`.
**Rationale**: Matches the existing pattern for Git and Shell observers.

## Risks / Trade-offs

- **[Risk]** Database Lock → **Mitigation**: Open the OpenCode database in read-only mode with query retries.
- **[Risk]** Large Conversation Ingestion → **Mitigation**: Extraction and ingestion are performed in a background task to prevent blocking the MCP server.
- **[Trade-off]** Polling Lag → A 60-second delay between a message and its appearance in memory is acceptable for "long-term" memory.
