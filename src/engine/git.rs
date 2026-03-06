use crate::mcp::tools::McpContext;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::json;

pub async fn spawn_git_observer(context: Arc<McpContext>) {
    let mut last_commit = String::new();

    tokio::spawn(async move {
        loop {
            if let Ok(current_commit) = get_latest_commit() {
                if current_commit != last_commit && !current_commit.is_empty() {
                    eprintln!("[git] New commit detected: {}", current_commit);
                    
                    if let Ok(summary) = get_commit_summary(&current_commit) {
                        // Truncate to first 500 chars to avoid hogging the LLM for huge diffs.
                        let summary_truncated = if summary.len() > 500 { &summary[..500] } else { &summary };
                        let _ = context.get_pipeline().run_with_namespace(
                            &format!("GIT COMMIT {}: {}", current_commit, summary_truncated),
                            json!({"type": "git_commit", "hash": current_commit}),
                            "git"
                        ).await;
                    }
                    
                    last_commit = current_commit;
                }
            }
            sleep(Duration::from_secs(60)).await;
        }
    });
}

fn get_latest_commit() -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    } else {
        anyhow::bail!("Not a git repo or git not found")
    }
}

fn get_commit_summary(hash: &str) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["show", "--summary", hash])
        .output()?;
    
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    } else {
        anyhow::bail!("Failed to get commit summary")
    }
}
