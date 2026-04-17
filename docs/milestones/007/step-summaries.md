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

## Step 2 — LLM trait + ClaudeCliProvider + error classification (commit: 9d404b4)
**Decisions**:
- Boxed-future trait return (`Pin<Box<dyn Future + Send + 'a>>`) — object-safe so BL-007 can store provider as `Box<dyn LlmProvider>` under shared state. No `async-trait` crate.
- Child stays OUTSIDE the timed future. Explicit `child.kill().await + child.wait().await` on both timeout and I/O-error paths. kill_on_drop(true) is belt-and-braces only. (Codex review finding; initial implementation relied on kill_on_drop alone.)
- Added 9th `Io { op: &'static str, source }` error variant for read/wait/write errors that aren't spawn failures. Prior draft misclassified stdout-read errors as `Spawn`, which was misleading. (Codex review finding.)
- `classify_output` is pure sync — builds `std::process::Output`-shaped input, returns `Result<String, LlmError>`. Keeps 9 of 12 AC3 rows testable without real subprocesses.
- Regex patterns for `ExitKind` are cached via `OnceLock<Regex>` (no per-call compilation).
- `regex = "1"` added as a direct dep (already transitive; now explicit).

**Rejected**:
- `async-trait` crate: unnecessary on Rust 1.94; defining `async fn` in trait directly would require `impl LlmProvider` generics at every call site, blocking `Box<dyn LlmProvider>`. Chose boxed-future explicitly.
- `wait_with_output`: deadlocks when child writes >64KB stderr before reading stdin. Replaced with three-way `tokio::join!`.
- `is_retryable()` helper on `LlmError`: classification != policy; first caller (BL-007) will decide what to do with `ExitKind::RateLimited | Network` vs `Auth | Model`.

**Cross-step deps**:
- `LlmProvider` trait + `ClaudeCliProvider::from_config(&LlmConfig)` — consumed by Step 3 integration tests and later by BL-007.
- Argv shape in `build_command` is a contract locked by the Step 3 help-smoke test (asserts each flag substring appears in `claude --help`).
- `classify_output` behavior is covered by unit tests in Step 2; Step 3 tests only exercise `complete()` end-to-end.

**Actual files**: Cargo.toml, Cargo.lock, src/core/mod.rs, src/core/llm.rs
