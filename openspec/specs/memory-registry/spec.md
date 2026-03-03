# memory-registry Specification

## Purpose
TBD - created by archiving change fill-gap-with-supermemory. Update Purpose after archive.
## Requirements
### Requirement: Namespace Isolation
The system SHALL support optional `namespace` or `collection` tags for documents and entities to allow scoped searches.

#### Scenario: Searching specific project
- **WHEN** user specifies a namespace (e.g., "work") in a search query
- **THEN** system SHALL only return results matching that namespace

### Requirement: Importance Decay Ranking
The retrieval funnel SHALL implement a `decay_factor` that reduces the relevance score of facts that have not been accessed or refreshed over time.

#### Scenario: Floating recent decisions
- **WHEN** multiple semantic matches exist for a query
- **THEN** system SHALL rank more recent or frequently accessed decisions higher than stale historical data

