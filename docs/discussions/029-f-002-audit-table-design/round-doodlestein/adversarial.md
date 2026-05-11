---
agent: doodlestein-adversarial-post
verdict: 3 findings — P2/P3/P2
timestamp: 2026-04-28T20:52:00Z
---

# Doodlestein-adversarial post-conclusion review

## Finding 1: METRIC_AUDIT_WRITE_FAILURES is ephemeral

**Severity**: P2

The conclusion's Topic 2 says best-effort + counter provides observability for bounded false negatives. This holds during a running process. But `AtomicU64` counters live in process memory. A disk-full event drops N audit rows, `METRIC_AUDIT_WRITE_FAILURES` increments to N — then the operator restarts mengdie-mcp (standard recovery action after disk-full). Counter resets to zero. The loss is invisible post-restart. The "bounded delays observable via the metric counter" claim is only true within a single process lifetime.

The conclusion's own rationale for rejecting hard-error is "infrastructure failures the operator can't recover from" — but the operator CAN recover from disk-full (free space, restart). After recovery, they have no way to know the audit table has gaps.

**Recommendation**: Backlog item (not a reopen). Add to the F-002 plan's acceptance criteria: the `tracing::warn!` log line must include the audit query text and timestamp so the audit gap is at least recoverable from logs (stderr) even after process restart. This costs one string format argument — no schema change, no behavioral change.

**TL disposition**: Plan-time TODO #2 in conclusion's "Plan-time TODOs" section.

## Finding 2: Orphan link-row accumulation poisons query planner cardinality after repeated rename_project

**Severity**: P3

The conclusion's R6 says "supersession query's inner join naturally excludes orphans" — correctness claim is valid. But ANALYZE-based cardinality estimates in SQLite are row-count statistics, not "live row" statistics. Each `rename_project` collision event creates orphan rows in `audit_returned_facts` (old fact_id FKs go dead). At v0.0.1 corpus (1000 facts / 300 audit rows / 3000 link rows), 10 rename events could push `audit_returned_facts` to 2× its "effective" row count. SQLite's planner may abandon the R4 covering index (`idx_audit_returned_facts_fact_audit`) in favor of a table scan when the estimated join cost flips. The R4 EXPLAIN was measured at a clean corpus; it was not measured with orphan inflation.

This is a slow-burn problem. It does not affect v0.0.1 correctness. It affects the supersession query's latency SLA 6–12 months out if rename_project is used regularly (which it will be — every new git remote triggers it).

**Recommendation**: Backlog item. Trigger: `audit_returned_facts` row count exceeds 10× the `memory_search_audit` row count (orphan ratio). Fix: periodic `DELETE FROM audit_returned_facts WHERE fact_id NOT IN (SELECT id FROM memory_entries)` — a cheap indexed scan given `idx_audit_returned_facts_fact_audit`. No schema change needed. The conclusion's "separate BL with PRAGMA enforcement trigger" is the right long-term fix but this is a simpler interim mitigation.

**TL disposition**: New backlog item BL-013 with concrete trigger condition (orphan ratio > 10:1).

## Finding 3: CI validation query does not cover operator-side audit population

**Severity**: P2

The conclusion distinguishes "minimal validation query as test" (v0.0.1 acceptance) from "user-facing CLI" (deferred). The CI validation query runs against a seeded test DB. The operator's production DB at `~/.mengdie/db.sqlite` is never queried by any v0.0.1 tooling. If the audit hook is silently broken (wrong call site after a merge, spawn_blocking removed, early-return on embedding error before the hook fires), `memory_search_audit` accumulates zero rows in production. A-MEM's deferred trigger depends on ≥5 searches in 30 days — if the table is empty, A-MEM never fires, and the operator has no signal that the audit system is broken vs. simply not yet triggered.

The conclusion's "operator can't recover from [this]" framing applies to search degradation, not to audit breakage. Audit breakage IS recoverable — it just requires a read path to discover.

**Recommendation**: The deferred read path is not strictly v0.0.1 blocking, but the plan should include a `mengdie doctor` or `mengdie audit-stats --count` subcommand as a P2 item for the v0.0.1.x patch window, before A-MEM is wired. Without it, the operator cannot distinguish "audit working, A-MEM trigger not yet reached" from "audit broken silently." The CI test alone is insufficient for this assurance.

**TL disposition**: New backlog item BL-014 (mengdie audit-stats subcommand for v0.0.1.x patch window). Does NOT expand v0.0.1's commitment per pre-decision; closes operator-discoverability gap before A-MEM lands.

## Summary

No reopen signal. All three are backlog-worthy follow-ons, not conclusion-invalidating. Finding 1 (ephemeral counter) and Finding 3 (operator debug discoverability) should both inform the F-002 plan's acceptance criteria annotations. Finding 2 is a future BL trigger item.
