/// Unit tests for `loader.rs` helpers (file-system utilities, no network).
use local_memory::model::candle::{get_model_dir, pub_test_missing_files, pub_test_model_exists};
use std::fs;
use tempfile::tempdir;

// ── get_model_dir ─────────────────────────────────────────────────────────────

#[test]
fn test_get_model_dir_replaces_slash_with_double_underscore() {
    let base = std::path::Path::new("/tmp/models");
    let dir = get_model_dir(base, "nomic-ai/nomic-embed-text-v1.5");
    assert_eq!(
        dir,
        std::path::PathBuf::from("/tmp/models/nomic-ai__nomic-embed-text-v1.5")
    );
}

#[test]
fn test_get_model_dir_no_slash_is_unchanged() {
    let base = std::path::Path::new("/tmp/models");
    let dir = get_model_dir(base, "my-local-model");
    assert_eq!(dir, std::path::PathBuf::from("/tmp/models/my-local-model"));
}

#[test]
fn test_get_model_dir_multiple_slashes() {
    let base = std::path::Path::new("/tmp/models");
    let dir = get_model_dir(base, "org/sub/model");
    // Each '/' becomes '__'.
    assert_eq!(dir, std::path::PathBuf::from("/tmp/models/org__sub__model"));
}

// ── pub_test_model_exists ─────────────────────────────────────────────────────

#[test]
fn test_model_exists_returns_false_for_empty_dir() {
    let dir = tempdir().unwrap();
    assert!(!pub_test_model_exists(dir.path()));
}

#[test]
fn test_model_exists_returns_false_when_only_some_files_present() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("config.json"), b"{}").unwrap();
    fs::write(dir.path().join("tokenizer.json"), b"{}").unwrap();
    // model.safetensors is missing.
    assert!(!pub_test_model_exists(dir.path()));
}

#[test]
fn test_model_exists_returns_true_when_all_files_present() {
    let dir = tempdir().unwrap();
    for name in ["config.json", "tokenizer.json", "model.safetensors"] {
        fs::write(dir.path().join(name), b"placeholder").unwrap();
    }
    assert!(pub_test_model_exists(dir.path()));
}

// ── pub_test_missing_files ────────────────────────────────────────────────────

#[test]
fn test_missing_files_empty_dir_returns_all_three() {
    let dir = tempdir().unwrap();
    let missing = pub_test_missing_files(dir.path());
    assert_eq!(missing.len(), 3);
    assert!(missing.contains(&"config.json".to_string()));
    assert!(missing.contains(&"tokenizer.json".to_string()));
    assert!(missing.contains(&"model.safetensors".to_string()));
}

#[test]
fn test_missing_files_partial_set() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("config.json"), b"{}").unwrap();
    let missing = pub_test_missing_files(dir.path());
    assert_eq!(missing.len(), 2);
    assert!(!missing.contains(&"config.json".to_string()));
    assert!(missing.contains(&"tokenizer.json".to_string()));
    assert!(missing.contains(&"model.safetensors".to_string()));
}

#[test]
fn test_missing_files_returns_empty_when_complete() {
    let dir = tempdir().unwrap();
    for name in ["config.json", "tokenizer.json", "model.safetensors"] {
        fs::write(dir.path().join(name), b"placeholder").unwrap();
    }
    let missing = pub_test_missing_files(dir.path());
    assert!(missing.is_empty());
}
