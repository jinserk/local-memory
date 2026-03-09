use crate::storage::sqlite::SqliteDatabase;
use crate::engine::vectors::{encode_bq, slice_vector};
use crate::KnowledgeEvent;
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;
use edgequake_llm::{LLMProvider, EmbeddingProvider};
use serde_json::json;
use std::path::Path;
use tokio::sync::broadcast;

pub struct IngestionPipeline {
    embedder: Arc<dyn EmbeddingProvider>,
    db: Arc<SqliteDatabase>,
    llm: Option<Arc<dyn LLMProvider>>,
    semantic_chunking: bool,
    event_tx: Option<broadcast::Sender<KnowledgeEvent>>,
}

impl IngestionPipeline {
    pub fn new(
        embedder: Arc<dyn EmbeddingProvider>, 
        db: Arc<SqliteDatabase>,
        llm: Option<Arc<dyn LLMProvider>>,
        semantic_chunking: bool,
        event_tx: Option<broadcast::Sender<KnowledgeEvent>>
    ) -> Self {
        Self { embedder, db, llm, semantic_chunking, event_tx }
    }

    pub async fn run(&self, text: &str, metadata: serde_json::Value) -> Result<Uuid> {
        self.run_with_namespace(text, metadata, "default").await
    }

    pub async fn run_auto(&self, input: &str, metadata: serde_json::Value, namespace: &str) -> Result<Uuid> {
        let path = Path::new(input);
        if path.exists() {
            return self.run_file(path, metadata, namespace).await;
        }
        self.run_with_namespace(input, metadata, namespace).await
    }

    pub async fn run_file(&self, path: &Path, metadata: serde_json::Value, namespace: &str) -> Result<Uuid> {
        eprintln!("DEBUG: [run_file] path={:?}, namespace={}", path, namespace);
        
        let content = if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            if let Some(llm) = &self.llm {
                eprintln!("DEBUG: [run_file] PDF conversion starting with provider: {} ({})", llm.name(), llm.model());
                let config = edgequake_pdf2md::ConversionConfig::builder()
                    .provider(llm.clone())
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build PDF config: {}", e))?;
                
                let output = edgequake_pdf2md::convert(path.to_string_lossy().as_ref(), &config).await
                    .map_err(|e| anyhow::anyhow!("PDF conversion failed for {:?}: {}", path, e))?;
                eprintln!("DEBUG: [run_file] PDF conversion complete. Pages processed: {}", output.stats.total_pages);
                output.markdown
            } else {
                anyhow::bail!("LLM required for PDF extraction via edgequake-pdf2md");
            }
        } else {
            // Fallback for non-PDF files: try reading as plain text
            std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Failed to read file as text {:?}: {}", path, e))?
        };

        eprintln!("DEBUG: Document content extracted ({} chars)", content.len());

        let mut file_metadata = metadata.clone();
        if let Some(obj) = file_metadata.as_object_mut() {
            obj.insert("source_file".to_string(), json!(path.to_string_lossy()));
            if !obj.contains_key("title") {
                obj.insert("title".to_string(), json!(path.file_name().unwrap_or_default().to_string_lossy()));
            }
        }

