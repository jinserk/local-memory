/// Unit tests for `ModelRegistry` and `ArchConfig::format_prompt`.
///
/// These tests operate entirely on the embedded `models.yaml` — no model
/// files, no network, no GPU required.
use local_memory::model::candle::registry::{ArchConfig, ArchKind, ModelRegistry};

// ── Registry::load ────────────────────────────────────────────────────────────

#[test]
fn test_registry_loads_without_error() {
    ModelRegistry::load().expect("embedded models.yaml should parse cleanly");
}

// ── Exact model lookups ───────────────────────────────────────────────────────

#[test]
fn test_exact_bert_nomic_v1_5() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("nomic-ai/nomic-embed-text-v1.5").unwrap();
    assert_eq!(resolved.arch, ArchKind::Bert);
    assert!(matches!(resolved.config, ArchConfig::Embedding(_)));
}

#[test]
fn test_exact_bert_nomic_v1() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("nomic-ai/nomic-embed-text-v1").unwrap();
    assert_eq!(resolved.arch, ArchKind::Bert);
}

#[test]
fn test_exact_nuextract_1_5_is_phi3() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("numind/NuExtract-1.5").unwrap();
    assert_eq!(resolved.arch, ArchKind::Phi3);
    assert!(matches!(resolved.config, ArchConfig::Generation(_)));
}

#[test]
fn test_exact_nuextract_2_0_2b_is_qwen2() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("numind/NuExtract-2.0-2B").unwrap();
    assert_eq!(resolved.arch, ArchKind::Qwen2);
    assert!(matches!(resolved.config, ArchConfig::Generation(_)));
}

#[test]
fn test_exact_nuextract_2_0_4b_is_qwen2() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("numind/NuExtract-2.0-4B").unwrap();
    assert_eq!(resolved.arch, ArchKind::Qwen2);
}

#[test]
fn test_exact_nuextract_2_0_8b_is_qwen2() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("numind/NuExtract-2.0-8B").unwrap();
    assert_eq!(resolved.arch, ArchKind::Qwen2);
}

// ── Pattern fallback lookups ──────────────────────────────────────────────────

#[test]
fn test_pattern_bert_via_bert_substring() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg
        .resolve("sentence-transformers/all-MiniLM-L6-v2-bert")
        .unwrap();
    assert_eq!(resolved.arch, ArchKind::Bert);
}

#[test]
fn test_pattern_bert_via_nomic_substring() {
    let reg = ModelRegistry::load().unwrap();
    // A hypothetical future nomic model not yet in the models table.
    let resolved = reg.resolve("nomic-ai/nomic-embed-text-v2").unwrap();
    assert_eq!(resolved.arch, ArchKind::Bert);
}

#[test]
fn test_pattern_phi3_via_phi_substring() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("microsoft/Phi-3-mini-4k-instruct").unwrap();
    assert_eq!(resolved.arch, ArchKind::Phi3);
}

#[test]
fn test_pattern_qwen2_via_qwen_substring() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("Qwen/Qwen2-1.5B-Instruct").unwrap();
    assert_eq!(resolved.arch, ArchKind::Qwen2);
}

#[test]
fn test_pattern_case_insensitive() {
    // Pattern matching lowercases the model name, so "QWEN" must still hit the qwen2 pattern.
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("org/QWEN-7B").unwrap();
    assert_eq!(resolved.arch, ArchKind::Qwen2);
}

// ── Unknown model error ───────────────────────────────────────────────────────

#[test]
fn test_unknown_model_returns_error() {
    let reg = ModelRegistry::load().unwrap();
    let result = reg.resolve("totally/unknown-model-xyz-999");
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("No architecture mapping"),
        "unexpected error message: {msg}"
    );
}

// ── EmbeddingConfig fields ────────────────────────────────────────────────────

#[test]
fn test_bert_embedding_config_prefixes() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("nomic-ai/nomic-embed-text-v1.5").unwrap();
    if let ArchConfig::Embedding(cfg) = resolved.config {
        assert_eq!(cfg.query_prefix, "search_query: ");
        assert_eq!(cfg.document_prefix, "search_document: ");
    } else {
        panic!("expected EmbeddingConfig");
    }
}

// ── GenerationConfig fields ───────────────────────────────────────────────────

#[test]
fn test_qwen2_generation_config_eos_tokens() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("numind/NuExtract-2.0-2B").unwrap();
    if let ArchConfig::Generation(cfg) = resolved.config {
        assert!(cfg.eos_tokens.contains(&"<|im_end|>".to_string()));
        assert_eq!(cfg.eos_fallback_id, 151645);
        assert!(cfg.max_new_tokens > 0);
    } else {
        panic!("expected GenerationConfig");
    }
}

