## ADDED Requirements

### Requirement: Git Commit Observer
The system SHALL optionally monitor the local `.git/` directory and automatically ingest/summarize new commits in the background.

#### Scenario: Automatic commit ingestion
- **WHEN** a new commit is detected in the workspace
- **THEN** system SHALL generate a summary of the diff and store it in the knowledge graph

### Requirement: Proactive MCP Resources
The MCP server SHALL implement Resource endpoints that provide current session context to agents without requiring an explicit tool call.

#### Scenario: Agent reading context
- **WHEN** an MCP-compatible client initializes
- **THEN** system SHALL expose `local-memory://current-context` as a readable resource containing recent architectural decisions
