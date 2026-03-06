## ADDED Requirements

### Requirement: Conversation Observer
The system SHALL optionally monitor the OpenCode session database and automatically ingest new conversation steps in the background.

#### Scenario: Automatic conversation ingestion
- **WHEN** a new conversation step is completed in the agent environment
- **THEN** the system SHALL reconstruct the Q&A and store it in the knowledge graph under the `conversation` namespace
