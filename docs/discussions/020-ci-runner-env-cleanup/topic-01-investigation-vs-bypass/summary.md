---
id: "01"
title: "Verify-then-decide vs. accept-bypass-now"
status: converged
current_round: 3
created: 2026-04-22
decision: "Neither. Direct root-cause fix supersedes both. UAG falsification uncovered `.cargo/config.toml` [env] CFLAGS as the leak; no verification nor bypass needed."
rationale: "During UAG on the Round 2 converged zig-cc-as-CC direction, challenger and minimal-change-engineer independently discovered `.cargo/config.toml` at repo root contains `[env] CFLAGS = \"-isysroot /Applications/.../MacOSX.sdk\"` — added 2026-04-16 commit af303d5. Cargo `[env]` table injects into every build-script subprocess, bypassing all shell-level unsets. Plan 008's 'no `.cargo/config.toml` in the repo' claim was factually wrong (file predated investigation by 1 day). The entire verify-vs-bypass dichotomy was chasing the wrong mechanism."
reversibility: "high"
reversibility_basis: "The fix (delete `.cargo/config.toml` or its [env] block) is a single file change, git-revertable. If the root cause turns out to be additional/different post-fix, reverting is trivial and re-opens investigation with the same analysis tree intact."
---

# Topic: Verify-then-decide vs. accept-bypass-now

## Current Status
**Converged**: the original dichotomy is overtaken by the root-cause discovery. Direct fix supersedes both verify and bypass.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | Split positions: 3 verify-then-decide (architect, codex, gemini), 1 bypass-now (minimal-change), 1 no-position (challenger — 6 challenges). |
| 2 | pending | Convergence on "dry-run IS the verification" (Stage 0 retest → Stage 1 zig-cc-as-CC dry-run). Collapses dichotomy via action. |
| UAG | **OVERTURNED** | challenger + minimal-change independently found `.cargo/config.toml` `[env] CFLAGS = -isysroot ...`. Zig-cc was chasing a ghost; 1-line fix exists. |
| 3 | converged | All 5 agents converge: delete the line (or file). Pre-condition: local `cargo build + cargo test` passes. Fallback: `CFLAGS_x86_64_apple_darwin` per-target scope or user shell env. |

## Context (frozen for record)
The original question presumed bypass-or-verify as the two paths. Reality: direct fix supersedes both.

## Decision Detail
- **What**: delete `.cargo/config.toml` `[env]` block (file contains only 2 lines; deleting the file outright is equivalent).
- **Pre-condition**: local `cargo build` + `cargo test` on Mac mini must pass cleanly without the line. Exercises `libsqlite3-sys` bundle per challenger R3 C3.
- **Fallback A** (if pre-condition breaks): replace line with `CFLAGS_x86_64_apple_darwin = "-isysroot /Applications/..."` (cc-rs per-target var) and `CFLAGS_aarch64_apple_darwin = "-isysroot ..."`. Confirmed functional by codex R3.
- **Fallback B** (if pre-condition breaks and Fallback A is complex): move the CFLAGS export to Kai's `~/.zshenv`. Shell-level, affects only Kai's dev workflow.
- **Post-fix verification**: push a test commit that exercises full ci.yml (clippy + cargo test). Success → cherry-pick reverted commits from plan 008.
