use crate::config::Config;
use crate::storage::sqlite::SqliteDatabase;
use crate::engine::vectors::{encode_bq, slice_vector};
use anyhow::Result;
use serde_json::Value;
use uuid::Uuid;

pub struct SearchFunnel<'a> {
    db: &'a SqliteDatabase,
    _config: &'a Config,
}

#[derive(Debug, Clone)]
pub struct FunnelResult {
    pub id: Uuid,
    pub score: f32,
    pub metadata: Value,
    pub context: Option<Value>,
}

impl<'a> SearchFunnel<'a> {
    pub fn new_sqlite(db: &'a SqliteDatabase, config: &'a Config) -> Self {
        Self { db, _config: config }
    }

    /// Full 3-stage funnel search
    pub fn search(&self, query_full: &[f32], top_k: usize) -> Result<Vec<FunnelResult>> {
        // Stage 1: BQ (768-bit)
        let query_bit = encode_bq(query_full);
        let s1_candidates = self.db.search_stage1_bit(&query_bit, 100)?;
        
        eprintln!("DEBUG: Stage 1 candidates: {}", s1_candidates.len());

        if s1_candidates.is_empty() {
            return Ok(vec![]);
        }

        // Stage 2: Matryoshka (256d)
        let query_short = slice_vector(query_full, 256);
        let s2_results = self.db.search_stage2_short(&s1_candidates, &query_short, 20)?;

        eprintln!("DEBUG: Stage 2 results: {}", s2_results.len());

        // Stage 3: Map to final format
        let mut results = Vec::new();
        for (id, score) in s2_results.into_iter().take(top_k) {
            if let Some((_, metadata)) = self.db.get_document_content(id)? {
                results.push(FunnelResult {
                    id,
                    score,
                    metadata,
                    context: None,
                });
            }
        }

        Ok(results)
    }

    pub fn hybrid_search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<FunnelResult>> {
        // 1. Vector Search (using Funnel)
        let mut results = self.search(query_vector, top_k)?;

        // 2. Enhance with Graph Context (EdgeQuake logic)
        for res in &mut results {
            if let Some(text) = res.metadata.get("text").and_then(|v| v.as_str()) {
                for word in text.split_whitespace() {
                    let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
                    if clean.len() > 3 && clean.chars().next().unwrap().is_uppercase() {
                        if let Ok(nb) = self.db.get_neighborhood(clean) {
                            if nb.get("error").is_none() {
                                res.context = Some(nb);
                                break; 
                            }
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
}
