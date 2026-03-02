use crate::storage::sqlite::SqliteDatabase;
use crate::engine::vectors::{encode_bq, slice_vector};
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;
use edgequake_llm::{LLMProvider, EmbeddingProvider};
use serde_json::json;

pub struct IngestionPipeline {
    embedder: Arc<dyn EmbeddingProvider>,
    db: Arc<SqliteDatabase>,
    llm: Option<Arc<dyn LLMProvider>>,
}

impl IngestionPipeline {
    pub fn new(
        embedder: Arc<dyn EmbeddingProvider>, 
        db: Arc<SqliteDatabase>,
        llm: Option<Arc<dyn LLMProvider>>
    ) -> Self {
        Self { embedder, db, llm }
    }

    pub async fn run(&self, text: &str, metadata: serde_json::Value) -> Result<Uuid> {
        let id = Uuid::new_v4();

        // 1. Generate FULL embedding via Unified Provider
        let v_full = self.embedder.embed_one(text).await
            .map_err(|e| anyhow::anyhow!("Embedding failed: {}", e))?;
        
        // 2. Generate Matryoshka (256d)
        let v_short = slice_vector(&v_full, 256);
        
        // 3. Generate BQ (768-bit)
        let v_bit = encode_bq(&v_full);

        // 4. Update metadata to include raw text for search results
        let mut full_metadata = metadata.clone();
        if let Some(obj) = full_metadata.as_object_mut() {
            obj.insert("text".to_string(), json!(text));
        }

        // 5. Insert into SQLite
        let title = metadata.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        
        self.db.insert_document(id, title, text, &full_metadata, &v_full, &v_short, &v_bit)?;

        // 6. Knowledge Graph Extraction
        if let Some(llm) = &self.llm {
            self.extract_and_store_graph(text, id, llm).await?;
        }

        Ok(id)
    }

    async fn extract_and_store_graph(&self, text: &str, _doc_id: Uuid, llm: &Arc<dyn LLMProvider>) -> Result<()> {
        let prompt = format!(
            "Extract entities and relationships from the following text.\n\
             Return the results in JSON format with two keys: 'entities' and 'relationships'.\n\
             Each entity should have: 'name', 'type', and 'description'.\n\
             Each relationship should have: 'source', 'target', 'predicate', and 'description'.\n\n\
             Text: {}\n\nJSON:",
            text
        );

        let response = llm.complete(&prompt).await?;
        let content = response.content;

        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') { &content[start..=end] } else { &content[start..] }
        } else { &content };

        let graph: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(g) => g,
            Err(_) => return Ok(()),
        };

        if let Some(entities) = graph.get("entities").and_then(|v| v.as_array()) {
            for entity in entities {
                let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let etype = entity.get("type").and_then(|v| v.as_str()).unwrap_or("Concept");
                let desc = entity.get("description").and_then(|v| v.as_str()).unwrap_or("");
                if !name.is_empty() {
                    let _ = self.db.insert_entity(name, etype, desc);
                }
            }
        }

        if let Some(relationships) = graph.get("relationships").and_then(|v| v.as_array()) {
            for rel in relationships {
                let s_name = rel.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let t_name = rel.get("target").and_then(|v| v.as_str()).unwrap_or("");
                let pred = rel.get("predicate").and_then(|v| v.as_str()).unwrap_or("");
                let desc = rel.get("description").and_then(|v| v.as_str()).unwrap_or("");

                let s = self.db.get_entity_by_name(s_name)?;
                let t = self.db.get_entity_by_name(t_name)?;

                if let (Some((s_id, _, _)), Some((t_id, _, _))) = (s, t) {
                    let _ = self.db.insert_relationship(s_id, t_id, pred, desc);
                }
            }
        }

        Ok(())
    }
}
