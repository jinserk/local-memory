use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;
use indicatif::{ProgressBar, ProgressStyle};
use edgequake_llm::LLMProvider;

pub const MODEL_FILES: [&str; 3] = ["config.json", "tokenizer.json", "model.safetensors"];

pub fn get_model_dir(base_path: &Path, model_name: &str) -> PathBuf {
    let safe_name = model_name.replace("/", "__");
    base_path.join(safe_name)
}

pub fn ensure_model_files(model_name: &str, base_path: &Path, auto_download: bool) -> Result<PathBuf> {
    let model_dir = get_model_dir(base_path, model_name);
    let missing = MODEL_FILES.iter().filter(|f| !model_dir.join(f).exists()).count();
    
    if missing == 0 {
        return Ok(model_dir);
    }

    if !auto_download {
        anyhow::bail!("Model files missing in {:?}", model_dir);
    }

    eprintln!("Downloading model '{}'...", model_name);
    std::fs::create_dir_all(&model_dir)?;

    let base_url = format!("https://huggingface.co/{}/resolve/main", model_name);
    let pb = ProgressBar::new(MODEL_FILES.len() as u64);
    pb.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40.cyan/blue}] {pos}/{len}").unwrap());

    for filename in MODEL_FILES.iter() {
        let url = format!("{}/{}", base_url, filename);
        let target_path = model_dir.join(filename);
        
        let response = reqwest::blocking::get(&url)?;
        if !response.status().is_success() {
            anyhow::bail!("Failed to download {}: {}", filename, response.status());
        }

        let mut file = File::create(target_path)?;
        let content = response.bytes()?;
        file.write_all(&content)?;
        pb.inc(1);
    }
    pb.finish_with_message("Download complete");
    Ok(model_dir)
}

pub async fn check_and_pull_llm(provider: &dyn LLMProvider) -> Result<()> {
    if provider.name() == "huggingface" { return Ok(()); }
    provider.complete("ping").await.map(|_| ()).map_err(|e| anyhow::anyhow!("LLM check failed: {}", e))
}
