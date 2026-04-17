# Plan 007 — Step Summaries

## Step 1 — Nested config scaffolding (commit: b19e61e)
**Decisions**:
- Nested TOML shape `[llm]` + `[llm.claude_cli]` from day one (codex-proxy-2's future-proof recommendation). Adding OpenAI later = add `[llm.openai]`, no breaking change.
- `load_with_env(base, &HashMap)` takes explicit env map; production wrapper `load_from_process_env()` collects `std::env::vars()`. Tests never touch process env → no `unsafe { std::env::set_var }` needed in Rust 1.94.
- `#[serde(default)]` at both struct and field level → any partial TOML section works.

**Rejected**:
- Hand-rolled TOML parse (like project.rs does). Nested structure makes it brittle; `toml = "0.8"` is cheap.
- Flat `[llm] cli_path = "claude"` shape. Would force breaking rename when BL-012 / future providers land.
- Bare env var `CLAUDE_CLI_PATH`. Renamed to `MENGDIE_LLM_CLAUDE_CLI_PATH` for provider-namespacing consistency.

**Cross-step deps**:
- `LlmConfig` and `ClaudeCliConfig` types are consumed by Step 2's `ClaudeCliProvider::from_config(&LlmConfig)`. `cfg.claude_cli.cli_path` is the binary path read-site.

**Actual files**: Cargo.toml, Cargo.lock, src/core/mod.rs, src/core/config.rs
