---
id: "028"
title: "v0.0.1 architecture design — layering, traits, schema, 4 open decisions"
status: concluded
created: 2026-04-27
concluded: 2026-04-28
pipeline:
  analyze: done
  discuss: done
  plan: pending
  work: pending
plan: ""
parent_plan: "docs/v0.0.1-rebuild-plan.md"
parent_blueprint: "docs/blueprint.md"
tags: [v0.0.1, architecture, module-layering, trait-boundaries, schema, instrumentation]
---

# v0.0.1 architecture design — layering, traits, schema, 4 open decisions

Architecture sits between blueprint (`docs/blueprint.md`, what
mengdie is) and per-feature plans (BLs, how to ship X). This
discussion derives the v0.0.1 module architecture starting from
v0.8.0's actual structure (per `docs/discussions/025-functional-inventory/analysis.md`)
and adjusting it to fit blueprint v0.2.

## Sub-questions

The TL's chat draft proposed:
- 6 layers (Storage / Ingestion / Retrieval / Reflection / LLM Provider / External Interface) + cross-cutting
- 6 trait abstractions (`Storage`, `LlmProvider`, `EmbeddingProvider`, `Reflector`, `Transport`, `EventEmitter`)
- 4 new modules / extensions (Instrumentation, bi-temporal schema, bidirectional update, AE Round-0 caller)
- 4 specific open decisions:
  1. Delete persisted `metrics` SQLite table → in-process tracing + AtomicU64
  2. Demote `watcher.rs` to opt-in fallback (push as default ingest)
  3. Bidirectional update timing — synchronous vs asynchronous
  4. `Reflector` default trigger — pure count threshold vs count + cron escape vs composite (SCM)

This discussion validates / challenges all of the above with team
review against blueprint, code, and industry findings.

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Storage trait + search-split refactor scope | [topic-01-storage-trait-search-split/](topic-01-storage-trait-search-split/) | converged | search-split YES; trait NO; free functions over `&Db` |
| 2 | Bi-temporal event_time vs ingested_at column | [topic-02-bi-temporal-event-time/](topic-02-bi-temporal-event-time/) | converged | REJECT permanently; optional `valid_from` parameter alternative |
| 3 | Reflection collapse + Reflector trait | [topic-03-reflection-collapse/](topic-03-reflection-collapse/) | converged (UAG) | defer collapse pending sqlite-vec; Reflector trait NO regardless |
| 4 | A-MEM bidirectional update deferral trigger | [topic-04-amem-bidirectional-trigger/](topic-04-amem-bidirectional-trigger/) | converged | defer; corpus + audit-log supersession trigger |

## Documents
- [Framing](framing.md)
- [Analysis](analysis.md)
- [Conclusion](conclusion.md) *(after Doodlestein review)*
