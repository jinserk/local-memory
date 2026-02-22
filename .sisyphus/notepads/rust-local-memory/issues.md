
## simsimd API Confusion
- `simsimd`'s `f32::cos` conflicts with the standard library's `f32::cos` (trigonometric cosine).
- Use `SpatialSimilarity::cos` or `simsimd::SpatialSimilarity::cos` to avoid ambiguity.
- `simsimd` functions return `Option<Distance>`, so they need to be handled with `.ok_or_else()` or similar.

## fjall Iterator and Guard
- In `fjall` 3.0.2, `Keyspace::iter()` yields `Guard` objects.
- `Guard` objects must be converted to `Result<(Slice, Slice), Error>` using `into_inner()` to access the key and value.
- `into_inner()` consumes the `Guard`, so it can only be called once.

## 2026-02-21: Code Quality Review Findings

### Critical Issues (Potential Panics)
1. **`src/engine/search_stage1.rs:24`**: `partial_cmp().unwrap()` can panic on NaN values
   - Fix: Use `unwrap_or(std::cmp::Ordering::Equal)` or handle NaN explicitly

### Missing Documentation (15 functions)
- All public functions returning `Result` need `# Errors` doc section
- `hamming_scan` needs `# Panics` section due to potential panic

### Clippy Warnings Summary
- **High Priority**: `collapsible_if` (config.rs), `derivable_impls` (tier.rs), `map_unwrap_or` (config.rs)
- **Medium Priority**: `uninlined_format_args` (12 occurrences), `missing_errors_doc` (15), `needless_pass_by_value` (mcp/tools.rs)
- **Low Priority**: `must_use_candidate` (7), `cast_possible_truncation` (3), `too_many_lines` (main.rs)

### Test Code Quality
- 17 `unwrap()` calls in test code (acceptable but could use `expect()` with context)
- Mock embedder duplicated in `engine/ingestion.rs` and `mcp/tools.rs`

### Quick Fix Command
```bash
cargo clippy --fix --allow-dirty --allow-staged
```
