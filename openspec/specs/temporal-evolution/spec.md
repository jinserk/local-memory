# temporal-evolution Specification

## Purpose
TBD - created by archiving change fill-gap-with-supermemory. Update Purpose after archive.
## Requirements
### Requirement: Versioned Documents
The system SHALL track versions of ingested documents using an integer `version` field.

#### Scenario: Ingesting newer version
- **WHEN** user inserts a document with the same unique identifier or content hash as an existing record but with updated facts
- **THEN** system SHALL increment the version number and store the new record as the current version

### Requirement: Latest Fact Flag
The system SHALL maintain an `is_latest` boolean flag for both documents and entities to identify the most current state of knowledge.

#### Scenario: Querying for current knowledge
- **WHEN** a retrieval request is made
- **THEN** the search funnel SHALL prioritize or filter records where `is_latest` is true unless historical context is requested

### Requirement: Fact Lifecycles
The Knowledge Graph SHALL support relationship types that define the lifecycle of facts, including `UPDATES`, `EXTENDS`, and `SUPERSEDES`.

#### Scenario: Detecting updates
- **WHEN** the LLM extractor identifies that a new sentence directly updates a previous fact
- **THEN** the system SHALL create an `UPDATES` relationship between the new and old entity/fact nodes

### Requirement: Conflict Detection
The ingestion pipeline SHALL utilize the LLM to identify if new incoming information contradicts existing facts marked as `is_latest`.

#### Scenario: Contradictory weather report
- **WHEN** user ingests "Boston is sunny" while memory contains "Boston is rainy" (is_latest=true)
- **THEN** system SHALL flag the conflict and prompt or automatically update the state based on timestamp

