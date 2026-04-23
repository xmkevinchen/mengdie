---
id: "020"
title: "CI Runner Env Cleanup — root-cause or bypass the `-isysroot` leak"
status: active
created: 2026-04-22
pipeline:
  analyze: done
  discuss: done
  plan: done
  work: pending
plan: "docs/plans/014-ci-runner-env-fix.md"
tags: [ci, forgejo, act, ring, cc-rs, runner-env, isysroot, cross-compile]
---

# CI Runner Env Cleanup

Root-cause the act-spawned `-isysroot` leak on `ckai-macmini.local` host-mode
Forgejo runner that blocked plan 008 Step 3, OR pick a bypass that unblocks
`BL-ci-full-clippy-test` (full clippy + `cargo test` on PR).

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Verify-then-decide vs. accept-bypass-now | [topic-01-investigation-vs-bypass/](topic-01-investigation-vs-bypass/) | converged | Neither — direct root-cause fix (`.cargo/config.toml` `[env]`) |
| 2 | Bypass mechanism selection (contingent) | [topic-02-bypass-mechanism/](topic-02-bypass-mechanism/) | converged | Does not activate — Topic 1 selected direct fix |

Topic table collapsed from 4 to 2 per Round 0 v1 feedback (minimal-change-engineer): scope-boundary between `006`/`BL` falls out of Topic 1's decision; `release.yml` race is a drive-by fix unrelated to the core decision.

## Documents
- [Framing](framing.md)
- [Analysis](analysis.md)
- [Conclusion](conclusion.md)

## Origin Context

Plan 008 Step 3 shipped fmt-only CI after ~2 days of debugging failed to
reproduce the ring `-isysroot` error in isolation. Sprint v0.8.0 bundles
`006-ci-runner-env-cleanup` (M, 3 pt) and `BL-ci-full-clippy-test` (L, 5 pt)
— this analysis decides whether to root-cause or bypass.

Related items:
- `.ae/backlog/v0.8.0/006-ci-runner-env-cleanup.md` — full evidence dump
  from plan 008 Step 3 investigation
- `.ae/backlog/v0.8.0/BL-ci-full-clippy-test.md` — fix options A/B/C
- Reverted commits `e4b8cbf` through `6658248` — cherry-pickable once
  the env issue is resolved
