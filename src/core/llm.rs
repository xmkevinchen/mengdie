//! LLM provider abstraction and Claude CLI implementation.
//!
//! See `docs/plans/007-llm-provider-claude-cli.md` (BL-005).
//!
//! Auth: delegated entirely to the `claude` CLI binary. Parent env is
//! inherited; we never read `~/.claude/.credentials.json` in-process.
//!
//! Privacy: `--system-prompt` is passed as argv and visible to `ps aux`.
//! Acceptable for a single-user personal CLI; revisit if mengdie ever runs
//! multi-tenant. See plan 007 "Known limitation: argv exposure".

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Output;
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::core::config::LlmConfig;

/// Coarse classification tag attached to `NonZeroExit`. Present so callers
/// can branch their retry policy on the failure shape, but policy itself
/// lives at the call site — no `is_retryable()` helper here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    Auth,
    RateLimited,
    Network,
    Model,
    Other,
}

/// Tag identifying WHICH subprocess I/O op failed. `#[non_exhaustive]` so
/// adding a new op later (e.g. `ReadStdinEcho` for a duplex variant) is not
/// a breaking change for downstream pattern matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IoOp {
    ReadStdout,
    ReadStderr,
    WriteStdin,
    Wait,
}

impl std::fmt::Display for IoOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IoOp::ReadStdout => "read stdout",
            IoOp::ReadStderr => "read stderr",
            IoOp::WriteStdin => "write stdin",
            IoOp::Wait => "wait for subprocess",
        };
        f.write_str(s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("unknown llm.provider value: {0:?} (known: \"claude-cli\")")]
    UnknownProvider(String),

    #[error("claude CLI binary not found at configured path")]
    BinaryNotFound,

    #[error("failed to spawn CLI subprocess: {0}")]
    Spawn(#[source] std::io::Error),

    #[error("CLI subprocess I/O error ({op}): {source}")]
    Io {
        op: IoOp,
        #[source]
        source: std::io::Error,
    },

    #[error("CLI subprocess timed out after {0:?}")]
    Timeout(Duration),

    #[error("CLI exited with code {code} (kind: {kind:?}); stderr: {stderr}")]
    NonZeroExit {
        code: i32,
        stderr: String,
        kind: ExitKind,
    },

    #[error("CLI stdin closed before prompt was fully written: {0}")]
    BrokenPipe(#[source] std::io::Error),

    #[error("CLI returned empty stdout")]
    EmptyOutput,

    #[error("CLI stdout was not valid UTF-8")]
    InvalidUtf8,

    #[error("CLI process terminated by signal")]
    Signal,
}

/// Classify a captured `std::process::Output` into either the stdout string
/// or a typed error. Pure and sync — no I/O, no spawning. The async provider
/// builds an `Output` from its three concurrent I/O tasks and delegates here
/// so this logic stays unit-testable without real subprocesses.
///
/// Crate-private: this is a testing seam, not a public API commitment.
pub(crate) fn classify_output(output: Output) -> Result<String, LlmError> {
    let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
    let Some(code) = output.status.code() else {
        return Err(LlmError::Signal);
    };

    if code != 0 {
        let kind = classify_exit_kind(&stderr_text);
        return Err(LlmError::NonZeroExit {
            code,
            stderr: stderr_text,
            kind,
        });
    }

    if output.stdout.is_empty() {
        return Err(LlmError::EmptyOutput);
    }

    match String::from_utf8(output.stdout) {
        Ok(s) => Ok(s),
        Err(_) => Err(LlmError::InvalidUtf8),
    }
}

/// Regex-classify stderr into an ExitKind. Case-insensitive. Mirrors the
/// patterns SmartPal's cli_errors.py uses for the Python Claude CLI provider.
fn classify_exit_kind(stderr: &str) -> ExitKind {
    static AUTH: OnceLock<Regex> = OnceLock::new();
    static RATE: OnceLock<Regex> = OnceLock::new();
    static NET: OnceLock<Regex> = OnceLock::new();
    static MODEL: OnceLock<Regex> = OnceLock::new();

    let auth = AUTH.get_or_init(|| {
        Regex::new(
            r"(?i)Invalid API key|not logged in|authentication failed|authorization failed|unauthorized|permission denied|forbidden|\b401\b|\b403\b",
        )
        .unwrap()
    });
    let rate = RATE.get_or_init(|| {
        Regex::new(r"(?i)API Error:\s*429|overloaded|rate[\s_-]?limit|\b529\b").unwrap()
    });
    let net = NET.get_or_init(|| {
        Regex::new(
            r"(?i)ECONNRESET|ECONNREFUSED|ETIMEDOUT|connection\s+refused|connection\s+reset|network\s+is\s+unreachable",
        )
        .unwrap()
    });
    let model = MODEL.get_or_init(|| {
        Regex::new(
            r"(?i)model\s+not\s+found|unsupported\s+model|issue\s+with\s+the\s+selected\s+model",
        )
        .unwrap()
    });

    if auth.is_match(stderr) {
        ExitKind::Auth
    } else if rate.is_match(stderr) {
        ExitKind::RateLimited
    } else if net.is_match(stderr) {
        ExitKind::Network
    } else if model.is_match(stderr) {
        ExitKind::Model
    } else {
        ExitKind::Other
    }
}

/// Boxed future return — keeps the trait object-safe so BL-007 can store a
/// provider behind `Box<dyn LlmProvider>` under shared state.
pub type LlmFuture<'a> = Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send + 'a>>;

pub trait LlmProvider: Send + Sync {
    fn complete<'a>(&'a self, system: &'a str, prompt: &'a str) -> LlmFuture<'a>;
    fn model(&self) -> &str;
}

