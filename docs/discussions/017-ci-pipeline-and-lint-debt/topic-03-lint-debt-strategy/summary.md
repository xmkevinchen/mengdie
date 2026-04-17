---
id: "03"
title: "Lint debt strategy"
status: converged
current_round: 1
created: 2026-04-17
decision: "Big-bang cleanup PR first addressing all 10 clippy items (1 hard error + 6 trivial + 2 needs-thought, 14 lines affected total), including approx_constant at embeddings.rs:116. Then add rust-toolchain.toml pinning 1.94.1 with profile=minimal and rustfmt+clippy components. ci.yml adds `rustup show` as first step (forces rustup toolchain-file resolution + belt-and-braces guard against the untested system-Rust-only fallback). Then cargo clippy --all-targets -- -D warnings enforced day 1. cargo fmt --check (no auto-format). No project-wide #![allow(...)]; case-by-case allow only for genuine false positives. Known risk (Doodlestein regret): agent-written code may trip existing deny-by-default lints and block PRs; mitigation is case-by-case #[allow(...)] on real false positives. Exact-version pin prevents the new-lint-every-6-weeks surprise class."
rationale: "All 10 items are mechanical (30–45 min batch). Non-blocking -W warnings leads to permanent background noise (codex ecosystem evidence). -D warnings from day 1 forces the cleanup first. Exact-version pin prevents silent 6-week-release clippy surprises. --all-targets is the minimum scope for honest build health — anything less hides bin-target warnings."
reversibility: "high"
reversibility_basis: "All changes in-file + in-YAML. Relaxing the gate or adding allow-lists is trivial. Toolchain bump is a one-line edit. Reverting cleanup commits is standard git."
---

# Topic: Lint debt strategy

## Current Status
Converged Round 1 (after Round 2 tension resolution on exact-pin vs channel + --all-targets vs --lib).

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | converged | Big-bang cleanup + pin 1.94.1 + -D warnings + --all-targets from day 1 |

## Context
See index.md. 14 warning sites across 6 files accumulated over months; no CI gate.

## Constraints
- Tests currently green
- No rustfmt.toml / rust-toolchain.toml / .clippy.toml exists
- approx_constant at embeddings.rs:116 is a hard error under --all-targets (clippy deny-by-default)

## Key Questions — resolved
- Big-bang vs incremental: big-bang (solo dev, no review bottleneck)
- -W vs -D warnings: -D from day 1, fix pile first
- rust-toolchain.toml: yes, pin exact 1.94.1 + components
- Project-wide #![allow(...)]: no
- rustfmt config: check-only, no rustfmt.toml unless specific opinion emerges
