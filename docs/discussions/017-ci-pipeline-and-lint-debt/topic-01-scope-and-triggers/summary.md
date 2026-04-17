---
id: "01"
title: "CI scope and triggers"
status: converged
current_round: 1
created: 2026-04-17
decision: "New .forgejo/workflows/ci.yml on push (all branches) + pull_request. Single serial job with `rustup show` as first step (forces toolchain resolution, fails loudly if rustup missing), then fmt --check → clippy --all-targets -D warnings → test. Host Rust via source ~/.cargo/env. Cache ~/.cargo/{registry,git}, target/, ~/.cache/fastembed keyed on Cargo.lock + rust-toolchain.toml. Bare actions/checkout@v4 syntax matching release.yml. Follow-on: after ci.yml is green, drop the redundant `test:` job from release.yml (Doodlestein strategic)."
rationale: "Solo-dev + single runner = fail-fast serial beats parallel. Fastembed cache is the single biggest cold-start cost (~90MB). Host mode confirmed (no Docker). Bare action syntax works today via DEFAULT_ACTIONS_URL. cargo audit + pre-commit hooks deferred to v2."
reversibility: "high"
reversibility_basis: "Single YAML file, no stateful migration. Splitting jobs, adding audit, switching to full-URL refs are additive."
---

# Topic: CI scope and triggers

## Current Status
Converged Round 1 (after Round 2 tension resolution on runner env + action URL syntax).

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | converged | ci.yml separate from release.yml, push + PR triggers, single serial job, host Rust, cache fastembed |

## Context
See index.md. CI today exists only for tag-based release; no push-time gate.

## Constraints
- Forgejo runner in host mode (no Docker)
- Self-hosted single runner
- `#[ignore]` tests (e2e.rs, llm_claude_cli.rs) must not fail CI

## Key Questions — resolved
- Which triggers: push (all branches) + pull_request
- Which jobs: fmt, clippy, test in one serial job
- Cache: registry, git, target, fastembed
- Coexistence with release.yml: no collision (different triggers)
- Pre-commit hooks: out of scope
