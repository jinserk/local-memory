use crate::mcp::tools::McpContext;
use std::sync::Arc;
use tokio::time::{self, Duration};

pub async fn spawn_decay_service(context: Arc<McpContext>) {
    tokio::spawn(async move {
        // Run decay process every 24 hours
        let mut interval = time::interval(Duration::from_secs(24 * 3600));
        
        // Skip the first immediate tick to avoid running it exactly at startup 
        // if we want, or just let it run once.
        // Actually, running once at startup is fine to prune old ones.
        
        loop {
            interval.tick().await;
            eprintln!("[decay] Running entity decay process...");
            if let Err(e) = context.db.process_decay() {
                eprintln!("[decay] Error during decay process: {}", e);
            } else {
                eprintln!("[decay] Entity decay process completed.");
            }
        }
    });
}
