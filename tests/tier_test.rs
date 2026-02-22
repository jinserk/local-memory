use anyhow::Result;
use local_memory::storage::db::{Database, Memory};
use local_memory::storage::tier::{
    current_timestamp, duration_to_expiration, is_expired, MemoryTier,
};
use local_memory::storage::TierConfig;
use serde_json::json;
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn test_create_episodic_memory() -> Result<()> {
    let dir = tempdir()?;
    let db = Database::open(dir.path())?;

    let id = Uuid::new_v4();
    let memory = Memory {
        id,
        metadata: json!({"text": "temporary memory"}),
        vector: vec![1.0, 2.0, 3.0],
        bit_vector: vec![0b10101010],
        tier: MemoryTier::Episodic,
        expires_at: Some(current_timestamp() + 3600),
    };

    db.insert_memory(&memory)?;

    let retrieved = db.get_memory(id)?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.tier, MemoryTier::Episodic);
    assert!(retrieved.expires_at.is_some());

    Ok(())
}

#[test]
fn test_create_semantic_memory() -> Result<()> {
    let dir = tempdir()?;
    let db = Database::open(dir.path())?;

    let id = Uuid::new_v4();
    let memory = Memory {
        id,
        metadata: json!({"text": "permanent knowledge"}),
        vector: vec![1.0, 2.0, 3.0],
        bit_vector: vec![0b10101010],
        tier: MemoryTier::Semantic,
        expires_at: None,
    };

    db.insert_memory(&memory)?;

    let retrieved = db.get_memory(id)?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.tier, MemoryTier::Semantic);
    assert!(retrieved.expires_at.is_none());

    Ok(())
}

#[test]
fn test_expired_episodic_memory() -> Result<()> {
    let dir = tempdir()?;
    let db = Database::open(dir.path())?;

    let id = Uuid::new_v4();
    let past_timestamp = current_timestamp() - 1000;
    let memory = Memory {
        id,
        metadata: json!({"text": "expired memory"}),
        vector: vec![1.0, 2.0, 3.0],
        bit_vector: vec![0b10101010],
        tier: MemoryTier::Episodic,
        expires_at: Some(past_timestamp),
    };

    db.insert_memory(&memory)?;

    let retrieved = db.get_memory(id)?;
    assert!(retrieved.is_none(), "Expired memory should return None");

    Ok(())
}

#[test]
fn test_not_expired_episodic_memory() -> Result<()> {
    let dir = tempdir()?;
    let db = Database::open(dir.path())?;

    let id = Uuid::new_v4();
    let future_timestamp = current_timestamp() + 3600;
    let memory = Memory {
        id,
        metadata: json!({"text": "valid memory"}),
        vector: vec![1.0, 2.0, 3.0],
        bit_vector: vec![0b10101010],
        tier: MemoryTier::Episodic,
        expires_at: Some(future_timestamp),
    };

    db.insert_memory(&memory)?;

    let retrieved = db.get_memory(id)?;
    assert!(
        retrieved.is_some(),
        "Non-expired memory should be retrievable"
    );

    Ok(())
}

#[test]
fn test_tier_config_default() {
    let config = TierConfig::default();
    assert_eq!(config.default_tier, MemoryTier::Semantic);
    assert_eq!(config.default_episodic_ttl_seconds, Some(3600));
}

#[test]
fn test_duration_to_expiration() {
    let duration = Duration::from_secs(60);
    let expiration = duration_to_expiration(duration);
    let now = current_timestamp();
    assert!(expiration >= now + 60 && expiration <= now + 61);
}

#[test]
fn test_is_expired_helper() {
    assert!(!is_expired(None));

    let past = current_timestamp() - 1000;
    assert!(is_expired(Some(past)));

    let future = current_timestamp() + 1000;
    assert!(!is_expired(Some(future)));
}

#[test]
fn test_tier_serialization_roundtrip() -> Result<()> {
    let tier = MemoryTier::Episodic;
    let json = serde_json::to_string(&tier)?;
    let deserialized: MemoryTier = serde_json::from_str(&json)?;
    assert_eq!(tier, deserialized);

    let tier = MemoryTier::Semantic;
    let json = serde_json::to_string(&tier)?;
    let deserialized: MemoryTier = serde_json::from_str(&json)?;
    assert_eq!(tier, deserialized);

    Ok(())
}
