use local_memory::config::{Config, ExtractorProvider};
use std::path::PathBuf;

#[test]
fn test_default_paths() {
    let config = Config::default();
    assert_eq!(config.storage_path, PathBuf::from(".local-memory/storage"));
    assert_eq!(config.model_path, PathBuf::from(".local-memory/models"));
}

#[test]
fn test_config_loading_with_alias() {
    // Test that the "embedding" field loads correctly
    let json = r#"{
        "embedding": {
            "name": "custom-model",
            "auto_download": false
        }
    }"#;
    
    let config: Config = serde_json::from_str(json).unwrap();
    assert_eq!(config.embedding.name, "custom-model");
    assert_eq!(config.embedding.auto_download, false);
}

#[test]
fn test_config_loading_with_extractor_aligned() {
    let json = r#"{
        "llm_extractor": {
            "provider": "huggingface",
            "name": "phi-3-mini-4k-instruct",
            "auto_download": true
        }
    }"#;
    
    let config: Config = serde_json::from_str(json).unwrap();
    let ext = config.llm_extractor.unwrap();
    assert_eq!(ext.provider, ExtractorProvider::HuggingFace);
    assert_eq!(ext.name, "phi-3-mini-4k-instruct");
    assert_eq!(ext.auto_download, true);
}
