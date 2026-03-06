use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;

const MODEL_FILES: [&str; 3] = ["config.json", "tokenizer.json", "model.safetensors"];

/// Returns the local directory path where a model's files are stored.
pub fn get_model_dir(base_path: &Path, model_name: &str) -> PathBuf {
    let safe_name = model_name.replace('/', "__");
    base_path.join(safe_name)
}

/// Ensures all required model files are present. Downloads them from
/// HuggingFace Hub if `auto_download` is true and any file is missing.
pub async fn ensure_model_files(
    model_name: &str,
    base_path: &Path,
    auto_download: bool,
) -> Result<PathBuf> {
    let model_dir = get_model_dir(base_path, model_name);
    let is_complete = MODEL_FILES.iter().all(|f| {
        let p = model_dir.join(f);
        p.exists() && p.metadata().map(|m| m.len() > 0).unwrap_or(false)
    });
    if is_complete {
        return Ok(model_dir);
    }
    if !auto_download {
        anyhow::bail!("Model files missing or corrupt in {:?}", model_dir);
    }
    eprintln!("Downloading model '{}'...", model_name);
    std::fs::create_dir_all(&model_dir)?;
    let client = reqwest::Client::new();
    let base_url = format!("https://huggingface.co/{}/resolve/main", model_name);
    let pb = ProgressBar::new(MODEL_FILES.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap(),
    );
    for filename in MODEL_FILES.iter() {
        let url = format!("{}/{}", base_url, filename);
        let target_path = model_dir.join(filename);
        let response = client.get(&url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to download {}: {}", filename, response.status());
        }
        let mut file = File::create(target_path)?;
        let mut stream = response.bytes_stream();
        while let Some(item) = stream.next().await {
            let chunk = item?;
            file.write_all(&chunk)?;
        }
        pb.inc(1);
    }
    pb.finish_with_message("Download complete");
    Ok(model_dir)
}

/// Test helper: returns true when all required model files exist.
pub fn pub_test_model_exists(path: &Path) -> bool {
    MODEL_FILES.iter().all(|f| path.join(f).exists())
}

/// Test helper: returns the names of any missing model files.
pub fn pub_test_missing_files(path: &Path) -> Vec<String> {
    MODEL_FILES
        .iter()
        .filter(|f| !path.join(f).exists())
        .map(|s| s.to_string())
        .collect()
}
