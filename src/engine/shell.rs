use crate::mcp::tools::McpContext;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use serde_json::json;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::fs::File;

pub async fn spawn_shell_observer(context: Arc<McpContext>) {
    let mut last_processed_line = 0;

    tokio::spawn(async move {
        loop {
            if let Some(history_path) = get_history_path() {
                if let Ok(file) = File::open(history_path) {
                    let reader = BufReader::new(file);
                    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
                    
                    if lines.len() > last_processed_line {
                        for line in &lines[last_processed_line..] {
                            // Filter for "interesting" commands (build, test, deploy, git, etc.)
                            if is_interesting_command(line) {
                                let _ = context.get_pipeline().run_with_namespace(
                                    &format!("SHELL COMMAND: {}", line),
                                    json!({"type": "shell_command", "cmd": line}),
                                    "shell"
                                ).await;
                            }
                        }
                        last_processed_line = lines.len();
                    }
                }
            }
            sleep(Duration::from_secs(30)).await;
        }
    });
}

fn get_history_path() -> Option<PathBuf> {
    let home = home::home_dir()?;
    // Try .zsh_history first, then .bash_history
    let zsh = home.join(".zsh_history");
    if zsh.exists() { return Some(zsh); }
    let bash = home.join(".bash_history");
    if bash.exists() { return Some(bash); }
    None
}

fn is_interesting_command(cmd: &str) -> bool {
    let cmd = cmd.to_lowercase();
    let keywords = ["cargo", "npm", "docker", "git", "kubectl", "python", "deploy", "build", "test"];
    keywords.iter().any(|k| cmd.contains(k))
}
