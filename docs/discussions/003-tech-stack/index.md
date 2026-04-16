---
id: "003"
title: "Tech Stack Selection"
status: done
created: 2026-04-04
pipeline:
  analyze: skipped
  discuss: done
  plan: done
  work: done
plan: "docs/plans/001-mvp-phase1.md"
tags: [tech-stack, language, architecture]
---

# Tech Stack Selection

## Problem Statement

The Second Brain project needs a finalized tech stack decision. Discussion 002 (MVP Phase 1) assumed TypeScript to match the Claude Code / OpenClaw ecosystem, but this was a brainstorming suggestion — not a deliberate, evidence-based choice. The project's core components (MCP server via stdio, SQLite with FTS5 + vector search, filesystem watcher, CLI for batch import) each have different ecosystem strengths. We need to evaluate language/runtime options against the actual requirements before committing.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Language & runtime choice | [topic-01-language-choice/](topic-01-language-choice/) | converged | **Rust** — agent-centric reframe overturned TypeScript (4-0 unanimous) |

## Documents
- [Conclusion](conclusion.md)
- Prior context: [002 MVP Phase 1 Conclusion](../002-mvp-phase1/conclusion.md)
