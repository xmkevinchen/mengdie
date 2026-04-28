---
id: "026"
title: "v0.0.1 Step A2 — Rust open-source library survey"
status: active
created: 2026-04-27
pipeline:
  analyze: done
  discuss: pending
  plan: pending
  work: pending
plan: ""
parent_plan: "docs/v0.0.1-rebuild-plan.md"
tags: [v0.0.1, survey, rust-ecosystem, rag-libraries, vector-stores, embedding-libs]
---

# v0.0.1 Step A2 — Rust open-source library survey

Survey mature Rust libraries that mengdie's v0.x reinvented. Per
candidate library: scope, maturity (releases, contributors, last
commit), license, idiomatic API surface, overlap with mengdie's
current modules, solo-project adoption cost (build complexity,
runtime deps, learning curve), abandonment risk.

Initial candidate list (from v0.0.1-rebuild-plan.md):
- swiftide (RAG framework)
- rig (LLM agent framework)
- Qdrant (vector DB)
- LanceDB (embedded vector DB)
- sqlite-vec (SQLite vector extension)
- Tantivy (Rust full-text search, FTS5 alternative)

Survey may add adjacent candidates discovered during research
(e.g., async-openai, candle for local inference, fastembed-rs
which mengdie already uses).

Step A2 of the v0.0.1 redesign migration outline. Pairs with A1
(mengdie functional inventory) — together they feed Step B
(integration strategy `/ae:discuss`).

## Topics
*Created by `/ae:discuss`*

## Documents
- [Analysis](analysis.md)
