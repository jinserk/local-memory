use crate::config::Config;
use crate::storage::sqlite::SqliteDatabase;
use crate::engine::vectors::{encode_bq, slice_vector};
use anyhow::Result;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct SearchFunnel<'a> {
    db: &'a SqliteDatabase,
    config: &'a Config,
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
        Self { db, config }
    }

    pub fn search(&self, query_full: &[f32], top_k: usize) -> Result<Vec<FunnelResult>> {
        self.search_with_namespace(query_full, top_k, "default")
    }

    pub fn search_with_namespace(&self, query_full: &[f32], top_k: usize, namespace: &str) -> Result<Vec<FunnelResult>> {
        let query_bit = encode_bq(query_full);
        let s1_candidates = self.db.search_stage1_bit_with_namespace(&query_bit, self.config.stage1_candidates, namespace)?;
        
        if s1_candidates.is_empty() {
            return Ok(vec![]);
        }

        let query_short = slice_vector(query_full, self.db.dimension() / 3);
        let s2_results = self.db.search_stage2_short(&s1_candidates, &query_short, self.config.stage2_candidates)?;

        let query_short = slice_vector(query_full, self.db.dimension() / 3);
        let s2_results = self.db.search_stage2_short(&s1_candidates, &query_short, 20)?;

=======
        let query_short = slice_vector(query_full, 256);
        let s2_results = self.db.search_stage2_short(&s1_candidates, &query_short, self.config.stage2_candidates)?;
>>>>>>> 829edf3 (feat(config): expose funnel stage parameters in config.json)
        let mut results = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        for (id, base_score) in s2_results.into_iter() {
            if let Some((_, metadata)) = self.db.get_document_content(id)? {
                let created_at = metadata.get("created_at").and_then(|v| v.as_u64()).unwrap_or(now);
                let age_days = (now - created_at) as f32 / (24.0 * 3600.0);
                let decay_lambda = 0.01;
                let decay_multiplier = (decay_lambda * age_days).exp();
                let final_score = base_score * decay_multiplier;

                results.push(FunnelResult {
                    id,
                    score: final_score,
                    metadata,
                    context: None,
                });
            }
        }

        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
        Ok(results.into_iter().take(top_k).collect())
    }

    pub fn hybrid_search_with_namespace(&self, query_vector: &[f32], top_k: usize, namespace: &str) -> Result<Vec<FunnelResult>> {
        // 1. Vector Search
        let mut results = self.search_with_namespace(query_vector, top_k, namespace)?;

        // 2. Thematic Fallback
        let is_poor_result = results.is_empty() || results.get(0).map_or(true, |r| r.score > 1.2);
        
        if is_poor_result {
            if let Ok(Some(global_ctx)) = self.search_global_summaries() {
                if !results.is_empty() {
                    results[0].context = Some(json!({"thematic_summary": global_ctx}));
                } else {
                    results.push(FunnelResult {
                        id: Uuid::nil(),
                        score: 1.0,
                        metadata: json!({"text": "No specific documents found, but a thematic summary is available.", "type": "global_insight"}),
                        context: Some(json!({"thematic_summary": global_ctx})),
                    });
                }
            }
        }

        // 3. Enhance with Graph Context
        for res in &mut results {
            if res.id == Uuid::nil() { continue; }
            if let Some(text) = res.metadata.get("text").and_then(|v| v.as_str()) {
                for word in text.split_whitespace() {
                    let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
                    if clean.len() > 3 && clean.chars().next().map_or(false, |c| c.is_uppercase()) {
                        if let Ok(nb) = self.db.get_neighborhood_with_namespace(clean, namespace) {
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

    fn search_global_summaries(&self) -> Result<Option<Value>> {
        let summaries_list = self.db.list_community_summaries(3)?;
        if summaries_list.is_empty() {
            Ok(None)
        } else {
            let summaries: Vec<Value> = summaries_list.into_iter()
                .map(|(_, title, summary)| json!({ "title": title, "summary": summary }))
                .collect();
            Ok(Some(json!(summaries)))
        }
    }
}