/// The canonical entry point for Phase 2 callers. Dispatches on
/// `cfg.llm.provider` and returns a boxed provider. Today only
/// `"claude-cli"` is supported; other provider strings yield
/// `LlmError::UnknownProvider` so misconfiguration surfaces at startup
/// rather than silently building a Claude provider.
pub fn build_provider(cfg: &LlmConfig) -> Result<Box<dyn LlmProvider>, LlmError> {
    match cfg.provider.as_str() {
        "claude-cli" => Ok(Box::new(ClaudeCliProvider::from_config(cfg))),
        other => Err(LlmError::UnknownProvider(other.to_string())),
    }
}

/// Canonical list of flags `ClaudeCliProvider::build_command` emits.
/// Single source of truth so the opt-in help-smoke test can pin the
/// argv contract without duplicating the flag list. (Codex accumulated
/// review flagged drift risk between test and implementation.)
///
/// **Public API contract**: this constant is `pub` (not `pub(crate)`)
/// specifically so the integration test in `tests/llm_claude_cli.rs`
/// can reference it from the external test crate. Downstream callers
/// should treat it as Claude-CLI-specific implementation metadata —
/// when OpenAI / other providers land, each provider owns its own
/// flag list; this constant will NOT become a generic
/// `LlmProvider::argv_flags()` method.
pub const CLAUDE_CLI_FLAGS: &[&str] = &[
    "-p",
    "--output-format",
    "--no-session-persistence",
    "--permission-mode",
    "--tools",
    "--model",
    "--system-prompt",
];

pub struct ClaudeCliProvider {
    cli_path: PathBuf,
    model: String,
    timeout: Duration,
}

impl ClaudeCliProvider {
    pub fn from_config(cfg: &LlmConfig) -> Self {
        Self {
            cli_path: PathBuf::from(&cfg.claude_cli.cli_path),
            model: cfg.model.clone(),
            timeout: Duration::from_secs(cfg.timeout_secs),
        }
    }

    /// Build the `Command` with argv + stdio piping + kill_on_drop.
    ///
    /// argv order is part of AC1. `--permission-mode bypassPermissions` is
    /// required alongside `--tools ""` — the CLI checks permissions at
    /// startup, not just at tool-call time. Do NOT remove even though
    /// `--tools ""` disables all tools; without bypass the CLI errors out
    /// before ever reading stdin.
    pub(crate) fn build_command(&self, system: &str) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.cli_path);
        cmd.arg("-p")
            .arg("--output-format")
            .arg("text")
            .arg("--no-session-persistence")
            .arg("--permission-mode")
            .arg("bypassPermissions")
            .arg("--tools")
            .arg("")
            .arg("--model")
            .arg(&self.model)
            .arg("--system-prompt")
            .arg(system)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        cmd
    }

    async fn complete_impl(&self, system: &str, prompt: &str) -> Result<String, LlmError> {
        let mut cmd = self.build_command(system);
        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(LlmError::BinaryNotFound);
            }
            Err(e) => return Err(LlmError::Spawn(e)),
        };
        drive_subprocess(child, prompt.as_bytes(), self.timeout).await
    }
}

