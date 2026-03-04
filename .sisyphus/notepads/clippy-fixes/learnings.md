## Clippy Fixes (2026-03-03)
- src/cli.rs: Changed run_inspect argument from &PathBuf to &Path for better idiomatic Rust.
- src/engine/shell.rs: Replaced filter_map(|l| l.ok()) with map_while(Result::ok) for cleaner iterator chain.
- src/engine/communities.rs: Collapsed nested match into if let Ok(...) and further collapsed with && let (let_chains).
- src/storage/sqlite.rs: Added explicit type annotations to transmute call for sqlite3_auto_extension.
