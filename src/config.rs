use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchStages {
    pub stage1_k: usize,
    pub stage2_k: usize,
}

impl Default for SearchStages {
    fn default() -> Self {
        Self {
            stage1_k: 100,
            stage2_k: 10,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    pub storage_path: PathBuf,
    pub model_path: PathBuf,
    pub search_stages: SearchStages,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("storage"),
            model_path: PathBuf::from("models"),
            search_stages: SearchStages::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = env::var("LOCAL_MEMORY_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("config.json"));

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }

        Config::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.storage_path, PathBuf::from("storage"));
        assert_eq!(config.model_path, PathBuf::from("models"));
        assert_eq!(config.search_stages.stage1_k, 100);
        assert_eq!(config.search_stages.stage2_k, 10);
    }

    #[test]
    fn test_load_missing_file_uses_defaults() {

        unsafe {
            env::set_var(
                "LOCAL_MEMORY_CONFIG",
                "definitely_not_a_real_config_file_12345.json",
            );
        }
        let config = Config::load();
        assert_eq!(config, Config::default());
        unsafe { env::remove_var("LOCAL_MEMORY_CONFIG") };
    }

    #[test]
    fn test_load_valid_file() {
        let temp_config = Config {
            storage_path: PathBuf::from("custom_storage"),
            model_path: PathBuf::from("custom_models"),
            search_stages: SearchStages {
                stage1_k: 50,
                stage2_k: 5,
            },
        };
        let content = serde_json::to_string(&temp_config).unwrap();
        let path = "test_config.json";
        fs::write(path, content).unwrap();

        unsafe { env::set_var("LOCAL_MEMORY_CONFIG", path) };
        let loaded_config = Config::load();
        assert_eq!(loaded_config, temp_config);

        fs::remove_file(path).unwrap();
        unsafe { env::remove_var("LOCAL_MEMORY_CONFIG") };
    }
}
