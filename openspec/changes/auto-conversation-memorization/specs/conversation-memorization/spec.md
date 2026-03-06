## ADDED Requirements

### Requirement: OpenCode Database Observation
The system SHALL monitor the OpenCode SQLite database (`opencode.db`) for new conversation entries.

#### Scenario: Detecting new messages
- **WHEN** a new row is added to the `message` table in `opencode.db`
- **THEN** the system SHALL extract the associated text parts for ingestion

### Requirement: Conversation Part Extraction
The system SHALL extract and reconstruct user and assistant messages from the `part` table in `opencode.db`.

#### Scenario: Reconstructing a Q&A pair
- **WHEN** an assistant message is completed following a user message
- **THEN** the system SHALL reconstruct the pair as a single textual unit for memorization

### Requirement: Automated GraphRAG Ingestion
The system SHALL automatically ingest reconstructed conversation units into the local GraphRAG system.

#### Scenario: Memorizing a conversation step
- **WHEN** a new Q&A pair is reconstructed
- **THEN** the system SHALL call the ingestion pipeline to `memorize` the text in the `conversation` namespace
