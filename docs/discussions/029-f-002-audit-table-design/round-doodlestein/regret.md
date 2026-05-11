---
agent: doodlestein-regret-post
verdict: P2 (plan-time TODO + A-MEM precondition)
timestamp: 2026-04-28T20:52:42Z
---

# Doodlestein-regret post-conclusion review

**Most-likely-reversed decision**: "Best-effort + warn" failure mode (Topic 2)

## Trigger conditions for reversal

1. A-MEM algorithm paper-level confirmation reveals the algorithm requires strict completeness guarantees — e.g., if the supersession-rate signal is used not just as a volume threshold (≥5/30d) but as a relative rate or ratio that is sensitive to systematic under-counting from a recurring infrastructure fault. The current "monotonic-lower under-counting cannot cause wrong-direction trigger" argument holds only when the trigger is a floor check, not a rate check. If A-MEM's actual design uses "search events in last 30d with at least one superseded result / total search events" as a ratio, then systematic audit write failures deflate both numerator and denominator asymmetrically, and the "no wrong-direction" claim breaks.

2. A persistent infrastructure failure (DB disk full, WAL stall) causes a sustained run of silent audit drops over multiple days. The `METRIC_AUDIT_WRITE_FAILURES` counter is in-process only — it resets on restart, it does not persist across MCP server restarts, and it is not surfaced to any operator-visible dashboard in v0.0.1. An operator troubleshooting "A-MEM never fires" would have no audit trail to distinguish "no searches happened" from "searches happened but all audit writes silently failed." This is not a wrong-direction trigger issue — it's a debuggability issue that could force a contract tightening.

3. Wave 2 / BL-009 moves the call site into `search::memory_search_audited`. If that refactor also introduces a retry or buffering layer, the "best-effort at the Db layer" framing may be re-examined as part of that refactor rather than as a standalone decision.

## Severity: P2

Reversal is not cheap. "Best-effort + warn" is a behavior contract visible to MCP callers (search UX stays clean). Flipping to hard-error degrades search UX for infrastructure failures outside operator control — the conclusion already articulates this as the primary rejection reason for hard-error. Flipping to transaction-coupled requires restructuring `Db::memory_search` mutex lifecycle (12 lock cycles per call today). Neither reversal path is a one-line change. P1 was considered but rejected: the MCP search path itself is unaffected (callers never see audit errors), so no data is lost or corrupted — reversal would be painful but not catastrophic.

## Recommendation

Accept the risk on the "no wrong-direction trigger" argument — it holds for the stated floor-threshold trigger design. The fragile assumption is that A-MEM's trigger remains a floor check and not a ratio. The preventive measure is cheap: when A-MEM's algorithm is specified (before `ae:plan` is filed for the A-MEM feature), add one explicit acceptance criterion that states whether the trigger is a floor or a ratio. If ratio, revisit Topic 2 at that point. No schema or code change needed now — the criterion goes in the A-MEM plan's preconditions.

Additionally, the `METRIC_AUDIT_WRITE_FAILURES` counter should be noted in the F-002 plan as requiring persistence or periodic log emission (not just in-process accumulation) — this is a one-liner in the plan's open-questions section, not a design reversal, but it closes the "silent multi-day failure" debuggability gap before it becomes a reason to revisit the contract.

## Secondary notes

**"no caller_kind column"**: P3 (cheap reversal confirmed — one ALTER TABLE + unambiguous backfill). The "zero internal callers today" observation is correct, but the conclusion's claim that "adding later is cheap" is slightly over-confident on one point: if dreaming-time auto-search or contradiction self-evaluation land before the audit feature is widely used, backfill of pre-existing rows to 'operator' will mislabel those rows. This is a labeling accuracy issue, not a wrong-result issue. Accept the risk; note it in the BL for `caller_kind` when filed.

**"No v0.0.1 read path"**: P3, low reversal risk. The operator can always `sqlite3 ~/.mengdie/db.sqlite "SELECT ..."` as a stopgap. The validation-query-as-test path the conclusion carves out is sufficient for pipeline confidence. This will not reverse in 6 months.

**`Db::record_search_audit` on `impl Db`**: P3, low reversal risk. The in-memory buffer scenario requires a performance problem that doesn't exist at v0.0.1 corpus scale (100-400µs per the EXPLAIN timing). The Wave 2 free-function migration (R5) already creates the abstraction point where buffering could be introduced without touching `impl Db`. This will not reverse.

## TL disposition

Plan-time TODO #3 in conclusion's "Plan-time TODOs" section: A-MEM floor-vs-ratio precondition + counter persistence note.

Secondary findings preserved in this file for the BL-014 backlog (operator-discoverability) cross-reference.
