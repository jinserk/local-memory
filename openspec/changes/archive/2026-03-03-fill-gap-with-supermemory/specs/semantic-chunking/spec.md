## ADDED Requirements

### Requirement: Concept Boundary Detection
The ingestion pipeline SHALL use the LLM to identify semantic topic shifts in input text rather than relying solely on fixed character counts.

#### Scenario: Splitting multi-topic text
- **WHEN** a document containing multiple distinct subjects is ingested
- **THEN** system SHALL create separate chunks at the logical transition points between those subjects

### Requirement: Contextual Parent Summaries
Each semantic chunk SHALL include a condensed summary of its parent document to provide global context during vector search.

#### Scenario: Retrieval of small chunk
- **WHEN** a small text snippet is retrieved via vector search
- **THEN** the system SHALL provide the parent summary along with the snippet to the final reasoning model