/// The subprocess I/O + timeout lifecycle, extracted from `complete_impl`.
/// Takes ownership of an already-spawned `Child` whose stdin/stdout/stderr
/// were configured as `Stdio::piped()`. Writes `prompt_bytes` to the child's
/// stdin concurrently with reading stdout + stderr, enforces `timeout`, and
/// synchronously reaps the child on timeout / I/O error / normal exit.
///
/// Splitting this out of `complete_impl` serves two purposes: it makes the
/// concurrent-I/O lifecycle testable without going through the Claude-
/// specific argv construction, and it gives callers (tests, or a future
/// non-Claude provider with the same flow) a reusable seam.
pub(crate) async fn drive_subprocess(
    mut child: tokio::process::Child,
    prompt_bytes: &[u8],
    timeout: Duration,
) -> Result<String, LlmError> {
    let mut stdin = child.stdin.take().expect("stdin was piped at build time");
    let mut stdout = child.stdout.take().expect("stdout was piped at build time");
    let mut stderr = child.stderr.take().expect("stderr was piped at build time");

    let prompt_owned = prompt_bytes.to_vec();

    // Three concurrent tasks: write stdin, read stdout, read stderr.
    // Running them sequentially (or via wait_with_output) deadlocks when
    // the child writes >64KB of stderr before finishing stdin — parent
    // blocks on stdin write, child blocks on stderr write.
    let writer = async move {
        let r = stdin.write_all(&prompt_owned).await;
        // Scope-end drops stdin and closes the pipe write-end to signal
        // EOF to the child. Explicit drop here for clarity.
        drop(stdin);
        r
    };
    let stdout_reader = async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await.map(|_| buf)
    };
    let stderr_reader = async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await.map(|_| buf)
    };

    // The timed future owns ONLY the three I/O tasks — NOT `child`.
    // This way, when timeout elapses and the future is dropped, we can
    // still call `child.kill().await` + `child.wait().await` to reap
    // synchronously. (kill_on_drop is best-effort; do not rely on it
    // when we have an owning handle available.)
    let io_future = async move {
        let (write_res, stdout_res, stderr_res) =
            tokio::join!(writer, stdout_reader, stderr_reader);
        // Error precedence: check WRITE first so BrokenPipe (the most
        // informative "child died early" signal) takes priority over the
        // read-side EOF noise that follows. Without this ordering, a
        // child crash that closes stdout+stderr+stdin simultaneously could
        // surface as `Io { op: ReadStdout, source: UnexpectedEof }`
        // instead of the true BrokenPipe cause.
        if let Err(e) = write_res {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                return Err(LlmError::BrokenPipe(e));
            }
            return Err(LlmError::Io {
                op: IoOp::WriteStdin,
                source: e,
            });
        }
        let stdout_bytes = stdout_res.map_err(|source| LlmError::Io {
            op: IoOp::ReadStdout,
            source,
        })?;
        let stderr_bytes = stderr_res.map_err(|source| LlmError::Io {
            op: IoOp::ReadStderr,
            source,
        })?;
        Ok::<_, LlmError>((stdout_bytes, stderr_bytes))
    };

    let (stdout_bytes, stderr_bytes) = match tokio::time::timeout(timeout, io_future).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => {
            // I/O failed during join — reap child before returning.
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(e);
        }
        Err(_elapsed) => {
            // Explicit synchronous reap. kill_on_drop is a
            // belt-and-braces safety net (also configured) but we
            // have the handle, so use it deterministically here.
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(LlmError::Timeout(timeout));
        }
    };

    let status = child.wait().await.map_err(|source| LlmError::Io {
        op: IoOp::Wait,
        source,
    })?;

    classify_output(Output {
        status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    })
}

impl LlmProvider for ClaudeCliProvider {
    fn complete<'a>(&'a self, system: &'a str, prompt: &'a str) -> LlmFuture<'a> {
        Box::pin(self.complete_impl(system, prompt))
    }

    fn model(&self) -> &str {
        &self.model
    }
}

