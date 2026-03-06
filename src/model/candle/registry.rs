use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

// The YAML is embedded at compile time so there is no runtime file-path dependency.
const MODELS_YAML: &str = include_str!("../../../models.yaml");

// ── Raw serde types (mirror the YAML schema exactly) ─────────────────────────

#[derive(Debug, Deserialize)]
struct RawRegistry {
    architectures: HashMap<String, RawArchConfig>,
    models: HashMap<String, RawModelEntry>,
    patterns: Vec<RawPattern>,
}

#[derive(Debug, Deserialize)]
struct RawArchConfig {
    role: String,
    // embedding-only fields
    query_prefix: Option<String>,
    document_prefix: Option<String>,
    // generation-only fields
    max_new_tokens: Option<usize>,
    eos_tokens: Option<Vec<String>>,
    eos_fallback_id: Option<u32>,
    prompt_templates: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct RawModelEntry {
    arch: String,
    prompt_template: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPattern {
    contains: String,
    arch: String,
    prompt_template: Option<String>,
}

// ── Public domain types ───────────────────────────────────────────────────────

/// Identifies which architecture family a model uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchKind {
    Bert,
    Phi3,
    Qwen2,
}

/// All configuration parameters needed to drive a specific architecture.
#[derive(Debug, Clone)]
pub enum ArchConfig {
    Embedding(EmbeddingConfig),
    Generation(GenerationConfig),
}

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub query_prefix: String,
    pub document_prefix: String,
}

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub max_new_tokens: usize,
    pub eos_tokens: Vec<String>,
    pub eos_fallback_id: u32,
    /// The prompt template string for the resolved template name.
    /// Contains `{text}` as the substitution placeholder.
    pub prompt_template: String,
}

impl ArchConfig {
    /// Format a raw `prompt` into the final string fed to the model.
    ///
    /// Rules (in priority order):
    /// 1. If the prompt already contains arch-specific markers (`<|im_start|>`,
    ///    `<|input|>`) it is returned unchanged — the caller already formatted it.
    /// 2. If the prompt starts with `"TEMPLATE:<name>\n"` the named template is
    ///    used and the remainder is treated as `{text}`.
    /// 3. Otherwise the default template (stored in the config) is used, with
    ///    the raw prompt as `{text}` (stripping a trailing `"Text: "` prefix if
    ///    present, for backwards-compatibility with `ingestion.rs`).
    pub fn format_prompt(&self, prompt: &str) -> String {
        match self {
            ArchConfig::Embedding(_) => prompt.to_string(),
            ArchConfig::Generation(cfg) => {
                // Already formatted — pass through.
                if prompt.contains("<|im_start|>") || prompt.contains("<|input|>") {
                    return prompt.to_string();
                }
                // Named template override from caller.
                // `ingestion.rs` / `communities.rs` use "TEMPLATE:summary\n{ctx}".
                if let Some(rest) = prompt.strip_prefix("TEMPLATE:") {
                    if let Some(nl) = rest.find('\n') {
                        let _name = &rest[..nl]; // reserved for future multi-template support
                        let text = rest[nl + 1..].trim();
                        return cfg.prompt_template.replace("{text}", text);
                    }
                }
                // Default: strip legacy "Text: " prefix if present.
                let text = if let Some(pos) = prompt.rfind("Text: ") {
                    prompt[pos + 6..].trim()
                } else {
                    prompt.trim()
                };
                cfg.prompt_template.replace("{text}", text)
            }
        }
    }
}

/// The resolved entry for a model (arch kind + full arch config).
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub arch: ArchKind,
    pub config: ArchConfig,
}

// ── Registry ──────────────────────────────────────────────────────────────────

/// Parses `models.yaml` and resolves model names to their architecture + config.
pub struct ModelRegistry {
    raw: RawRegistry,
}

impl ModelRegistry {
    /// Load and parse the embedded `models.yaml`.
    pub fn load() -> Result<Self> {
        let raw: RawRegistry =
            serde_yaml::from_str(MODELS_YAML).context("Failed to parse embedded models.yaml")?;
        Ok(Self { raw })
    }

    /// Resolve a model name to its `ArchKind` and `ArchConfig`.
    ///
    /// Lookup order:
    /// 1. Exact match in `models:` table.
    /// 2. First matching pattern in `patterns:` (substring `contains` check, lowercased).
    pub fn resolve(&self, model_name: &str) -> Result<ResolvedModel> {
        // 1. Exact match.
        let (arch_name, template_name) = if let Some(entry) = self.raw.models.get(model_name) {
            (
                entry.arch.clone(),
                entry
                    .prompt_template
                    .clone()
                    .unwrap_or_else(|| "extraction".to_string()),
            )
        } else {
            // 2. Pattern match (lowercased).
            let lower = model_name.to_lowercase();
            let matched = self
                .raw
                .patterns
                .iter()
                .find(|p| lower.contains(&p.contains))
                .with_context(|| {
                    format!(
                        "No architecture mapping found for model '{}'. \
                         Add it to models.yaml.",
                        model_name
                    )
                })?;
            (
                matched.arch.clone(),
                matched
                    .prompt_template
                    .clone()
                    .unwrap_or_else(|| "extraction".to_string()),
            )
        };

        let raw_arch =
            self.raw.architectures.get(&arch_name).with_context(|| {
                format!("Architecture '{}' not defined in models.yaml", arch_name)
            })?;

        let arch_kind = match arch_name.as_str() {
            "bert" => ArchKind::Bert,
            "phi3" => ArchKind::Phi3,
            "qwen2" => ArchKind::Qwen2,
            other => anyhow::bail!("Unknown architecture '{}' in models.yaml", other),
        };

        let config = match raw_arch.role.as_str() {
            "embedding" => ArchConfig::Embedding(EmbeddingConfig {
                query_prefix: raw_arch
                    .query_prefix
                    .clone()
                    .unwrap_or_else(|| "search_query: ".to_string()),
                document_prefix: raw_arch
                    .document_prefix
                    .clone()
                    .unwrap_or_else(|| "search_document: ".to_string()),
            }),
            "generation" => {
                let templates = raw_arch.prompt_templates.as_ref().with_context(|| {
                    format!("Architecture '{}' has no prompt_templates", arch_name)
                })?;
                let template = templates.get(&template_name).with_context(|| {
                    format!(
                        "Template '{}' not found in architecture '{}'",
                        template_name, arch_name
                    )
                })?;
                ArchConfig::Generation(GenerationConfig {
                    max_new_tokens: raw_arch.max_new_tokens.unwrap_or(512),
                    eos_tokens: raw_arch.eos_tokens.clone().unwrap_or_default(),
                    eos_fallback_id: raw_arch.eos_fallback_id.unwrap_or(0),
                    prompt_template: template.clone(),
                })
            }
            other => anyhow::bail!("Unknown role '{}' in models.yaml", other),
        };

        Ok(ResolvedModel {
            arch: arch_kind,
            config,
        })
    }
}
