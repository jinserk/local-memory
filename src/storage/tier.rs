use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Memory tier representing the storage class of a memory.
///
/// - `Episodic`: Short-term, temporary memories with optional TTL (time-to-live)
/// - `Semantic`: Long-term, permanent memories for knowledge storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryTier {
    /// Short-term, temporary memories with optional TTL
    Episodic,
    /// Long-term, permanent memories for knowledge storage
    Semantic,
}

impl Default for MemoryTier {
    fn default() -> Self {
        MemoryTier::Semantic
    }
}

impl std::fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryTier::Episodic => write!(f, "episodic"),
            MemoryTier::Semantic => write!(f, "semantic"),
        }
    }
}

impl std::str::FromStr for MemoryTier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "episodic" => Ok(MemoryTier::Episodic),
            "semantic" => Ok(MemoryTier::Semantic),
            _ => Err(format!("Invalid memory tier: {}", s)),
        }
    }
}

/// Helper functions for TTL management
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Convert Duration to expiration timestamp (seconds since UNIX epoch)
pub fn duration_to_expiration(duration: Duration) -> u64 {
    current_timestamp() + duration.as_secs()
}

/// Check if an expiration timestamp has passed
pub fn is_expired(expires_at: Option<u64>) -> bool {
    match expires_at {
        Some(exp) => current_timestamp() >= exp,
        None => false,
    }
}

/// Configuration for memory tier behavior
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TierConfig {
    /// Default tier for new memories
    pub default_tier: MemoryTier,
    /// Default TTL for episodic memories (in seconds)
    /// None means no expiration, Some(seconds) means expiration after that many seconds
    pub default_episodic_ttl_seconds: Option<u64>,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            default_tier: MemoryTier::Semantic,
            default_episodic_ttl_seconds: Some(3600), // 1 hour default TTL for episodic
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_tier_serialization() {
        let tier = MemoryTier::Episodic;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"episodic\"");

        let tier = MemoryTier::Semantic;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"semantic\"");
    }

    #[test]
    fn test_tier_deserialization() {
        let tier: MemoryTier = serde_json::from_str("\"episodic\"").unwrap();
        assert_eq!(tier, MemoryTier::Episodic);

        let tier: MemoryTier = serde_json::from_str("\"semantic\"").unwrap();
        assert_eq!(tier, MemoryTier::Semantic);
    }

    #[test]
    fn test_tier_from_str() {
        assert_eq!(
            MemoryTier::from_str("episodic").unwrap(),
            MemoryTier::Episodic
        );
        assert_eq!(
            MemoryTier::from_str("EPISODIC").unwrap(),
            MemoryTier::Episodic
        );
        assert_eq!(
            MemoryTier::from_str("semantic").unwrap(),
            MemoryTier::Semantic
        );
        assert!(MemoryTier::from_str("invalid").is_err());
    }

    #[test]
    fn test_tier_default() {
        assert_eq!(MemoryTier::default(), MemoryTier::Semantic);
    }

    #[test]
    fn test_tier_config_default() {
        let config = TierConfig::default();
        assert_eq!(config.default_tier, MemoryTier::Semantic);
        assert_eq!(config.default_episodic_ttl_seconds, Some(3600));
    }

    #[test]
    fn test_expiration_check() {
        // No expiration means not expired
        assert!(!is_expired(None));

        // Past timestamp means expired
        let past = current_timestamp() - 1000;
        assert!(is_expired(Some(past)));

        // Future timestamp means not expired
        let future = current_timestamp() + 1000;
        assert!(!is_expired(Some(future)));
    }
}