#[test]
fn test_phi3_generation_config_eos_tokens() {
    let reg = ModelRegistry::load().unwrap();
    let resolved = reg.resolve("numind/NuExtract-1.5").unwrap();
    if let ArchConfig::Generation(cfg) = resolved.config {
        assert!(cfg.eos_tokens.contains(&"<|endoftext|>".to_string()));
        assert_eq!(cfg.eos_fallback_id, 0);
    } else {
        panic!("expected GenerationConfig");
    }
}

#[test]
fn test_generation_config_contains_text_placeholder() {
    // The prompt template must contain {text} so format_prompt can substitute.
    let reg = ModelRegistry::load().unwrap();
    for name in ["numind/NuExtract-1.5", "numind/NuExtract-2.0-2B"] {
        let resolved = reg.resolve(name).unwrap();
        if let ArchConfig::Generation(cfg) = resolved.config {
            assert!(
                cfg.prompt_template.contains("{text}"),
                "template for {name} is missing {{text}} placeholder"
            );
        }
    }
}

// ── ArchConfig::format_prompt ─────────────────────────────────────────────────

fn qwen2_gen_config() -> ArchConfig {
    let reg = ModelRegistry::load().unwrap();
    reg.resolve("numind/NuExtract-2.0-2B").unwrap().config
}

fn phi3_gen_config() -> ArchConfig {
    let reg = ModelRegistry::load().unwrap();
    reg.resolve("numind/NuExtract-1.5").unwrap().config
}

fn bert_embed_config() -> ArchConfig {
    let reg = ModelRegistry::load().unwrap();
    reg.resolve("nomic-ai/nomic-embed-text-v1.5")
        .unwrap()
        .config
}

// Rule 1 — already-formatted prompts pass through unchanged.

#[test]
fn test_format_prompt_passthrough_qwen2_markers() {
    let cfg = qwen2_gen_config();
    let prompt = "<|im_start|>user\nhello<|im_end|>\n<|im_start|>assistant\n";
    assert_eq!(cfg.format_prompt(prompt), prompt);
}

#[test]
fn test_format_prompt_passthrough_phi3_markers() {
    let cfg = phi3_gen_config();
    let prompt = "<|input|>\n### Template:\n{}\n### Text:\nhello\n<|output|>\n";
    assert_eq!(cfg.format_prompt(prompt), prompt);
}

// Rule 2 — TEMPLATE:<name> prefix uses the named template.

#[test]
fn test_format_prompt_template_tag_substitutes_text() {
    let cfg = qwen2_gen_config();
    // communities.rs sends this format for summarisation.
    let prompt = "TEMPLATE:summary\nentity A: desc\nentity B: desc";
    let result = cfg.format_prompt(prompt);
    // The summary template has {text} replaced with the body.
    assert!(result.contains("entity A: desc"));
    assert!(result.contains("entity B: desc"));
    // And it should NOT contain the raw TEMPLATE: prefix.
    assert!(!result.contains("TEMPLATE:"));
}

#[test]
fn test_format_prompt_template_tag_trims_body() {
    let cfg = qwen2_gen_config();
    // Extra whitespace around body should be trimmed.
    let prompt = "TEMPLATE:summary\n  trimmed content  ";
    let result = cfg.format_prompt(prompt);
    assert!(result.contains("trimmed content"));
    assert!(!result.contains("  trimmed content  "));
}

// Rule 3 — plain prompt uses default template, strips "Text: " prefix.

#[test]
fn test_format_prompt_strips_text_prefix_for_qwen2() {
    let cfg = qwen2_gen_config();
    // ingestion.rs appends the text after "Text: ".
    let prompt = "Extract entities ...\nText: The quick brown fox";
    let result = cfg.format_prompt(prompt);
    assert!(result.contains("The quick brown fox"));
    // The legacy instruction text should NOT appear in the final prompt.
    assert!(!result.contains("Extract entities"));
}

#[test]
fn test_format_prompt_plain_text_inserted_when_no_text_prefix() {
    let cfg = qwen2_gen_config();
    let prompt = "just some raw text";
    let result = cfg.format_prompt(prompt);
    assert!(result.contains("just some raw text"));
}

#[test]
fn test_format_prompt_embedding_passthrough() {
    // Embedding configs must return the prompt unchanged.
    let cfg = bert_embed_config();
    let prompt = "any text whatsoever";
    assert_eq!(cfg.format_prompt(prompt), prompt);
}

// Edge cases.

#[test]
fn test_format_prompt_empty_string() {
    let cfg = qwen2_gen_config();
    // Should not panic; result contains the template structure with empty body.
    let result = cfg.format_prompt("");
    assert!(!result.contains("{text}"));
}

#[test]
fn test_format_prompt_template_tag_no_newline_is_treated_as_plain() {
    // If the "TEMPLATE:" tag has no newline, it falls through to plain handling.
    let cfg = qwen2_gen_config();
    let prompt = "TEMPLATE:summary-no-newline";
    // Must not panic; result should include the prompt content.
    let result = cfg.format_prompt(prompt);
    assert!(!result.is_empty());
}
