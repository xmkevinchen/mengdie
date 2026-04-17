---
id: "007"
title: "BL-005 — LLM Provider Trait + Claude CLI Implementation"
type: plan
created: 2026-04-16
status: reviewed
discussion: "docs/discussions/016-dreaming-evolution/"
---

# Feature: BL-005 — LLM Provider Trait + Claude CLI Implementation

## Goal

Add a minimal `LlmProvider` trait and a `ClaudeCliProvider` implementation that shells out to the `claude` CLI binary, so later Phase 2 items (BL-007 dream synthesis, BL-012 RAG) can call an LLM without rewriting auth, config, or error plumbing.

## Scope boundaries

- **In**: trait, Claude CLI provider, nested config (`[llm]` + `[llm.claude_cli]`), error classification with coarse retry taxonomy, unit tests, opt-in integration test.
- **Out**: clustering (BL-006), synthesis (BL-007), decay (BL-008), MCP dream tool (BL-009), other providers (OpenAI/Codex OAuth). Trait must not leak assumptions that block those.
- **No callers yet**: the trait is unused by any existing code after this plan. That is intentional — BL-007 will be the first caller.

## Auth Decision — Delegate Credentials to the Claude CLI

This plan intentionally does **not** read credentials in-process. BL-005's original wording mentioned reading `~/.claude/credentials` + `ANTHROPIC_API_KEY` fallback; we deviate from that and defer to the CLI.

Rationale:
- Claude CLI owns its credential storage format (Keychain on macOS, `~/.claude/.credentials.json` on Linux, future changes). Mengdie should not parse, copy, log, validate, or migrate that.
- `tokio::process::Command` inherits the parent process env; an authenticated CLI works immediately with zero config.
- Security boundary is cleanest this way: mengdie passes only prompt input + CLI flags. It never touches secrets.
- User env hygiene is **not** our job here. Unlike a server-side product (e.g. SmartPal clearing `ANTHROPIC_API_KEY` to prevent cross-account bleed), mengdie is a personal CLI — a user who sets `ANTHROPIC_API_KEY` in their shell expects it to be respected. We do not strip it.

Matches SmartPal's `bare=False` pattern (reference: `/Users/ckai/Workspace/Projects/SmartPal/backend/app/core/llm/providers/claude_cli.py`).

### Known limitation: argv exposure of the system prompt

`--system-prompt <value>` is passed as a command-line argument, which means the system prompt content is visible in the OS process table (`ps aux`, `/proc/<pid>/cmdline`) for the lifetime of the child subprocess. When BL-007 synthesizes memory clusters, that content will include private development notes.

**Risk in BL-005 context**: low. Mengdie runs on the user's own macOS/Linux workstation, single-user. Any local process already has access to the memory files at `~/.mengdie/db.sqlite` anyway. This is not a net-new leak.

**Trigger to revisit**: mengdie is ever deployed to a shared / multi-tenant machine, or runs in a CI environment next to untrusted workloads. At that point, investigate whether `claude -p` can accept the system prompt via a stdin envelope (`stream-json` input format, as SmartPal does) instead of argv, or fold the system content into the user turn.

For now, proceed with argv. Add this note to the `ClaudeCliProvider` doc comment so a future reader sees the tradeoff at the call site.

### Anticipated extension point: multi-message trait method

The current trait signature is `complete(system: &str, prompt: &str)`. When BL-012 (RAG `memory_query`) lands, it will want a `[system_context, user_question]` message array, and potentially multi-turn follow-ups. The plan for BL-012 is NOT to rewrite `complete()` — it is to add an additive method:

```rust
pub trait LlmProvider: Send + Sync {
    fn complete<'a>(&'a self, system: &'a str, prompt: &'a str) -> BoxedFuture<'a, String>;
    fn complete_messages<'a>(&'a self, messages: &'a [Message]) -> BoxedFuture<'a, String> {
        // default impl: collapse to complete() by concatenating with role markers
    }
    fn model(&self) -> &str;
}
```

BL-007 (single-shot synthesis) keeps using `complete()`. BL-012 uses `complete_messages()`. The default impl keeps existing providers working. This is a post-BL-005 change, not a blocker for this plan.

## Claude CLI Version Compatibility

