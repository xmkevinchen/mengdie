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

## Step 2 — rewrite ci.yml fresh with 4 jobs (commit: fc8114e)

**Decisions**:
- Wrote fresh file from scratch (44 lines → 65 lines), not cherry-picked from reverted commits `e4b8cbf`..`6658248`. Reason: those commits were written against the falsified Apple-Clang hypothesis and carried dead scaffolding (`CARGO_BUILD_TARGET` exports, `unset SDKROOT`, `CC_x86_64_unknown_linux_gnu`, `env | sort`). Fresh file is simpler and matches the actual root cause.
- Added cross-check job (`cargo check --target x86_64-unknown-linux-gnu`) as mechanical regression guard per strategic-Doodlestein. Compile-phase only — no Linux linker needed on macOS host.
- Kept `source ~/.cargo/env` per-job idiom (plan 008 host-mode runner pattern; matches release.yml).

**Rejected**:
- `container: rust:latest` directive (discussion 017 memory: host-mode runner, `container:` unavailable).
- Cherry-picking reverted commits (Doodlestein adversarial on plan: would land dead scaffolding).
- Matrix strategy for target variants (out of scope — discussion 017 decided Linux-x86_64 only target).

**Cross-step deps**:
- Step 4's verification reads all 4 job logs for `running:.*isysroot` — relies on job names being `fmt`, `clippy`, `test`, `cross-check` (or similar).
- Step 1's `.cargo/config.toml` per-target scoping is what makes the "no env manipulation" constraint achievable; any reversion of Step 1 breaks this step's assumption.

**Actual files**: `.forgejo/workflows/ci.yml`

## Step 3 — release.yml build-linux needs: [test] (commit: ebc44a7)

**Decisions**:
- Used `needs: [test]` instead of inlining fmt/clippy/test into build-linux. Rationale: modular jobs simpler than one mega-job; no duplication with ci.yml's equivalent jobs (different triggers — ci runs on push/PR, release runs on tag).
- Kept release.yml otherwise unchanged — strictly the race fix.

**Rejected**:
- Inlining the gate into build-linux (more churn, duplicates ci.yml).
- Making test depend on build-linux (inverse order — misses the point; tests should gate release, not the reverse).

**Cross-step deps**:
- None downstream. This is an independent drive-by fix.

**Actual files**: `.forgejo/workflows/release.yml`
