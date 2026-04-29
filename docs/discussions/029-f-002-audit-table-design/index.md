---
id: "029"
title: "F-002 audit table design — 5 plan-time decisions"
status: concluded
created: 2026-04-28
concluded: 2026-04-28
pipeline:
  analyze: skipped
  discuss: done
  plan: done
  work: done
plan: ".ae/features/active/F-002-persisted-domain-audit-table-audit-retur/plan.md"
parent_feature: ".ae/features/active/F-002-persisted-domain-audit-table-audit-retur/"
parent_blueprint: "docs/blueprint.md"
tags: [v0.0.1, audit-table, schema-v6, p0-instrumentation, fts-fallback, on-delete-policy, caller-kind, failure-mode, read-path]
---

# F-002 audit table design — 5 plan-time decisions

Discussion converging on 5 open decisions surfaced by F-002's `/ae:analyze` phase
(see `.ae/features/active/F-002-persisted-domain-audit-table-audit-retur/analysis.md`).
The link-table schema shape is settled by discussion 028 Topic 4; this discussion
resolves the remaining hook-placement and contract decisions before plan-time.

## Problem Statement

See [framing.md](framing.md).

## Topics

Round 0 (2026-04-28) reframed the original 5 topics into 2 active topics + 3
pre-discussion YAGNI decisions. See [framing.md](framing.md) for the
"Decided pre-discussion" section.

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | Audit hook placement and coverage scope | [topic-01-hook-placement/](topic-01-hook-placement/) | converged | Option B — `mcp_tools.rs` call + `Db::record_search_audit` helper |
| 2 | Audit-write failure mode contract (depends on T1) | [topic-02-failure-mode/](topic-02-failure-mode/) | converged | Best-effort + warn + `METRIC_AUDIT_WRITE_FAILURES` |

## Documents

- [Framing](framing.md) — problem statement + Round 0 verdict
- [F-002 analysis](../../../.ae/features/active/F-002-persisted-domain-audit-table-audit-retur/analysis.md) — 5-reviewer analyze output (precondition for this discussion)
- [Discussion 028 conclusion](../028-v0.0.1-architecture-design/conclusion.md) — settles link-table shape; defines Wave 1 BL A
- [Conclusion](conclusion.md) — UAG-PASS 5/5 on both topics; pre-discussion YAGNI on 3 (no FK clause, no caller_kind, no read path)
