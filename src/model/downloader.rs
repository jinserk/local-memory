use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Files required for a Hugging Face embedding model
const MODEL_FILES: [&str; 3] = ["config.json", "tokenizer.json", "model.safetensors"];

/// Hugging Face model downloader
pub struct ModelDownloader {
    model_name: String,
    base_url: String,
}

impl ModelDownloader {
    /// Create a new downloader for a Hugging Face model
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
            base_url: format!("https://huggingface.co/{}/resolve/main", model_name),
        }
    }

    /// Check if all required model files exist in the target directory
    pub fn model_exists(target_dir: &Path) -> bool {
        MODEL_FILES
            .iter()
            .all(|file| target_dir.join(file).exists())
    }

    /// Get the list of missing model files
    pub fn missing_files(target_dir: &Path) -> Vec<String> {
        MODEL_FILES
            .iter()
            .filter(|file| !target_dir.join(file).exists())
            .map(|s| s.to_string())
            .collect()
    }

    /// Download all missing model files to the target directory
    pub fn download(&self, target_dir: &Path) -> Result<()> {
        // Create target directory if it doesn't exist
        std::fs::create_dir_all(target_dir)
            .with_context(|| format!("Failed to create directory: {:?}", target_dir))?;

        let missing = Self::missing_files(target_dir);
        if missing.is_empty() {
            eprintln!("All model files already present in {:?}", target_dir);
            return Ok(());
        }

        eprintln!(
            "Downloading model '{}' to {:?}",
            self.model_name, target_dir
        );
        eprintln!("Missing files: {}", missing.join(", "));

        // Create a progress bar for overall progress
        let total_files = MODEL_FILES.len();
        let main_pb = ProgressBar::new(total_files as u64);
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} files")
                .unwrap()
                .progress_chars("=>-"),
        );
        main_pb.set_message("Downloading model files");

        for file in MODEL_FILES.iter() {
            let target_path = target_dir.join(file);
            if target_path.exists() {
                main_pb.inc(1);
                continue;
            }

            self.download_file(file, &target_path)?;
            main_pb.inc(1);
        }

        main_pb.finish_with_message("Download complete");
        eprintln!();

        Ok(())
    }

    /// Download a single file from Hugging Face
    fn download_file(&self, filename: &str, target_path: &Path) -> Result<()> {
        let url = format!("{}/{}", self.base_url, filename);

        // Create a progress bar for this file
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} {spinner}")
                .unwrap(),
        );
        pb.set_message(format!("Downloading {}...", filename));

        // Use blocking reqwest for simplicity (we're in a startup context)
        let response = reqwest::blocking::get(&url)
            .with_context(|| format!("Failed to fetch URL: {}", url))?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to download {}: HTTP {}",
                filename,
                response.status()
            );
        }

        // Get content length for progress bar
        let total_size = response.content_length().unwrap_or(0);
        let file_pb = ProgressBar::new(total_size);
        file_pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})")
                .unwrap()
                .progress_chars("=>-"),
        );
        file_pb.set_message(format!("  {}", filename));

        // Create the file and write content
        let mut file = File::create(target_path)
            .with_context(|| format!("Failed to create file: {:?}", target_path))?;

        for chunk in response.bytes()?.chunks(8192) {
            file.write_all(&chunk)
                .with_context(|| format!("Failed to write to file: {:?}", target_path))?;
            file_pb.inc(chunk.len() as u64);
        }

        file_pb.finish_with_message(format!("  {} ✓", filename));
        pb.finish();

        Ok(())
    }
}

/// Ensure model files are available, downloading if necessary
pub fn ensure_model_files(model_name: &str, model_path: &Path, auto_download: bool) -> Result<()> {
    if ModelDownloader::model_exists(model_path) {
        return Ok(());
    }

    if !auto_download {
        anyhow::bail!(
            "Model files not found in {:?} and auto_download is disabled. \
             Please download manually or set auto_download: true in config.",
            model_path
        );
    }

    eprintln!("Model files not found. Starting download...");
    let downloader = ModelDownloader::new(model_name);
    downloader.download(model_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_missing_files_empty_dir() {
        let dir = tempdir().unwrap();
        let missing = ModelDownloader::missing_files(dir.path());
        assert_eq!(missing.len(), 3);
        assert!(missing.contains(&"config.json".to_string()));
        assert!(missing.contains(&"tokenizer.json".to_string()));
        assert!(missing.contains(&"model.safetensors".to_string()));
    }

    #[test]
    fn test_missing_files_partial() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), "{}").unwrap();
        let missing = ModelDownloader::missing_files(dir.path());
        assert_eq!(missing.len(), 2);
        assert!(!missing.contains(&"config.json".to_string()));
    }

    #[test]
    fn test_model_exists_false() {
        let dir = tempdir().unwrap();
        assert!(!ModelDownloader::model_exists(dir.path()));
    }

    #[test]
    fn test_model_exists_true() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), "{}").unwrap();
        std::fs::write(dir.path().join("tokenizer.json"), "{}").unwrap();
        std::fs::write(dir.path().join("model.safetensors"), "data").unwrap();
        assert!(ModelDownloader::model_exists(dir.path()));
    }
}
