---
round: 01
date: 2026-04-17
score: converged
---

# Round 01 — Lint debt strategy

## Discussion

### Architect (Round 1)
Big-bang cleanup PR first, then `-D warnings` from day 1. All 14 warning sites (6 files) are mechanical — 30–45 min estimated for the batch. Starting with `-W warnings` (non-blocking) path rejected — permanent background noise, never gets fixed. `rust-toolchain.toml` pin channel `stable`. `cargo fmt --check` (no auto-format). No project-wide `#![allow(...)]` suppressions.

### Rust-archaeologist (Round 1)
Exact inventory (file:line indexed):
- **1 hard ERROR**: `approx_constant` at `embeddings.rs:116` (`3.14159` → `std::f32::consts::PI`). Deny-by-default in clippy; blocks `cargo clippy --all-targets`. `cargo check --all-targets` passes, but `cargo clippy --all-targets` fails.
- **6 trivial warnings**: redundant_closure (db.rs ×2), collapsible_if (schema.rs), manual_clamp (search.rs), len_zero (search.rs), print_literal (cli.rs), manual_is_multiple_of (embeddings.rs)
- **2 needs-thought warnings**: manual_strip ×4 (project.rs — TOML parse loop), collapsible_match ×2 (cli.rs — error chain walking)

Total: 10 distinct fix sites (duplicates counted once). No rustfmt.toml, no rust-toolchain.toml, no .clippy.toml today.

### Codex (Round 1)
Rust ecosystem consensus: `cargo clippy -- -D warnings` from day 1 is norm. rust-analyzer uses selective denies (dbg_macro, todo, print_stdout/stderr); tokio pins Clippy version + avoids `--all-features`. Solo dev with 14 warnings: big-bang cleanup is low-risk (no review bottleneck, tests green). `rust-toolchain.toml` strongly recommended — pin specific version, not `stable` (which floats every 6 weeks and can introduce new deny-default lints on unchanged code).

### Round 2 tension resolution

**Toolchain pin version**: architect initially said `channel = "stable"`, codex pushed for exact version pin. Architect converged in Round 2: `1.94.1` (current local verified Rust). Damage ratio argument: floating stable with 14 existing warnings means every rustc release could add unrelated new clippy noise that breaks CI; pinned only breaks on intentional upgrade. Solo dev = deliberate cadence.

**Clippy scope**: `--all-targets` vs `--lib --tests`. Archaeologist's ground truth — `approx_constant` is a hard error under `--all-targets` via clippy's built-in deny. Running `--lib --tests` would hide bin-target warnings (cli.rs has 2 sites). All three agents converged on `--all-targets` + fix approx_constant in the cleanup PR.

## Outcome

- Score: **converged**
- **Decision**:
  1. Big-bang cleanup PR addressing all 10 clippy items identified by archaeologist (1 hard error + 6 trivial + 2 needs-thought sites with duplicates counted once = 14 lines affected total). Must keep tests green.
  2. Add `rust-toolchain.toml` pinning `channel = "1.94.1"`, `profile = "minimal"`, components `rustfmt, clippy`.
  3. CI `ci.yml` enforces `cargo clippy --all-targets -- -D warnings` from day 1.
  4. `cargo fmt --all -- --check` (check-only, no auto-format). No `rustfmt.toml` unless specific style opinions emerge later.
  5. No project-wide `#![allow(...)]` suppressions. Case-by-case `#[allow(...)]` on genuine false positives only.
- **Rationale**: Starting with non-blocking warnings (`-W`) leads to permanent background noise (codex's ecosystem evidence, architect's judgment). Big-bang cleanup is cheap on solo-dev (no review bottleneck). Pinning exact version matches the "deliberate update cadence" posture of this project. `--all-targets` is the minimum scope that gives honest build health — anything less is lying about the state.
- **Reversibility**: high
- **Reversibility basis**: All changes in-file + in-YAML. Relaxing the gate to `-W warnings` or adding allow-lists is trivial. Toolchain bump is a one-line edit. Reverting cleanup commits is standard git.
