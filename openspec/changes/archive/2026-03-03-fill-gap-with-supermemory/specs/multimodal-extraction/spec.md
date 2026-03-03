## ADDED Requirements

### Requirement: Vision-Language Ingestion
The system SHALL support ingesting images (PNG, JPG) and extracting structured entities and relationships from them using compatible VLM providers.

#### Scenario: Ingesting an architecture diagram
- **WHEN** a user provides an image path to the ingestion tool
- **THEN** system SHALL identify components and their connections from the image and store them in the Knowledge Graph

### Requirement: Document OCR
The system SHALL support extracting text from scanned PDF documents and images for inclusion in semantic search.

#### Scenario: Ingesting a scanned document
- **WHEN** a scanned image is ingested
- **THEN** system SHALL perform OCR to extract text before generating embeddings and graph triples