Target: Claude Code CLI **2.1.x**. The flags used (`-p`, `--output-format text`, `--no-session-persistence`, `--tools ""`, `--permission-mode bypassPermissions`, `--model`, `--system-prompt`) are application flags, not a semver-versioned API. An opt-in "help smoke test" (Step 3) asserts each flag appears in `claude --help` output, so a breaking CLI update surfaces as a test failure rather than a silent runtime error.

## Steps

### Step 1: Nested config scaffolding (AC2) — DONE b19e61e
- [x] Add `toml = "0.8"` to `Cargo.toml` dependencies
- [x] Create `src/core/config.rs` with nested shape — generic defaults in `[llm]`, provider-specific overrides in `[llm.<provider>]`:
  ```toml
  [llm]
  provider = "claude-cli"
  model = "claude-sonnet-4-6"
  timeout_secs = 120

  [llm.claude_cli]
  cli_path = "claude"
  ```
  Types:
  - `MengdieConfig { llm: LlmConfig }` (serde `Deserialize`, `Default`)
  - `LlmConfig { provider: String, model: String, timeout_secs: u64, claude_cli: ClaudeCliConfig }` — all fields `#[serde(default)]` so partial TOML works
  - `ClaudeCliConfig { cli_path: String }` — defaults `cli_path = "claude"`
  - Defaults at `[llm]` level: `provider = "claude-cli"`, `model = "claude-sonnet-4-6"`, `timeout_secs = 120`
- [x] `MengdieConfig::load()` — reads `~/.mengdie/config.toml` if present; missing file → defaults; parse error → `anyhow::Error` whose `Display` includes the file path
- [x] `MengdieConfig::load_with_env(base: Self, env: &HashMap<String, String>) -> Result<Self>` — applies env overrides to a pre-loaded `base` config. Takes an explicit env map (not `std::env::vars()`) so tests never mutate process env. Kept file loading and env overlay in SEPARATE functions for testability: `load_from_process_env()` composes them for production callers. Env overrides applied:
  - `MENGDIE_LLM_PROVIDER` → `llm.provider`
  - `MENGDIE_LLM_MODEL` → `llm.model`
  - `MENGDIE_LLM_TIMEOUT_SECS` → `llm.timeout_secs`
  - `MENGDIE_LLM_CLAUDE_CLI_PATH` → `llm.claude_cli.cli_path` (provider-namespaced; no bare `CLAUDE_CLI_PATH`)
- [x] Thin wrapper `MengdieConfig::load_from_process_env()` collects `std::env::vars()` into the map and calls `load_with_env` (callers in production use this).
- [x] Register module: add `pub mod config;` to `src/core/mod.rs`
- [x] Unit test: missing file → all defaults
- [x] Unit test: TOML with only `[llm] model = "claude-haiku-4-5"` returns that model, all other fields default, `claude_cli.cli_path = "claude"`
- [x] Unit test: TOML with `[llm.claude_cli] cli_path = "/opt/bin/claude"` applies
- [x] Unit test: env map override wins over file value (no `std::env::set_var` — use the explicit-env API)
- [x] Unit test: malformed TOML → `Err` whose `Display` contains the file path

Expected files: `Cargo.toml`, `src/core/config.rs`, `src/core/mod.rs`

### Step 2: LLM trait + Claude CLI provider + error classification (AC1, AC3) — DONE 9d404b4

