//! End-to-end integration tests for `ClaudeCliProvider`.
//!
//! These tests are `#[ignore]` by default. They require:
//! - An authenticated `claude` CLI on `$PATH` (mengdie does not read
//!   credentials itself — see plan 007 auth decision).
//! - Network access to Anthropic's API.
//! - Real LLM usage (non-zero cost / rate-limit budget).
//!
//! Run them explicitly:
//!     cargo test --test llm_claude_cli -- --ignored
//!
//! Unit tests in `src/core/llm.rs` cover the subprocess lifecycle and
//! error classification paths using pure `Output` fixtures and Unix-only
//! helper binaries (`/usr/bin/yes`, `/bin/sh`). Those run on every
//! `cargo test`. This file is reserved for the two checks that require
//! the real Claude CLI: an end-to-end `complete()` call, and a smoke
//! test pinning the CLI flag contract.

use tokio::process::Command;

use mengdie::core::config::{ClaudeCliConfig, LlmConfig};
use mengdie::core::llm::{
    ClaudeCliProvider, LlmProvider, CLAUDE_CLI_FLAGS, CLAUDE_CLI_STRUCTURED_FLAGS,
};

/// Returns true if `claude` binary is discoverable on PATH.
async fn claude_on_path() -> bool {
    Command::new("which")
        .arg("claude")
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

#[tokio::test]
#[ignore = "requires authenticated claude CLI on PATH; run with --ignored"]
async fn end_to_end_complete_returns_ok() {
    if !claude_on_path().await {
        eprintln!("[SKIP] `claude` binary not found on PATH; skipping e2e test.");
        return;
    }

    let cfg = LlmConfig {
        provider: "claude-cli".into(),
        model: "claude-sonnet-4-6".into(),
        timeout_secs: 60,
        claude_cli: ClaudeCliConfig {
            cli_path: "claude".into(),
        },
    };
    let provider = ClaudeCliProvider::from_config(&cfg);

    let result = provider
        .complete("Respond with exactly the word OK.", "ping")
        .await;

    let response = result.expect("complete() returned Err; is claude CLI authenticated?");
    assert!(
        !response.trim().is_empty(),
        "response must be non-empty, got {response:?}"
    );
    let head: String = response.chars().take(40).collect();
    assert!(
        head.to_lowercase().contains("ok"),
        "expected 'ok' (case-insensitive) in first 40 chars; got {head:?}"
    );
}

/// Pins the set of CLI flags `ClaudeCliProvider::build_command` emits.
/// If a future `claude` release drops or renames any of these flags,
/// this test fails loudly instead of the failure showing up at runtime
/// inside `complete()` with a confusing error message.
#[tokio::test]
#[ignore = "requires claude CLI on PATH; run with --ignored"]
async fn help_output_contains_all_flags_we_emit() {
    if !claude_on_path().await {
        eprintln!("[SKIP] `claude` binary not found on PATH; skipping help smoke test.");
        return;
    }

    let output = Command::new("claude")
        .arg("--help")
        .output()
        .await
        .expect("failed to run `claude --help`");
    assert!(
        output.status.success(),
        "`claude --help` exited non-zero: {:?}",
        output.status
    );
    let help_text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);

    // Single source of truth: src/core/llm.rs::CLAUDE_CLI_FLAGS. If
    // build_command ever drops or renames a flag, update that constant
    // in one place and this test stays in sync.
    for flag in CLAUDE_CLI_FLAGS {
        assert!(
            help_text.contains(flag),
            "`claude --help` output does not contain flag {flag:?} — \
             build_command argv contract may be broken. Full output follows:\n{help_text}"
        );
    }
}

/// Plan 019: parallel of `help_output_contains_all_flags_we_emit` for
/// the structured-output path. `--json-schema` is required by
/// `build_structured_command` and was first available in claude-CLI
/// 2.1.138 (per BL-027 verification + dep-analyst plan-review finding).
/// Operators on older versions get a clear "flag missing" diagnostic
/// from this test instead of the runtime `StructuredOutputWrapperInvalid`
/// surface (which carries a version-hint suffix anyway, but proactive
/// detection beats reactive).
#[tokio::test]
#[ignore = "requires claude CLI >= 2.1.138 on PATH; run with --ignored"]
async fn help_output_contains_all_structured_flags_we_emit() {
    if !claude_on_path().await {
        eprintln!("[SKIP] `claude` binary not found on PATH; skipping structured help smoke test.");
        return;
    }

    let output = Command::new("claude")
        .arg("--help")
        .output()
        .await
        .expect("failed to run `claude --help`");
    assert!(
        output.status.success(),
        "`claude --help` exited non-zero: {:?}",
        output.status
    );
    let help_text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);

    for flag in CLAUDE_CLI_STRUCTURED_FLAGS {
        assert!(
            help_text.contains(flag),
            "`claude --help` does not contain {flag:?} — \
             upgrade claude to >= 2.1.138 (per plan 019 / BL-027). \
             Full output follows:\n{help_text}"
        );
    }
}
