# Integration QA Unresolved Problems

- Compilation warning: `is_image_file` is never used in `src/engine/ingestion.rs`.
- The `lmcli ingest` command was previously reported as broken (missing `run_file` method), but upon inspection, the method exists. However, `cargo run --bin lmcli -- test` failed initially with a compilation error that disappeared after `cargo check`. This suggests some inconsistency in the build environment or transient state.