- [x] Add module declaration `pub mod llm;` to `src/core/mod.rs`
- [x] Create `src/core/llm.rs` with the error enum. **No `async-trait` crate** — project already uses stable Rust 1.75+ `async fn` in traits. `thiserror = "2"` already in deps; use the `2.x` derive syntax.
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum LlmError {
      #[error("claude CLI binary not found at configured path")]
      BinaryNotFound,
      #[error("failed to spawn CLI subprocess: {0}")]
      Spawn(#[source] std::io::Error),
      #[error("CLI subprocess timed out after {0:?}")]
      Timeout(std::time::Duration),
      #[error("CLI exited with code {code}; stderr: {stderr}")]
      NonZeroExit { code: i32, stderr: String, kind: ExitKind },
      #[error("CLI stdin closed early: {0}")]
      BrokenPipe(#[source] std::io::Error),
      #[error("CLI returned empty stdout")]
      EmptyOutput,
      #[error("CLI stdout was not valid UTF-8")]
      InvalidUtf8,
      #[error("CLI killed by signal")]
      Signal,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum ExitKind { Auth, RateLimited, Network, Model, Other }
  ```
  The `ExitKind` tag is **classification only** — no `is_retryable()` helper in this plan (callers decide policy; that lives in BL-007's first retry use). Classification is done by regex on combined `stderr + result_text`, mirroring SmartPal's `cli_errors.py`:
  - `Auth`: `Invalid API key`, `not logged in`, `unauthorized`, `\b401\b`, `\b403\b`
  - `RateLimited`: `API Error:\s*429`, `overloaded`, `\b529\b`
  - `Network`: `ECONNRESET`, `ECONNREFUSED`, `ETIMEDOUT`, `connection refused`, `connection reset`
  - `Model`: `model not found`, `unsupported model`
  - `Other`: everything else
- [x] Define the trait. Use boxed-future return so `Box<dyn LlmProvider>` is object-safe (BL-007 will need to store the provider behind a trait object under shared state):
  ```rust
  use std::future::Future;
  use std::pin::Pin;

  pub trait LlmProvider: Send + Sync {
      fn complete<'a>(
          &'a self,
          system: &'a str,
          prompt: &'a str,
      ) -> Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send + 'a>>;
      fn model(&self) -> &str;
  }
  ```
- [x] `ClaudeCliProvider { cli_path: PathBuf, model: String, timeout: Duration }` with:
  - `pub fn from_config(cfg: &LlmConfig) -> Self` — reads `cfg.claude_cli.cli_path` for the binary path
  - `pub(crate) fn build_command(&self, system: &str, prompt_stdin: bool) -> tokio::process::Command` — returns a configured `Command` with argv and `.kill_on_drop(true)` and `.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())`. Argv (**order-sensitive**):
    ```
    [<cli_path>, "-p",
     "--output-format", "text",
     "--no-session-persistence",
     "--permission-mode", "bypassPermissions",  # NOTE: required even with --tools "" — CLI checks permissions at startup, not just at tool-call time. Do NOT remove.
     "--tools", "",
     "--model", <model>,
     "--system-prompt", <system>]
    ```
  - `impl LlmProvider for ClaudeCliProvider`:
    1. `build_command()` → `spawn()` (map `io::ErrorKind::NotFound` → `BinaryNotFound`, other `io::Error` → `Spawn`)
    2. Concurrently drive three tasks (critical — `wait_with_output` + timeout deadlocks when the child writes >64KB stderr before reading stdin):
       - Task A: write `prompt` bytes to child stdin + close; map `BrokenPipe` to `LlmError::BrokenPipe`
       - Task B: read child stdout to `Vec<u8>`
       - Task C: read child stderr to `String` (lossy)
       Use `tokio::try_join!` or a manual select, wrapped in `tokio::time::timeout(self.timeout, ...)`.
    3. On timeout: call `child.start_kill()` → `child.wait().await` to reap — NO zombies. Return `LlmError::Timeout(self.timeout)`.
    4. After clean exit, call `classify_output(status, &stdout, &stderr)` helper (pure, sync, no I/O) to produce `Result<String, LlmError>`. `status.code()`:
       - `Some(0)` → validate UTF-8 → empty check → `Ok(String)`
       - `Some(code)` → regex-classify stderr → `NonZeroExit { code, stderr, kind }`
       - `None` → `Signal` (Unix: process terminated by signal; on Windows, unreachable via normal flow)
- [x] Unit test (pure): `classify_output` happy path — exit 0, stdout = `"hello\n"` → `Ok("hello\n")`
- [x] Unit test (pure): `classify_output` exit 0, stdout = `""` → `EmptyOutput`
- [x] Unit test (pure): `classify_output` exit 0, stdout = `[0xFF, 0xFE, 0xFD]` → `InvalidUtf8`
- [x] Unit test (pure): `classify_output` exit 1, stderr = `"Invalid API key"` → `NonZeroExit { code: 1, kind: Auth, .. }`
- [x] Unit test (pure): `classify_output` exit 1, stderr = `"API Error: 429 rate_limit"` → `NonZeroExit { kind: RateLimited, .. }`
- [x] Unit test (pure): `classify_output` exit 1, stderr = `"ECONNRESET"` → `NonZeroExit { kind: Network, .. }`
- [x] Unit test (pure): `classify_output` exit 1, stderr = `"model not found: claude-zzz"` → `NonZeroExit { kind: Model, .. }`
- [x] Unit test (pure): `classify_output` exit 1, stderr = `"weird error"` → `NonZeroExit { kind: Other, .. }`
- [x] Unit test (pure): `classify_output` signal exit (`status.code() = None`) → `Signal`
- [x] Unit test (`build_command`): argv matches the exact order above when `system = "answer in JSON"`, `model = "claude-sonnet-4-6"`, `cli_path = "/usr/bin/claude"`
- [x] Unit test (`build_command`): pathological system prompt (contains `"`, newlines, a trailing backslash — e.g. `"say \"hi\"\nworld\\"`) is passed as **one** argv element (confirm by counting argv length is identical to happy-path case). Not shell-escaped — `tokio::process::Command` bypasses the shell.
- [x] Unit test (trait object-safety, compile-only): `let _: Box<dyn LlmProvider> = Box::new(ClaudeCliProvider::from_config(&LlmConfig::default()));`
- [x] Unit test (`from_config`): timeout, model, cli_path are applied correctly (including nested `claude_cli.cli_path`)
- [x] Unix-only integration test (behind `#[cfg(unix)]`, non-ignored — runs in CI): verify `BrokenPipe` variant surfaces when the child closes stdin before the parent finishes writing. The naive fixture `/bin/sh -c "exit 0"` is NOT reliable — short writes land in the 64KB kernel pipe buffer and succeed even after the child exits. Two-part approach:
  1. Fixture: `/bin/sh -c "exec 0<&- ; sleep 0.2"` — child explicitly closes its read end of the pipe, then sleeps. Any subsequent write to that pipe will get `EPIPE` on the next syscall once the kernel notices.
  2. Write a large-enough payload (≥128 KiB of bytes — generate with `"x".repeat(128 * 1024)`) in a loop, not a single call. Short writes that fit in the kernel buffer silently succeed; the payload must exceed the buffer OR the syscall must happen after the kernel has delivered the close.
  3. Assert the error is `LlmError::BrokenPipe(_)`.
  If a deterministic reproduction still proves flaky on macOS across runs, demote this test to `#[ignore]` and keep AC3 row 12 as an opt-in check rather than CI-enforced. Do not leave the test passing by accident — either it reliably hits `BrokenPipe` or it's explicitly opt-in.
- [x] Unix-only integration test (non-ignored): spawn `/bin/sh -c "sleep 5"` with 100ms timeout → verify `Timeout` returned AND that `child.wait()` is called (no zombie — check by completing the test without hanging).

Expected files: `src/core/llm.rs`, `src/core/mod.rs`, `Cargo.toml`

### Step 3: End-to-end integration test (AC1) — DONE 8385d4d

- [x] Add `tests/llm_claude_cli.rs` with:
  - One `#[tokio::test] #[ignore]` end-to-end test that:
    - Checks for `claude` binary via `tokio::process::Command::new("which").arg("claude").status()`. Missing → early `return` with `eprintln!` marker, not a failure.
    - Builds `ClaudeCliProvider::from_config(&LlmConfig::default())` with `timeout_secs = 60`
    - Calls `complete(system = "Respond with exactly the word OK.", prompt = "ping")`
    - Asserts: `Ok(s)`, `!s.trim().is_empty()`, `s.to_lowercase().contains("ok")` within first 40 chars
  - One `#[tokio::test] #[ignore]` **help smoke test** that shells `claude --help` and asserts each flag the provider emits is present as a substring (`"-p"`, `"--output-format"`, `"--no-session-persistence"`, `"--permission-mode"`, `"--tools"`, `"--model"`, `"--system-prompt"`). Catches CLI flag-removal as a contract break.
- [x] Document at the top of the test file why `#[ignore]` is used: these tests require an authenticated `claude` CLI on PATH, incur LLM usage, and are skipped in default `cargo test` runs.
- [x] Run `cargo build`, `cargo clippy`, `cargo test` — all must pass (covers the Step 2 Unix-only integration tests automatically)
- [x] Manually run `cargo test -- --ignored llm_claude_cli` once locally with `claude` authenticated. Record outcome (PASS / FAIL + first line of output) in the step's completion commit message. _(Ran against claude 2.1.112; 2 passed in 7.12s — recorded in commit 8385d4d.)_

Expected files: `tests/llm_claude_cli.rs`

Parallel strategy:
- Step 1 → Step 2 serial (Step 2 imports types from Step 1).
- Step 3 strictly depends on Step 2 (uses `ClaudeCliProvider`).
- Sub-tests within Step 2 are independent and can be written in any order.

## Acceptance Criteria

### AC1: Trait and provider compile, behave correctly end-to-end
- `cargo build --release` succeeds with new `src/core/llm.rs` module exported from `src/core/mod.rs`
- `LlmProvider` is object-safe — the `Box<dyn LlmProvider>` compile-only test passes
- `ClaudeCliProvider::build_command()` produces the exact argv order specified in Step 2 (checked by a unit test)
- A pathological system prompt containing unescaped quotes, newlines, and a trailing backslash is passed as a single argv element (unit-tested — argv length identical to happy-path case)
- Integration test (run with `cargo test -- --ignored llm_claude_cli`, with authenticated `claude` on PATH): returns non-empty stdout, contains "ok" (case-insensitive) within the first 40 characters
- Help smoke test (opt-in): all flags the provider emits are substrings of `claude --help` output

### AC2: Config loads correctly from all sources with nested provider sections
- `MengdieConfig::load()` with no file present returns exact defaults: `provider="claude-cli"`, `model="claude-sonnet-4-6"`, `timeout_secs=120`, `claude_cli.cli_path="claude"`
- TOML containing only `[llm] model = "claude-haiku-4-5"` → returns `model="claude-haiku-4-5"` and all other fields at default, including nested `claude_cli.cli_path="claude"`
- TOML containing only `[llm.claude_cli] cli_path = "/opt/claude/bin/claude"` → returns defaults for `[llm]` and the overridden `cli_path`
- Env map containing `MENGDIE_LLM_MODEL=foo` overrides the file value via `load_with_env`
- Env map containing `MENGDIE_LLM_CLAUDE_CLI_PATH=/x` overrides nested `claude_cli.cli_path`
- No test writes to `std::env` (all env tests pass explicit `HashMap`)
- Malformed TOML produces an `Err` whose `Display` includes the file path

### AC3: Failure modes are classified into distinct variants
For each synthetic input, the pure `classify_output` helper (or the spawn path for `BinaryNotFound`/`Spawn`) returns the matching variant:

| # | Input | Expected variant |
|---|---|---|
| 1 | `io::Error { kind: NotFound }` during `spawn()` | `BinaryNotFound` |
| 2 | `io::Error { kind: PermissionDenied }` during `spawn()` | `Spawn(_)` |
| 3 | `tokio::time::timeout` elapses; child reaped via `start_kill + wait` (no zombie) | `Timeout(_)` |
| 4 | Exit 1, stderr `"Invalid API key"` | `NonZeroExit { code: 1, kind: Auth, .. }` |
| 5 | Exit 1, stderr `"API Error: 429 rate_limit"` | `NonZeroExit { kind: RateLimited, .. }` |
| 6 | Exit 1, stderr `"ECONNRESET"` | `NonZeroExit { kind: Network, .. }` |
| 7 | Exit 1, stderr `"model not found: claude-zzz"` | `NonZeroExit { kind: Model, .. }` |
| 8 | Exit 1, stderr `"weird error"` | `NonZeroExit { kind: Other, .. }` |
| 9 | Exit 0, stdout `""` | `EmptyOutput` |
| 10 | Exit 0, stdout `[0xFF, 0xFE, 0xFD]` | `InvalidUtf8` |
| 11 | Unix: `status.code() = None` (signal exit) | `Signal` |
| 12 | Helper child exits before reading stdin; parent writes prompt bytes | `BrokenPipe(_)` |

All twelve rows must have a corresponding passing test. Rows 1–11 are unit / pure tests where possible; row 12 is the Unix-only integration test in Step 2.

## Non-goals (explicit)

- No streaming API (`complete_stream`, async iterators) — add when a caller needs it.
- No `is_retryable()` on `LlmError` — the `ExitKind` classification is surfaced so BL-007's retry policy has the information it needs, but the policy itself is not in this plan.
- No retry loops, no backoff — caller decides policy.
- No temperature / top-p / max-tokens parameters — not configurable via `claude -p` in a stable way.
- No OpenAI / Codex OAuth provider — trait allows it later (boxed future, `system + prompt` maps to `[system msg, user msg]`), but not in this plan.
- No env sanitization (no clearing of `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_AUTH_TOKEN`) — mengdie is a user-run CLI; trust the user's environment. SmartPal clears these for server isolation, not applicable here.
- No stderr token scrubbing — stderr is only surfaced in `LlmError::NonZeroExit`, which callers handle; not written to logs/telemetry in BL-005.
- No `memory_synthesis_links` schema, no `Synthesis` SourceType variant — those belong to BL-007.
