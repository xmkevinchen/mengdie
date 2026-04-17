---
id: "017"
title: "CI Pipeline + Lint Debt Cleanup"
status: active
created: 2026-04-17
pipeline:
  analyze: skipped
  discuss: done
  plan: done
  work: pending
plan: "docs/plans/008-ci-pipeline-and-lint-debt.md"
tags: [ci, forgejo, clippy, lint-debt, project-hygiene]
---

# CI Pipeline + Lint Debt Cleanup

## Problem Statement

After plan 007 (BL-005 LLM provider) shipped, we noticed:

1. **No CI for PR-time checks.** The repo has `.forgejo/workflows/release.yml` for tag-based release builds, but no workflow runs `cargo build + cargo test + cargo clippy` on push or PR. Regressions can land silently — we're relying on manual `cargo test` discipline per commit.

2. **Lint debt has accumulated.** Running `cargo clippy --lib --tests` surfaces 9+ pre-existing warnings spread across six files — `src/core/embeddings.rs`, `project.rs`, `db.rs`, `search.rs`, `schema.rs`, `bin/cli.rs`. Specific items include `3.14159` literal (should be `f64::consts::PI`), manual `is_multiple_of`, `.len() > 0` (should be `!is_empty()`), redundant closures, `clamp`-like patterns, collapsible `if let`, manual prefix-stripping, and an approx-PI constant. None break the build; none have been cleaned because `cargo clippy` is not gated.

3. **Review P3s want CI support.** `/ae:review` of plan 007 deferred three items to "next feature cycle = CI + lint debt": fixture-binary test for `CLAUDE_CLI_FLAGS`, regex false-positive counterexamples, stderr token scrubbing. All become easier with CI to run them.

This discussion decides the shape of the CI workflow(s), what platform matrix we actually cover, and how aggressive we are about the clippy debt.

## Infrastructure Context

- **Forgejo** v11.0.11 at `ckai-macmini.local:3300` (Ubuntu x86_64 server, NOT a Mac despite the hostname). Repo is `ssh://git@ckai-macmini.local:2222/ckai/mengdie.git`.
- **Forgejo Actions runner**: `forgejo-runner` v6.3.1, host mode, label `ubuntu-latest`, systemd user service already enabled.
- **Cross-compilation constraints**:
  - Linux x86_64: native on Ubuntu runner ✅
  - macOS arm64 from Linux: ❌ blocked by macOS framework deps (CoreFoundation, fsevent-sys). `cargo-zigbuild` installed but insufficient without Apple SDK.
  - macOS arm64 native: build locally on the Mac mini only.
- **fastembed** uses `ort-download-binaries-rustls-tls` + `hf-hub-rustls-tls` (no native-tls), which avoids Security.framework on macOS but pulls ring + ONNX runtime shared libs. First-run downloads ~90MB model to `~/.cache/fastembed/` — CI impact TBD.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | CI scope and triggers | [topic-01-scope-and-triggers/](topic-01-scope-and-triggers/) | converged | New ci.yml on push+PR, single serial job, `rustup show` first, host Rust, cache fastembed; follow-on: drop release.yml test job |
| 2 | Platform matrix | [topic-02-platform-matrix/](topic-02-platform-matrix/) | converged | Linux x86_64 only, documented manual macOS ritual |
| 3 | Lint debt strategy | [topic-03-lint-debt-strategy/](topic-03-lint-debt-strategy/) | converged | Big-bang cleanup → pin 1.94.1 profile=minimal → -D warnings --all-targets day 1 |

## Documents
- [Conclusion](conclusion.md)
