use crate::engine::funnel::SearchFunnel;
use crate::engine::ingestion::IngestionPipeline;
use crate::storage::sqlite::SqliteDatabase;
use crate::model::UnifiedModel;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct McpContext {
    pub db: Arc<SqliteDatabase>,
    pub model: Arc<dyn UnifiedModel>,
    pub config: crate::config::Config,
}

impl McpContext {
    pub fn get_pipeline(&self) -> IngestionPipeline {
        IngestionPipeline::new(self.model.clone(), self.db.clone(), Some(self.model.clone()))
    }

    pub fn get_funnel(&self) -> SearchFunnel<'_> {
        SearchFunnel::new_sqlite(&self.db, &self.config)
    }
}

pub fn list_tools() -> Value {
    json!([
        {
            "name": "memory_insert",
            "description": "Insert a new memory into the local database and extract knowledge graph",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "The text content to remember" },
                    "metadata": { "type": "object", "description": "Optional metadata associated with the memory" }
                },
                "required": ["text"]
            }
        },
        {
            "name": "memory_search",
            "description": "Search for relevant memories using hybrid vector and graph search",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The search query" },
                    "top_k": { "type": "integer", "description": "The number of results to return", "default": 5 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "graph_get_neighborhood",
            "description": "Explore an entity's neighborhood in the knowledge graph",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "entity_name": { "type": "string", "description": "The name of the entity to explore" }
                },
                "required": ["entity_name"]
            }
        }
    ])
}

pub async fn call_tool(name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
    match name {
        "memory_insert" => {
            let text = arguments.get("text").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'text' argument"))?;
            let metadata = arguments.get("metadata").cloned().unwrap_or(json!({}));

            let id = context.get_pipeline().run(text, metadata).await?;
            Ok(json!({
                "content": [{"type": "text", "text": format!("Memory inserted and knowledge graph updated. ID: {}", id)}]
            }))
        }
        "memory_search" => {
            let query = arguments.get("query").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'query' argument"))?;
            let top_k = arguments.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

            let query_vector = context.model.embed_one(query).await
                .map_err(|e| anyhow!("Embedding failed: {}", e))?;
            
            let results = context.get_funnel().hybrid_search(&query_vector, top_k)?;

            let formatted_results: Vec<Value> = results.into_iter()
                .map(|r| json!({"id": r.id, "score": r.score, "metadata": r.metadata, "context": r.context}))
                .collect();

            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&formatted_results)?}]
            }))
        }
        "graph_get_neighborhood" => {
            let entity_name = arguments.get("entity_name").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'entity_name' argument"))?;

            let neighborhood = context.db.get_neighborhood(entity_name)?;
            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&neighborhood)?}]
            }))
        }
        _ => Err(anyhow!("Unknown tool: {}", name)),
    }
}