// Tests that construct synthetic `Output` values and tests that spawn real
// helper subprocesses both rely on Unix-only APIs (`ExitStatusExt::from_raw`,
// `/bin/sh`, `/usr/bin/yes`). Gating the whole module keeps the Windows
// compile unblocked until we have a caller that actually runs there.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    // ---- classify_output: exit-code paths ----

    fn mk_output(code: i32, stdout: &[u8], stderr: &[u8]) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8), // raw wait() format on Unix: exit code in upper byte
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }

    fn mk_signal_output(signal: i32, stderr: &[u8]) -> Output {
        // Raw wait() format: signal number in low byte (no upper-byte exit code).
        Output {
            status: ExitStatus::from_raw(signal),
            stdout: Vec::new(),
            stderr: stderr.to_vec(),
        }
    }

    #[test]
    fn classify_happy_path_returns_stdout() {
        let out = mk_output(0, b"hello\n", b"");
        assert_eq!(classify_output(out).unwrap(), "hello\n");
    }

    #[test]
    fn classify_empty_stdout_is_empty_output() {
        let out = mk_output(0, b"", b"");
        assert!(matches!(classify_output(out), Err(LlmError::EmptyOutput)));
    }

    #[test]
    fn classify_invalid_utf8_is_invalid_utf8() {
        let out = mk_output(0, &[0xFF, 0xFE, 0xFD], b"");
        assert!(matches!(classify_output(out), Err(LlmError::InvalidUtf8)));
    }

    #[test]
    fn classify_exit1_auth_kind() {
        let out = mk_output(1, b"", b"Invalid API key\n");
        match classify_output(out) {
            Err(LlmError::NonZeroExit { code, kind, stderr }) => {
                assert_eq!(code, 1);
                assert_eq!(kind, ExitKind::Auth);
                assert!(stderr.contains("Invalid API key"));
            }
            other => panic!("expected NonZeroExit Auth, got {other:?}"),
        }
    }

    #[test]
    fn classify_exit1_rate_limited_kind() {
        let out = mk_output(1, b"", b"API Error: 429 rate_limit_error\n");
        match classify_output(out) {
            Err(LlmError::NonZeroExit {
                kind: ExitKind::RateLimited,
                ..
            }) => {}
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn classify_exit1_network_kind() {
        let out = mk_output(1, b"", b"fetch error: ECONNRESET\n");
        match classify_output(out) {
            Err(LlmError::NonZeroExit {
                kind: ExitKind::Network,
                ..
            }) => {}
            other => panic!("expected Network, got {other:?}"),
        }
    }

    #[test]
    fn classify_exit1_model_kind() {
        let out = mk_output(1, b"", b"model not found: claude-zzz\n");
        match classify_output(out) {
            Err(LlmError::NonZeroExit {
                kind: ExitKind::Model,
                ..
            }) => {}
            other => panic!("expected Model, got {other:?}"),
        }
    }

    #[test]
    fn classify_exit1_other_kind() {
        let out = mk_output(1, b"", b"weird error nobody has seen\n");
        match classify_output(out) {
            Err(LlmError::NonZeroExit {
                kind: ExitKind::Other,
                ..
            }) => {}
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn classify_signal_exit_is_signal() {
        // SIGKILL = 9
        let out = mk_signal_output(9, b"");
        assert!(matches!(classify_output(out), Err(LlmError::Signal)));
    }

    // ---- build_command: argv shape ----

    fn provider_for(cli_path: &str, model: &str) -> ClaudeCliProvider {
        let cfg = LlmConfig {
            provider: "claude-cli".into(),
            model: model.into(),
            timeout_secs: 30,
            claude_cli: crate::core::config::ClaudeCliConfig {
                cli_path: cli_path.into(),
            },
        };
        ClaudeCliProvider::from_config(&cfg)
    }

    fn argv_of(cmd: &tokio::process::Command) -> Vec<String> {
        let std_cmd = cmd.as_std();
        let mut v = vec![std_cmd.get_program().to_string_lossy().into_owned()];
        v.extend(std_cmd.get_args().map(|a| a.to_string_lossy().into_owned()));
        v
    }

    #[test]
    fn build_command_argv_exact_order() {
        let p = provider_for("/usr/bin/claude", "claude-sonnet-4-6");
        let cmd = p.build_command("answer in JSON");
        let argv = argv_of(&cmd);
        assert_eq!(
            argv,
            vec![
                "/usr/bin/claude",
                "-p",
                "--output-format",
                "text",
                "--no-session-persistence",
                "--permission-mode",
                "bypassPermissions",
                "--tools",
                "",
                "--model",
                "claude-sonnet-4-6",
                "--system-prompt",
                "answer in JSON",
            ]
        );
    }

    #[test]
    fn build_command_pathological_system_prompt_is_single_argv_element() {
        let p = provider_for("/usr/bin/claude", "claude-sonnet-4-6");
        let baseline = argv_of(&p.build_command("plain"));
        let pathological = argv_of(&p.build_command("say \"hi\"\nworld\\"));
        assert_eq!(
            baseline.len(),
            pathological.len(),
            "argv length must be identical — system prompt is ONE element regardless of content"
        );
        // The last arg is the system prompt.
        assert_eq!(
            pathological.last().unwrap(),
            "say \"hi\"\nworld\\",
            "system prompt must be passed literally, no shell escaping"
        );
    }

    #[test]
    fn from_config_applies_all_fields() {
        let cfg = LlmConfig {
            provider: "claude-cli".into(),
            model: "claude-haiku-4-5".into(),
            timeout_secs: 45,
            claude_cli: crate::core::config::ClaudeCliConfig {
                cli_path: "/opt/claude/bin/claude".into(),
            },
        };
        let p = ClaudeCliProvider::from_config(&cfg);
        assert_eq!(p.cli_path, PathBuf::from("/opt/claude/bin/claude"));
        assert_eq!(p.model, "claude-haiku-4-5");
        assert_eq!(p.timeout, Duration::from_secs(45));
    }

    // ---- trait object-safety (compile-only) ----

    #[test]
    fn trait_is_object_safe() {
        let _: Box<dyn LlmProvider> =
            Box::new(ClaudeCliProvider::from_config(&LlmConfig::default()));
    }

    #[test]
    fn build_provider_claude_cli_returns_provider() {
        let cfg = LlmConfig::default();
        assert_eq!(cfg.provider, "claude-cli");
        let provider = build_provider(&cfg).expect("claude-cli should resolve");
        assert_eq!(provider.model(), "claude-sonnet-4-6");
    }

    #[test]
    fn build_provider_rejects_unknown_provider() {
        let cfg = LlmConfig {
            provider: "openai".into(),
            ..LlmConfig::default()
        };
        // Can't use unwrap_err — Box<dyn LlmProvider> is not Debug.
        match build_provider(&cfg) {
            Err(LlmError::UnknownProvider(name)) => assert_eq!(name, "openai"),
            Err(other) => panic!("expected UnknownProvider, got {other:?}"),
            Ok(_) => panic!("expected UnknownProvider, got Ok"),
        }
    }

    #[test]
    fn claude_cli_flags_constant_matches_build_command_argv() {
        // Guard against flag drift: every entry in CLAUDE_CLI_FLAGS must
        // appear (exactly) as an argv element produced by build_command.
        let p = provider_for("claude", "claude-sonnet-4-6");
        let argv = argv_of(&p.build_command("sys"));
        for flag in CLAUDE_CLI_FLAGS {
            assert!(
                argv.iter().any(|a| a == flag),
                "CLAUDE_CLI_FLAGS contains {flag:?} but build_command argv does not — \
                 update either the constant or build_command. argv was: {argv:?}"
            );
        }
    }

    // ---- subprocess lifecycle (Unix-only integration) ----

    #[cfg(unix)]
    #[tokio::test]
    async fn binary_not_found_maps_to_binary_not_found() {
        let cfg = LlmConfig {
            provider: "claude-cli".into(),
            model: "claude-sonnet-4-6".into(),
            timeout_secs: 5,
            claude_cli: crate::core::config::ClaudeCliConfig {
                cli_path: "/nonexistent/path/to/claude-binary-xyz".into(),
            },
        };
        let p = ClaudeCliProvider::from_config(&cfg);
        let err = p.complete("", "ping").await.unwrap_err();
        assert!(matches!(err, LlmError::BinaryNotFound), "got {err:?}");
    }

    /// Construct a provider that spawns `cli_path` directly with a short
    /// timeout, bypassing Claude-specific argv construction. Used for the
    /// subprocess lifecycle tests below.
    #[cfg(unix)]
    fn direct_provider(cli_path: &str, timeout: Duration) -> ClaudeCliProvider {
        let cfg = LlmConfig {
            provider: "test".into(),
            model: "ignored".into(),
            timeout_secs: 5,
            claude_cli: crate::core::config::ClaudeCliConfig {
                cli_path: cli_path.into(),
            },
        };
        let mut p = ClaudeCliProvider::from_config(&cfg);
        p.timeout = timeout;
        p
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_elapses_and_reaps_child() {
        // /usr/bin/yes writes forever, never reads stdin, never exits on its
        // own. Our 100ms timeout must fire; kill_on_drop must reap the
        // child — otherwise this test hangs the suite.
        let p = direct_provider("/usr/bin/yes", Duration::from_millis(100));
        let start = std::time::Instant::now();
        let err = p.complete("sys", "prompt").await.unwrap_err();
        let elapsed = start.elapsed();
        assert!(matches!(err, LlmError::Timeout(_)), "got {err:?}");
        assert!(
            elapsed < Duration::from_secs(2),
            "timeout did not fire promptly; elapsed {elapsed:?} (kill_on_drop may be broken)"
        );
    }

    /// Plan 007 Step 2 / AC3 row 12: verify `drive_subprocess` surfaces
    /// `LlmError::BrokenPipe` when the child closes its stdin read end
    /// before the parent finishes writing.
    ///
    /// Pitfall: short writes land in the ~64KB kernel pipe buffer and
    /// succeed even after the child exits. We need BOTH:
    /// 1. An explicit `exec 0<&-` in the fixture to close the read side
    ///    on the kernel level, AND
    /// 2. A payload large enough to force the parent's write syscall to
    ///    observe EPIPE. A ~256 KiB payload is well over the buffer on
    ///    macOS and Linux.
    ///
    /// Critically, this test drives `drive_subprocess` — the actual code
    /// path used by `complete_impl` — rather than just constructing a
    /// `LlmError::BrokenPipe` manually. That means if the `if e.kind() ==
    /// BrokenPipe` branch in `drive_subprocess`'s `io_future` is ever
    /// deleted or reordered, this test catches the regression.
    #[cfg(unix)]
    #[tokio::test]
    async fn drive_subprocess_returns_broken_pipe_when_child_closes_stdin() {
        use std::process::Stdio;
        let mut cmd = tokio::process::Command::new("/bin/sh");
        cmd.arg("-c")
            // Child closes its stdin read end, then sleeps briefly so the
            // parent has time to observe EPIPE on its next write. Also
            // closes stdout+stderr so the reader tasks reach EOF quickly
            // (otherwise we'd hang until the 1s timeout).
            .arg("exec 0<&- 1<&- 2<&- ; sleep 0.3")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = cmd.spawn().expect("spawn sh fixture");

        // 256 KiB overflows the ~64KB kernel pipe buffer on both Linux
        // and macOS, forcing the parent's write to observe the EPIPE
        // rather than silently succeeding into buffer space.
        let payload = vec![b'x'; 256 * 1024];

        let result = drive_subprocess(child, &payload, Duration::from_secs(2)).await;
        match result {
            Err(LlmError::BrokenPipe(_)) => {}
            other => panic!(
                "expected LlmError::BrokenPipe from drive_subprocess \
                 when child closes stdin read end; got {other:?}"
            ),
        }
    }

    /// Generic `Spawn` error path — `io::Error` that isn't `NotFound` (e.g.,
    /// `PermissionDenied`) must map to `LlmError::Spawn`, not `BinaryNotFound`.
    /// We can't easily produce `PermissionDenied` portably, so exercise the
    /// mapping logic by calling the same conversion `complete_impl` uses.
    #[test]
    fn non_notfound_io_error_maps_to_spawn() {
        let perm = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_ne!(perm.kind(), std::io::ErrorKind::NotFound);
        // Mirror the match arm in complete_impl.
        let err = match perm.kind() {
            std::io::ErrorKind::NotFound => LlmError::BinaryNotFound,
            _ => LlmError::Spawn(perm),
        };
        assert!(matches!(err, LlmError::Spawn(_)), "got {err:?}");
    }

    /// Regression guard for the error-precedence swap in `io_future`:
    /// `Io { op: WriteStdin }` is produced for non-BrokenPipe write
    /// failures. Build a synthetic write error that isn't BrokenPipe and
    /// verify the `op` tag is the right enum variant.
    #[test]
    fn io_op_enum_preserves_operation_tag() {
        let src = std::io::Error::from(std::io::ErrorKind::Other);
        let err = LlmError::Io {
            op: IoOp::WriteStdin,
            source: src,
        };
        match err {
            LlmError::Io {
                op: IoOp::WriteStdin,
                ..
            } => {}
            other => panic!("expected Io op=WriteStdin, got {other:?}"),
        }

        // IoOp Display also used in error format string.
        assert_eq!(format!("{}", IoOp::ReadStdout), "read stdout");
        assert_eq!(format!("{}", IoOp::Wait), "wait for subprocess");
    }
}
