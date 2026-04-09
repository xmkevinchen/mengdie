---
id: "001"
title: "Retrospect: Plans 001-002 (MVP + Close the Loop)"
type: retrospect
created: 2026-04-08
data_sources: 2 review files
---

# Pipeline Retrospect: Plans 001-002

## Data Summary

| Feature | Steps | Rework | P1 Escape | P2 Findings | Drift | Auto-pass |
|---------|-------|--------|-----------|-------------|-------|-----------|
| 001 MVP Phase 1 | 8/8 | 100% (8/8) | 5 | 9 (7 fixed, 2 deferred) | 0 | 100% |
| 002 Close the Loop | 35/35 | 0% | 0 | 2 (2 fixed) | 0 | N/A (manual) |

## Trends

### Steps Completed: Stable (100%)
Both plans fully completed. Plan 002 was significantly larger (35 sub-items vs 8 steps) but both reached 100%. The pipeline completes what it plans.

### Rework Rate: ↓ Improving (100% → 0%)
Plan 001 had every step need fixup in review. Plan 002 had zero rework. Two factors explain the dramatic improvement:
1. Plan 002 was executed manually (not via ae:work), so per-commit code review caught issues inline rather than deferring to feature review.
2. Plan 002 included validation (analysis sessions) rather than pure code, reducing code-level defects.

### P1 Escape Rate: ↓ Improving (5 → 0)
Plan 001 had 5 P1 findings escape per-commit review into feature review. Plan 002 had zero. Key lesson from 001: **per-commit reviews catch localized bugs but miss systemic issues** (Dreaming math, FTS5 injection, embedding dim mismatch). Plan 002 benefited from this learning — inline code review during analysis sessions caught issues before commit.

### P2 Findings: ↓ Improving (9 → 2)
Plan 001: 9 P2s (7 fixed, 2 deferred). Plan 002: 2 P2s (2 fixed, 0 deferred).
- Both plan 002 P2s were cross-cutting concerns (project_id guard, migration safety) — the same category as plan 001's P1s. The difference: plan 002's concerns were caught before they caused data integrity issues (P2, not P1).
- Challenger agent continues to be the highest-value reviewer — found the migration guard issue that no other reviewer flagged.

### Drift Events: Stable (0)
No contract violations in either plan. Plan adherence is strong.

## Actionable Insights

### 1. Challenger agent is consistently the most productive reviewer
- Plan 001: 8 structured challenges, 5 became P1 fixes
- Plan 002: Found migration guard issue (highest-value finding), challenged LONGTERM_BOOST calibration, identified list_memories complexity
- **Recommendation**: Always include challenger in reviews. Consider giving challenger more context (prior review findings) to compound learning.

### 2. Per-commit review misses cross-cutting concerns
- Plan 001 lesson: per-commit reviews caught localized issues but missed systemic problems visible only at feature level
- Plan 002 validation: inline code review during manual execution worked better — 0 P1 escapes
- **Recommendation**: For critical features, run a lightweight feature-level review mid-execution (after 50% steps) rather than only at the end.

### 3. Security reviewer catches privilege boundary violations
- Plan 001: found input length limits missing, error message leaking internals
- Plan 002: found cross-project invalidation via resolves param (no project_id guard)
- **Recommendation**: Continue including security-reviewer. Its findings are consistently high-value P2s that other reviewers miss.

### 4. Manual execution (plan 002) produced better outcomes than ae:work (plan 001)
- 0% rework vs 100% rework
- 0 P1 escapes vs 5 P1 escapes
- Caveat: plan 002 was primarily validation (analysis sessions), not pure code. Code-heavy steps (steps 1-2) were done in a prior session.
- **Recommendation**: Not enough data to conclude manual > ae:work. The improvement likely reflects learning from plan 001, not execution method. Continue using ae:work for code-heavy plans.

### 5. Deferred P2s from plan 001 are still open
- P2-3 (Db god object): tracked in backlog BL-002-3, no trigger has fired
- P2-6 (Contradiction TOCTOU): partially addressed by plan 002's `insert_memory_resolving` with SQLite transaction, but the original two-call pattern still exists via `memory_invalidate`
- **Recommendation**: P2-6 is partially resolved. Update backlog to reflect current state.

## Recommendations (Prioritized)

1. **Continue including challenger + security-reviewer in all reviews** — consistently highest-value findings across both plans
2. **For next plan using ae:work**: add a mid-execution review gate after 50% steps to catch cross-cutting P1s earlier
3. **Update backlog BL-002-6** (TOCTOU) — partially resolved by `insert_memory_resolving`, remaining risk is acceptable for single-client stdio
4. **Accumulate more data** — 2 reviews is the minimum for trend detection. After 3-4 more plans, trends will be statistically meaningful
