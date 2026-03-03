## ADDED Requirements

### Requirement: 3-Stage Funnel with Temporal Filtering
The retrieval funnel SHALL filter out superseded facts by default using the `is_latest` flag during Stage 1 (Hamming Search).

#### Scenario: Retrieval excludes stale facts
- **WHEN** multiple versions of a fact exist in the database
- **THEN** only the version marked `is_latest=true` SHALL be considered for Stage 2 and Stage 3 processing unless explicitly overridden

### Requirement: Namespace Aware Retrieval
The system SHALL support filtering retrieval results by `namespace`.

#### Scenario: Project-specific search
- **WHEN** a search query includes a namespace parameter
- **THEN** the funnel SHALL restrict all three stages of search to records matching that namespace

### Requirement: Decay-Adjusted Similarity
The system SHALL combine cosine similarity scores with importance decay scores to produce a final rank.

#### Scenario: Ranking by relevance and recency
- **WHEN** two chunks have similar cosine distance
- **THEN** the chunk with the higher importance score (lower decay) SHALL be ranked higher
