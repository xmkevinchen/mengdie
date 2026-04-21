---
id: "019"
title: "Exponential Decay for Dreaming (BL-008)"
status: done
created: 2026-04-20
pipeline:
  analyze: skipped
  discuss: done
  plan: done
  work: done
plan: "docs/plans/013-exponential-decay.md"
tags: [dreaming, decay, exponential-decay, promotion, demotion, long-term-memory, bl-008]
---

# Exponential Decay for Dreaming (BL-008)

> **Naming note (2026-04-20)**: Originally titled "Power-Law Decay" per
> BL-008's sketch. Renamed to "Exponential Decay" at Round 2 convergence —
> `0.95^days` and `2^(-d/H)` are both exponential families, not power-laws.
> Directory name retained as `019-power-law-decay/` for stability (commit
> URL compatibility); frontmatter + titles reflect the correct term.

Source backlog item: `docs/backlog/005-phase2-roadmap.md` → Phase 2.1 → BL-008
(`effective_relevance = avg_relevance × 0.95^days` at promotion/demotion time;
demotion when `effective < 0.01`; NEVER mutate stored `avg_relevance`;
independent of LLM — no dependency gate).

## Problem Statement
See [framing.md](framing.md).

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Decay formula & constants | [topic-01-decay-formula/](topic-01-decay-formula/) | converged | `effective = avg × 2^(-d/60)`, floor=0.20, last_recalled only |
| 2 | Computation location | [topic-02-computation-location/](topic-02-computation-location/) | converged | Hybrid: Dreaming pass + search post-fetch, both in Rust, same-age-clock |
| 3 | Demotion semantics & threshold | [topic-03-demotion-semantics/](topic-03-demotion-semantics/) | converged | Asymmetric; demote if effective<0.20; clear is_longterm; skip NULL-recall memories |
| 4 | Interaction with existing promotion | [topic-04-promotion-interaction/](topic-04-promotion-interaction/) | converged | Promotion predicate UNCHANGED; narrow scope — staleness only, not burst bias |
| 5 | Observability & testing strategy | [topic-05-observability-testing/](topic-05-observability-testing/) | converged | `Option<DateTime<Utc>>` clock, 3 new counters, `--dry-run-decay` flag, cliff-comments |

## Documents
- [Framing](framing.md)
- [Prior Art from Mengdie](prior-art.md)
- [Round 1 synthesis](round-01/synthesis.md)
- [Round 2 synthesis](round-02/synthesis.md)
- [Conclusion](conclusion.md)
