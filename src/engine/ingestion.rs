use crate::model::nomic::Embedder;
use crate::storage::sqlite::SqliteDatabase;
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;
use edgequake_llm::LLMProvider;
use serde_json::json;

pub struct IngestionPipeline {
    model: Arc<dyn Embedder + Send + Sync>,
    db: Arc<SqliteDatabase>,
    llm: Option<Arc<dyn LLMProvider + Send + Sync>>,
}

impl IngestionPipeline {
    pub fn new(
        model: Arc<dyn Embedder + Send + Sync>, 
        db: Arc<SqliteDatabase>,
        llm: Option<Arc<dyn LLMProvider + Send + Sync>>
    ) -> Self {
        Self { model, db, llm }
    }

    pub async fn run(&self, text: &str, metadata: serde_json::Value) -> Result<Uuid> {
        let id = Uuid::new_v4();

        // 1. Generate embedding for the full text
        let vector = self.model.encode(text)?;

        // 2. Insert document into SQLite
        let title = metadata.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        
        self.db.insert_document(id, title, text, &metadata, &vector)?;

        // 3. Knowledge Graph Extraction (if LLM is available)
        if let Some(llm) = &self.llm {
            self.extract_and_store_graph(text, id, llm).await?;
        }

        Ok(id)
    }

    async fn extract_and_store_graph(&self, text: &str, _doc_id: Uuid, llm: &Arc<dyn LLMProvider + Send + Sync>) -> Result<()> {
        let prompt = format!(
            "Extract entities and relationships from the following text.\n\
             Return the results in JSON format with two keys: 'entities' and 'relationships'.\n\
             Each entity should have: 'name', 'type' (Person, Organization, Location, Concept, Event, Technology, Product), and 'description'.\n\
             Each relationship should have: 'source', 'target', 'predicate', and 'description'.\n\n\
             Text: {}\n\nJSON:",
            text
        );

        let response = llm.complete(&prompt).await?;
        let content = response.content;

        // Try to parse the JSON from the LLM response
        // LLMs often wrap JSON in code blocks
        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                &content[start..=end]
            } else {
                &content[start..]
            }
        } else {
            &content
        };

        let graph: serde_json::Value = serde_json::from_str(json_str)?;

        // Store entities
        if let Some(entities) = graph.get("entities").and_then(|v| v.as_array()) {
            for entity in entities {
                let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let entity_type = entity.get("type").and_then(|v| v.as_str()).unwrap_or("Concept");
                let description = entity.get("description").and_then(|v| v.as_str()).unwrap_or("");
                
                // We could also generate embeddings for entities here if needed
                self.db.insert_entity(name, entity_type, description, &json!({}), None)?;
            }
        }

        // Store relationships
        if let Some(relationships) = graph.get("relationships").and_then(|v| v.as_array()) {
            for rel in relationships {
                let source_name = rel.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let target_name = rel.get("target").and_then(|v| v.as_str()).unwrap_or("");
                let predicate = rel.get("predicate").and_then(|v| v.as_str()).unwrap_or("");
                let description = rel.get("description").and_then(|v| v.as_str()).unwrap_or("");

                let source = self.db.get_entity_by_name(source_name)?;
                let target = self.db.get_entity_by_name(target_name)?;

                if let (Some((source_id, _, _)), Some((target_id, _, _))) = (source, target) {
                    self.db.insert_relationship(source_id, target_id, predicate, description, &json!({}))?;
                }
            }
        }

        Ok(())
    }
}
