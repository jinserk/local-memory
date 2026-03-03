use crate::engine::funnel::SearchFunnel;
use crate::engine::ingestion::IngestionPipeline;
use crate::storage::sqlite::SqliteDatabase;
use crate::model::UnifiedModel;
use crate::KnowledgeEvent;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct McpContext {
    pub db: Arc<SqliteDatabase>,
    pub model: Arc<dyn UnifiedModel>,
    pub config: crate::config::Config,
    pub event_tx: broadcast::Sender<KnowledgeEvent>,
}

impl McpContext {
    pub fn get_pipeline(&self) -> IngestionPipeline {
        IngestionPipeline::new(
            self.model.clone(), 
            self.db.clone(), 
            Some(self.model.clone()),
            self.config.semantic_chunking,
            Some(self.event_tx.clone())
        )
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
                    "metadata": { "type": "object", "description": "Optional metadata associated with the memory" },
                    "namespace": { "type": "string", "description": "Optional namespace for isolation (default: 'default')" }
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
                    "top_k": { "type": "integer", "description": "The number of results to return", "default": 5 },
                    "namespace": { "type": "string", "description": "Optional namespace to filter by" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "memory_global_search",
            "description": "Perform a holistic search over community summaries to answer broad thematic questions",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The thematic search query" },
                    "namespace": { "type": "string", "description": "Optional namespace (default: 'default')" }
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
                    "entity_name": { "type": "string", "description": "The name of the entity to explore" },
                    "namespace": { "type": "string", "description": "Optional namespace" }
                },
                "required": ["entity_name"]
            }
        }
    ])
}

pub fn list_resources() -> Value {
    json!([
        {
            "uri": "local-memory://current-context",
            "name": "Current Memory Context",
            "description": "Proactive context containing recent architectural decisions and session history",
            "mimeType": "application/json"
        }
    ])
}

pub async fn call_tool(name: &str, arguments: Value, context: &McpContext) -> Result<Value> {
    match name {
        "memory_insert" => {
            let text = arguments.get("text").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'text' argument"))?;
            let metadata = arguments.get("metadata").cloned().unwrap_or(json!({}));
            let namespace = arguments.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

            let id = context.get_pipeline().run_auto(text, metadata, namespace).await?;
            Ok(json!({
                "content": [{"type": "text", "text": format!("Memory inserted and knowledge graph updated. ID: {}", id)}]
            }))
        }
        "memory_search" => {
            let query = arguments.get("query").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'query' argument"))?;
            let top_k = arguments.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let namespace = arguments.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

            let query_vector = context.model.embed_one(query).await
                .map_err(|e| anyhow!("Embedding failed: {}", e))?;
            
            let results = context.get_funnel().hybrid_search_with_namespace(&query_vector, top_k, namespace)?;

            let formatted_results: Vec<Value> = results.into_iter()
                .map(|r| json!({"id": r.id, "score": r.score, "metadata": r.metadata, "context": r.context}))
                .collect();

            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&formatted_results)?}]
            }))
        }
        "memory_global_search" => {
            let query = arguments.get("query").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'query' argument"))?;
            
            let results = handle_global_search(query, context).await?;
            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&results)?}]
            }))
        }
        "graph_get_neighborhood" => {
            let entity_name = arguments.get("entity_name").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'entity_name' argument"))?;
            let namespace = arguments.get("namespace").and_then(|v| v.as_str()).unwrap_or("default");

            let neighborhood = context.db.get_neighborhood_with_namespace(entity_name, namespace)?;
            Ok(json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&neighborhood)?}]
            }))
        }
        _ => Err(anyhow!("Unknown tool: {}", name)),
    }
}

async fn handle_global_search(query: &str, context: &McpContext) -> Result<Value> {
    let summaries_list = context.db.list_community_summaries(20)?;
    let summaries: Vec<String> = summaries_list.into_iter()
        .map(|(_, title, summary)| format!("### {}\n{}", title, summary))
        .collect();

    if summaries.is_empty() {
        return Ok(json!({"message": "No communities found to perform global search."}));
    }

    let prompt = format!(
        "You are an expert knowledge synthesizer. Answer the following user question based ONLY on the community summaries provided below.\n\n\
         Summaries:\n{}\n\n\
         Question: {}\n\n\
         Answer:",
        summaries.join("\n\n---\n\n"),
        query
    );

    let response = context.model.complete(&prompt).await?;
    Ok(json!({
        "answer": response.content,
        "source_communities_count": summaries.len()
    }))
}

pub async fn read_resource(uri: &str, context: &McpContext) -> Result<Value> {
    match uri {
        "local-memory://current-context" => {
            let recent = context.db.list_entities(10)?;
            Ok(json!({
                "contents": [
                    {
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&recent)?
                    }
                ]
            }))
        }
        _ => Err(anyhow!("Unknown resource: {}", uri)),
    }
}
