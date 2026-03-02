use anyhow::Result;

/// Internal helper to pull an Ollama model from a local host.
pub async fn pull_ollama_model(host: &str, model_name: &str) -> Result<()> {
    eprintln!("Pulling Ollama model '{}' from {}...", model_name, host);
    
    let client = reqwest::Client::new();
    let url = format!("{}/api/pull", host);
    
    let response = client.post(&url)
        .json(&serde_json::json!({
            "name": model_name,
            "stream": false
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to pull Ollama model: HTTP {}", response.status());
    }

    eprintln!("  ✓ Ollama model '{}' is ready", model_name);
    Ok(())
}
