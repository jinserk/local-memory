use crate::engine::funnel::SearchFunnel;
use crate::engine::ingestion::IngestionPipeline;
use crate::model::nomic::Embedder;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

pub fn list_tools() -> Value {
    json!([
        {
            "name": "memory_insert",
            "description": "Insert a new memory into the local database",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text content to remember"
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Optional metadata associated with the memory"
                    }
                },
                "required": ["text"]
            }
        },
        {
            "name": "memory_search",
            "description": "Search for relevant memories in the local database",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "The number of results to return",
                        "default": 5
                    }
                },
                "required": ["query"]
            }
        }
    ])
}

pub fn call_tool(
    name: &str,
    arguments: Value,
    pipeline: &IngestionPipeline,
    funnel: &SearchFunnel<'_>,
    embedder: &dyn Embedder,
) -> Result<Value> {
    match name {
        "memory_insert" => {
            let text = arguments
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'text' argument"))?;
            let metadata = arguments.get("metadata").cloned().unwrap_or(json!({}));

            let id = pipeline.run(text, metadata)?;
            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Memory inserted with ID: {}", id)
                    }
                ]
            }))
        }
        "memory_search" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing 'query' argument"))?;
            let top_k = arguments.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

            let query_vector = embedder.encode(query)?;
            let results = funnel.search(&query_vector, top_k)?;

            let formatted_results: Vec<Value> = results
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.id,
                        "score": r.score,
                        "metadata": r.metadata
                    })
                })
                .collect();

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&formatted_results)?
                    }
                ]
            }))
        }
        _ => Err(anyhow!("Unknown tool: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::db::Database;
    use std::sync::Arc;
    use tempfile::tempdir;

    struct MockEmbedder;
    impl Embedder for MockEmbedder {
        fn encode(&self, _text: &str) -> Result<Vec<f32>> {
            Ok(vec![1.0; 768])
        }
    }

    #[test]
    fn test_memory_insert_mock() -> Result<()> {
        let dir = tempdir()?;
        let db = Arc::new(Database::open(dir.path())?);
        let config = Config::default();
        let embedder = Arc::new(MockEmbedder);
        let pipeline = IngestionPipeline::new(embedder.clone(), db.clone());
        let funnel = SearchFunnel::new(&db, &config);

        let args = json!({
            "text": "test memory",
            "metadata": {"source": "unit-test"}
        });

        let result = call_tool("memory_insert", args, &pipeline, &funnel, embedder.as_ref())?;

        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("Memory inserted with ID:"));

        Ok(())
    }
}
