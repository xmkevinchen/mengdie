# Plan 014 — Step Summaries

## Step 1 — `.cargo/config.toml` unscoped CFLAGS → per-target form (commit: 1780198)

**Decisions**:
- Pre-verify removing the file entirely confirmed the CFLAGS line IS needed locally — `ring` build fails with `'TargetConditionals.h' file not found`. Not an orphan config; captures real Apple SDK path need.
- Per-target form (`CFLAGS_x86_64_apple_darwin` + `CFLAGS_aarch64_apple_darwin`) chosen as durable shape. cc-rs reads per-target CFLAGS only when matched target is built; Linux cross-compile sees none.
- Inline heuristic comment preserved at top of file — future debuggers editing the config will see the "grep `.cargo/config.toml` `[env]` first" rule without having to re-learn plan 008 Step 3's mistake.

**Rejected**:
- Delete the file outright (Topic 1 primary per conclusion) — pre-verify showed macOS build fails without it. Would have broken local dev.
- Fallback B (move to `~/.zshenv`) — per-target form is better: keeps env hint in-repo + machine-reproducible for any future contributor.

**Cross-step deps**:
- `.cargo/config.toml` shape is now a contract Step 2's ci.yml relies on: Step 2's workflow omits ALL env manipulation (no `CARGO_BUILD_TARGET`, no `unset SDKROOT/CFLAGS`) because this step's per-target scoping makes the injection target-safe.
- Step 2's cross-check job (`cargo check --target x86_64-unknown-linux-gnu`) is the mechanical regression guard for this file.

**Actual files**: `.cargo/config.toml`
