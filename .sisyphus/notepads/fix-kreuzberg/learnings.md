## Kreuzberg Dependency Update
- Updated `kreuzberg` from `0.1` to `4.0.0-rc.29` in `Cargo.toml`.
- Verified with `cargo test`, all 5 integration tests passed (took ~171s).
- Note: Cargo resolved `4.0.0-rc.29` to `4.4.1` during the test run, but the `Cargo.toml` reflects the requested version.
