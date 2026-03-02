use local_memory::model::candle::{pub_test_model_exists, pub_test_missing_files};
use tempfile::tempdir;

#[test]
fn test_missing_files_empty_dir() {
    let dir = tempdir().unwrap();
    let missing = pub_test_missing_files(dir.path());
    assert_eq!(missing.len(), 3);
    assert!(missing.contains(&"config.json".to_string()));
    assert!(missing.contains(&"tokenizer.json".to_string()));
    assert!(missing.contains(&"model.safetensors".to_string()));
}

#[test]
fn test_model_exists_false() {
    let dir = tempdir().unwrap();
    assert!(!pub_test_model_exists(dir.path()));
}