        self.run_with_namespace(&content, file_metadata, namespace).await
    }

    pub async fn run_image(&self, path: &Path, metadata: serde_json::Value, namespace: &str) -> Result<Uuid> {
        self.run_file(path, metadata, namespace).await
    }

    pub async fn run_with_namespace(&self, text: &str, metadata: serde_json::Value, namespace: &str) -> Result<Uuid> {
        if text.contains("---CHUNK---") {
            let chunks: Vec<&str> = text.split("---CHUNK---").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let parent_id = Uuid::new_v4();
            for chunk in chunks {
                let mut chunk_meta = metadata.clone();
                if let Some(obj) = chunk_meta.as_object_mut() {
                    obj.insert("parent_id".to_string(), json!(parent_id.to_string()));
                }
                self.process_chunk(chunk, chunk_meta, namespace).await?;
            }
            return Ok(parent_id);
        }

        if self.semantic_chunking && self.llm.is_some() {
            return self.run_semantic(text, metadata, namespace).await;
        }

        self.process_chunk(text, metadata, namespace).await
    }

    async fn process_chunk(&self, text: &str, metadata: serde_json::Value, namespace: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        eprintln!("DEBUG: Embedding chunk (len={})...", text.len());
        let v_full = self.embedder.embed_one(text).await
            .map_err(|e| anyhow::anyhow!("Embedding failed: {}", e))?;
        
        let v_short = slice_vector(&v_full, self.db.dimension() / 3);
        let v_bit = encode_bq(&v_full);

        let mut full_metadata = metadata.clone();
        if let Some(obj) = full_metadata.as_object_mut() {
            obj.insert("text".to_string(), json!(text));
            obj.insert("created_at".to_string(), json!(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()));
        }

        let title = metadata.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                if text.len() > 50 { &text[..50] } else { text }
            });
        
        self.db.insert_document_with_namespace(id, title, text, &full_metadata, &v_full, &v_short, &v_bit, namespace)?;

        // Emit Event
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(KnowledgeEvent::DocumentInserted { 
                id, 
                title: title.to_string(), 
                namespace: namespace.to_string() 
            });
        }

        if let Some(llm) = &self.llm {
            eprintln!("DEBUG: Extracting Knowledge Graph from chunk...");
            self.extract_and_store_graph(text, id, llm, namespace).await?;
            eprintln!("DEBUG: KG Extraction complete.");
        }
        Ok(id)
    }

    async fn run_semantic(&self, text: &str, metadata: serde_json::Value, namespace: &str) -> Result<Uuid> {
        let llm = self.llm.as_ref().unwrap();
        let prompt = format!(
            "Divide the following text into logical semantic chunks.\n\
             Return each chunk separated by '---CHUNK---'.\n\n\
             Text: {}",
            text
        );
        let response = llm.complete(&prompt).await?;
        let chunks: Vec<&str> = response.content.split("---CHUNK---").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        let parent_id = Uuid::new_v4();
        
        let summary_prompt = format!("Provide a concise one-sentence summary of the following text:\n{}", text);
        let summary_resp = llm.complete(&summary_prompt).await?;
        let parent_summary = summary_resp.content.trim();

        for chunk in chunks {
            let mut chunk_meta = metadata.clone();
            if let Some(obj) = chunk_meta.as_object_mut() {
                obj.insert("parent_id".to_string(), json!(parent_id.to_string()));
                obj.insert("parent_summary".to_string(), json!(parent_summary));
            }
            self.process_chunk(chunk, chunk_meta, namespace).await?;
        }
        Ok(parent_id)
    }

    async fn extract_and_store_graph(&self, text: &str, _doc_id: Uuid, llm: &Arc<dyn LLMProvider>, namespace: &str) -> Result<()> {
        let mut existing_context = String::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            if clean.len() > 2 && clean.chars().next().is_some_and(|c| c.is_uppercase())
                && let Ok(Some((_, etype, desc))) = self.db.get_entity_by_name_with_namespace(clean, namespace) {
                    existing_context.push_str(&format!("- {} ({}): {}\n", clean, etype, desc));
                }
        }

        let context_prompt = if existing_context.is_empty() { "".to_string() } else { format!("\nEXISTING KNOWLEDGE:\n{}\n", existing_context) };

        let prompt = format!(
            "Extract entities and relationships from the following text.\n\
             Return the results in JSON format with three keys: 'entities', 'relationships', and 'conflicts'.\n\
             Each entity should have: 'name', 'type', and 'description'.\n\
             Each relationship should have: 'source', 'target', 'predicate', and 'description'.\n\n\
             KNOWLEDGE EVOLUTION:\n\
             - If a fact in the text updates, extends, or supersedes existing knowledge, use predicates like 'UPDATES', 'EXTENDS', or 'SUPERSEDES'.\n\
             - CONFLICT DETECTION: If the text directly CONTRADICTS existing knowledge provided below, list the conflict details in the 'conflicts' key.\n\
             {}\n\
             Text: {}\n\nJSON:",
            context_prompt,
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

        if let Some(conflicts) = graph.get("conflicts").and_then(|v| v.as_array()) {
            for conflict in conflicts { eprintln!("CONFLICT DETECTED: {}", conflict); }
        }

        if let Some(entities) = graph.get("entities").and_then(|v| v.as_array()) {
            for entity in entities {
                let name = entity.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let etype = entity.get("type").and_then(|v| v.as_str()).unwrap_or("Concept");
                let desc = entity.get("description").and_then(|v| v.as_str()).unwrap_or("");
                if !name.is_empty()
                    && let Ok(entity_id) = self.db.insert_entity_with_namespace(name, etype, desc, namespace) {
                        // Emit Event
                        if let Some(tx) = &self.event_tx {
                            let _ = tx.send(KnowledgeEvent::EntityInserted { 
                                id: entity_id, 
                                name: name.to_string(), 
                                namespace: namespace.to_string() 
                            });
                        }
                    }
            }
        }

        if let Some(relationships) = graph.get("relationships").and_then(|v| v.as_array()) {
            for rel in relationships {
                let s_name = rel.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let t_name = rel.get("target").and_then(|v| v.as_str()).unwrap_or("");
                let pred = rel.get("predicate").and_then(|v| v.as_str()).unwrap_or("");
                let desc = rel.get("description").and_then(|v| v.as_str()).unwrap_or("");
                
                if s_name.is_empty() || t_name.is_empty() { continue; }

                // Auto-upsert entities referenced in relationships that the LLM omitted from 'entities'.
                let s_id = self.db.get_entity_by_name_with_namespace(s_name, namespace)?
                    .map(|(id, _, _)| id)
                    .or_else(|| self.db.insert_entity_with_namespace(s_name, "Concept", desc, namespace).ok());
                let t_id = self.db.get_entity_by_name_with_namespace(t_name, namespace)?
                    .map(|(id, _, _)| id)
                    .or_else(|| self.db.insert_entity_with_namespace(t_name, "Concept", desc, namespace).ok());
                
                if let (Some(s_id), Some(t_id)) = (s_id, t_id)
                    && self.db.insert_relationship(s_id, t_id, pred, desc).is_ok() {
                        if let Some(tx) = &self.event_tx {
                            let _ = tx.send(KnowledgeEvent::RelationshipInserted {
                                source_id: s_id,
                                target_id: t_id,
                                predicate: pred.to_string()
                            });
                        }
                    }
            }
        }
        Ok(())
    }
}
