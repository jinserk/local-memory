pub mod cli;
pub mod config;
pub mod engine;
pub mod mcp;
pub mod model;
pub mod storage;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeEvent {
    DocumentInserted { id: Uuid, title: String, namespace: String },
    EntityInserted { id: Uuid, name: String, namespace: String },
    RelationshipInserted { source_id: Uuid, target_id: Uuid, predicate: String },
}
