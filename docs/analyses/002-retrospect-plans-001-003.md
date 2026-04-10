---
id: "002"
title: "Retrospect: Plans 001-003 (MVP → Close Loop → Phase 1.1)"
type: retrospect
created: 2026-04-09
data_sources: 3 review files
---

# Pipeline Retrospect: Plans 001-003

## Prior Art from Project Knowledge Base

Prior context: no prior retrospective insights from Mengdie (retrospect 001 predates Knowledge Capture wiring). Operational findings surfaced but not retrospective trends.

## Data Summary

| Feature | Steps | Rework | P1 Escape | P2 | Drift | Auto-pass | Cross-family |
|---------|-------|--------|-----------|-----|-------|-----------|-------------|
| 001 MVP Phase 1 | 8/8 | 100% (8/8) | 5 | 9 | 0 | 100% | Complete |
| 002 Close the Loop | 35/35 | 0% | 0 | 2 | 0 | N/A (manual) | Codex only |
| 003 Phase 1.1 | 6/6 | 0% | 0 | 1 | 0 | N/A (manual) | Degraded |

## Trends

### Steps Completed: Stable (100%)
All 3 plans fully completed. Plan sizes varied (8, 35, 6 steps) — the pipeline finishes what it plans regardless of scale.

### Rework Rate: ↓ Improving (100% → 0% → 0%)
Plan 001 had every step reworked. Plans 002-003 had zero rework. The improvement correlates with: (a) learning from plan 001 review patterns, (b) plans 002-003 executed manually vs ae:work, (c) plans 002-003 had more targeted scope (consolidation/validation vs greenfield build).

### P1 Escape Rate: ↓ Improving (5 → 0 → 0)
Plan 001's 5 P1 escapes were all cross-cutting concerns (Dreaming math, FTS5 injection, embedding dim mismatch) that per-commit review missed. Plans 002-003 had zero P1s. The lesson from plan 001 was absorbed — inline review during manual execution catches issues before commit.

### P2 Findings: ↓ Improving (9 → 2 → 1)
Steady decline: 9 → 2 → 1. Plan 003's single P2 (CLI/watcher enum bypass) is a non-trivial architectural mismatch but has no immediate impact. The trend suggests code quality is improving with each iteration.

### Drift Events: Stable (0)
Zero drift across all 3 plans. Plan adherence is strong — no unexpected files modified.

### Cross-family Coverage: ↓ Degrading
Plan 001: full (Codex + Gemini). Plan 002: Codex only (Gemini quota). Plan 003: degraded (both at quota). This is a resource constraint, not a process issue. Cross-family adds value (challenger consistently highest-value) but depends on external quota availability.

## Actionable Insights

### 1. Challenger agent is the consistent MVP
Across all 3 reviews, challenger found the highest-value findings:
- Plan 001: 8 challenges → 5 P1 fixes (Dreaming math, FTS5 injection)
- Plan 002: migration guard issue (only reviewer to flag it)
- Plan 003: CLI/watcher enum bypass (only reviewer to flag it)

Code-reviewer found 0 unique issues in plans 002-003 that challenger didn't also identify. **Challenger is load-bearing; code-reviewer is confirmatory.**

### 2. Manual execution outperforms ae:work on quality metrics
Plans 002-003 (manual) had 0% rework and 0 P1 escapes. Plan 001 (ae:work) had 100% rework and 5 P1 escapes. Caveat: plan 001 was a greenfield build (more complex), plans 002-003 were consolidation/validation (simpler). The comparison is not apples-to-apples. Next ae:work run will test whether the ae:work process has improved.

### 3. Cross-family quota is a systemic constraint
Gemini free tier exhausted quickly. Codex $20 Pro plan has limits too. The fallback to Claude model agents works but loses the "fresh eyes" benefit of a different model family. Consider: (a) upgrading Gemini to paid tier, (b) accepting degraded mode as normal for low-stakes reviews.

### 4. The knowledge loop is accelerating review quality
Plan 003's review benefited from prior context (Phase 1.1 scope decision surfaced at rank #1). The challenger's finding about CLI/watcher enum bypass was more targeted because the team had context about the enum change rationale. Knowledge accumulation is making reviews more focused.

## Recommendations (Prioritized)

1. **Keep challenger as mandatory reviewer** — consistently highest-value, 3/3 plans
2. **Test ae:work on the next plan** — plans 002-003 were manual, need data on whether ae:work quality has improved since plan 001
3. **Accept degraded cross-family as normal** — the Claude fallback model agents still produce high-quality findings; don't block reviews on quota
4. **Knowledge loop validation (AC9)**: run ae:plan on a real feature in a fresh session to verify the "part of workflow" signal — this is the remaining acceptance criterion from plan 003
