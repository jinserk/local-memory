use crate::mcp::tools::McpContext;
use crate::KnowledgeEvent;
use std::sync::Arc;
use tokio::sync::broadcast;
use serde_json::json;

pub struct CommunityService {
    context: Arc<McpContext>,
}

impl CommunityService {
    pub fn new(context: Arc<McpContext>) -> Self {
        Self { context }
    }

    pub async fn run(self, mut rx: broadcast::Receiver<KnowledgeEvent>) {
        eprintln!("DEBUG: CommunityService starting...");
        
        loop {
            tokio::select! {
                Ok(event) = rx.recv() => {
                    match event {
                        KnowledgeEvent::RelationshipInserted { source_id, .. } => {
                            if let Ok(Some(comm_id)) = self.context.db.get_entity_community_id(source_id) {
                                let _ = self.summarize_community(&comm_id).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn summarize_community(&self, comm_id: &str) -> anyhow::Result<()> {
        let members = self.context.db.list_community_members(comm_id)?;
        if members.is_empty() { return Ok(()); }

        let context_text = members.iter()
            .map(|(name, desc)| format!("- {}: {}", name, desc))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Summarize the following group of related entities into a single thematic summary.\n\
             Provide a title and a concise 2-3 sentence summary.\n\n\
             Entities:\n{}\n\n\
             Return as JSON with 'title' and 'summary' keys.",
            context_text
        );

        let response = self.context.model.complete(&prompt).await?;
        let content = response.content;
        
        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') { &content[start..=end] } else { &content[start..] }
        } else { &content };

        let data: serde_json::Value = serde_json::from_str(json_str).unwrap_or(json!({
            "title": format!("Community {}", comm_id),
            "summary": "Thematic group of related concepts."
        }));

        let title = data.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Cluster");
        let summary = data.get("summary").and_then(|v| v.as_str()).unwrap_or("No summary available.");

        self.context.db.upsert_community(comm_id, title, summary)?;

        Ok(())
    }
}

pub async fn spawn_community_service(context: Arc<McpContext>, rx: broadcast::Receiver<KnowledgeEvent>) {
    let service = CommunityService::new(context);
    tokio::spawn(async move {
        service.run(rx).await;
    });
}
