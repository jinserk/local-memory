use anyhow::Result;
use local_memory::config::{MemoryTier, TierConfig};

#[test]
fn test_tier_config_default() {
    let config = TierConfig::default();
    assert_eq!(config.default_tier, MemoryTier::Semantic);
    assert_eq!(config.default_episodic_ttl_seconds, Some(3600));
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
