---
id: "014"
title: "Analysis: Engineering Progress State Audit"
type: analysis
created: 2026-04-16
tags: [project-hygiene, progress-audit, stale-state, docs-drift]
---

# Analysis: Engineering Progress State Audit

## Question

What's left undone? Audit all discussions, plans, reviews, milestones, and backlog items for stale state, inconsistencies, and drift.

## Findings

### Prior Art from Project Knowledge Base

- **[discuss] Phase 1.1 scope** (`docs/discussions/012-phase-1.1-scope/conclusion.md`, decisional, 2026-04-09): Phase 1.1 theme was API contract correctness + knowledge capture completeness. All 7 items completed.
- **[discuss] Dreaming threshold fix** (`docs/discussions/013-what-next-after-pause/conclusion.md`, decisional, 2026-04-16): Lowered DEFAULT_MIN_RELEVANCE from 0.65 to 0.45. Also explicitly closes discussion 005.
- **[discuss] No new conflict resolution tool** (`docs/discussions/008-contradiction-detection/conclusion.md`, decisional, 2026-04-06): 4 targeted fixes to existing tools instead. Implemented directly, no separate plan.

### A. Pipeline Field Inconsistencies (state ≠ reality)

| File | Field | Says | Actually | Fix |
|---|---|---|---|---|
| `docs/plans/001-mvp-phase1.md:6` | `status` | `reviewed` | All 8 steps [x], review PASS | → `done` |
| `docs/discussions/002-mvp-phase1/index.md:11` | `pipeline.work` | `pending` | Plan 001 done | → `done`, `status` → `done` |
| `docs/discussions/003-tech-stack/index.md:11` | `pipeline.work` | `pending` | Plan 001 done | → `done`, `status` → `done` |
| `docs/discussions/005-hybrid-search-analysis/index.md:5` | `status` | `active` | Explicitly closed by discussion 013 conclusion | → `done` |
| `docs/discussions/008-contradiction-detection/index.md:5` | `status` | `active` | Topic 1 done, fixes implemented, no plan needed | → `done` |
| `docs/discussions/012-phase-1.1-scope/index.md:5` | `status` | `active` | `work: done` in pipeline | → `done` |
| `docs/discussions/013-what-next-after-pause/index.md:5` | `status` | `active` | `work: done` in pipeline | → `done` |
| `CLAUDE.md:158` | Project Status | "Phase 1 MVP — plan reviewed, ready for implementation" | 4 plans done, 4 reviews PASS, at validation gate | Rewrite section |

**Note on "reviewed" vs "done"**: The project uses `status: reviewed` as a pre-work gate (plan reviewed, ready for work). Plans 002–004 correctly use `status: done` after completion. Plan 001 is the only outlier — stuck at the gate state despite all work being finished.

### B. Duplicate Discussion ID

`docs/discussions/003-memory-credibility/index.md` and `docs/discussions/003-tech-stack/index.md` both use `id: "003"`. The tech-stack discussion is done (created 2026-04-04). The memory-credibility discussion is a Phase 2+ parking lot item (created 2026-04-05, `discuss: pending`, no progress). Any ID-based lookup would collide.

**Fix**: Renumber 003-memory-credibility to next available ID (015) or mark with a distinct sub-ID.

### C. Orphaned Artifacts in discussions/

Two directories lack `index.md` and are not discussions:

| Directory | Contents | Origin |
|---|---|---|
| `004-mvp-assessment/` | `analysis.md` (type: analysis, status: draft) | One-off analysis, not a pipeline discussion |
| `005-loop-validation/` | `validation.md` (no frontmatter) | Plan 002 Step 6 validation artifact |

These are complete artifacts that served their purpose. Moving them would break plan references. **Recommendation**: Leave in place but do not count as active discussions.

### D. Stale Discussions (analyze-only, no discuss/plan/work needed)

Six discussions created by `/ae:analyze` (2026-04-05) have `status: active`, `discuss: pending`, but their actionable findings were extracted to `docs/backlog/004-analyze-findings.md` with trigger conditions. None have triggered.

| Discussion | Topic | Backlog Items |
|---|---|---|
| 006 — SQLite Concurrency | Connection pooling, tokio safety | 004-19 (Phase 2) |
| 007 — Embedding Model Tradeoffs | Model size, quality, alternatives | 004-06/07/08 |
| 009 — Dreaming/Promotion Tuning | Thresholds, batch scheduling | 004-13/14 |
| 010 — Cross-Project Sharing | project_id scoping, privacy | 004-15/16/17/18 |
| 011 — MCP Tool API Design | Description quality, param design | (no numbered items) |
| 003 — Memory Credibility | Team-level quality variance | (none — Phase 2+ concept) |

**Recommendation**: Mark 006, 007, 009, 010, 011 as `status: deferred` (analysis captured in backlog, discuss when trigger fires). Mark 003-memory-credibility as deferred with renumbered ID.

### E. Stale Backlog Items

| Item | Issue | Evidence | Action |
|---|---|---|---|
| `002-review-deferred.md` #002-12 | Score normalization 0-1 | Plan 002 Step 1 explicitly says "resolves backlog 002-12" (commit 299b4e6) | Mark fixed |
| `002-review-deferred.md` #002-4 | FTS5 syntax abuse | `sanitize_fts_query` exists (Plan 004), but original trigger "untrusted input" hasn't changed — MCP is still trusted-caller-only | Leave open — partial fix, trigger still valid |
| `004-analyze-findings.md` #004-21 | Skill wiring (ae:plan/review/retrospect/think) | Plan 003 Steps 3–6 all [x] | Mark done |

### F. CLAUDE.md Technical Drift

| Line | Says | Reality | Severity |
|---|---|---|---|
| 35 | `rusqlite` features include `"fts5"` | Cargo.toml: `["bundled", "load_extension"]` — no `fts5` flag (works via bundled SQLite) | Low |
| 37 | `rmcp v0.16` | Cargo.toml: `v1.3`. Line 91 also says `v1.3`. Two contradictory versions in same file. | Medium |
| 36 | `sqlite-vec (optional, behind VectorStore interface)` | No `VectorStore` trait in code, no `sqlite-vec` in Cargo.toml. Cosine is methods on `Db` struct. | Low |
| 158 | "Phase 1 MVP — plan reviewed, ready for implementation" | 4 completed plan cycles, at validation gate | High |

### G. Milestones Directory Gaps

- Plans 001 and 003 have no milestone step-summaries (milestone system created after plan 001 execution)
- Plan 002 milestone covers Steps 1–2 only (Steps 3–6 missing)
- Plan 004 milestone is complete
- Not a bug — milestones were introduced mid-project. No action needed unless historical completeness matters.

### Challenges & Disagreements

**Challenger vs Archaeologist on "reviewed"**: Challenger proved that `status: reviewed` is a pre-work gate state, not a terminal state equivalent to "done". Plan 003's own work section uses "reviewed" as a gate condition. Archaeologist's claim of functional equivalence is incorrect.

**002-4 closure dispute**: Archaeologist said it was resolved by Plan 004. Challenger correctly noted the fix was motivated by AND-term recall, not by the untrusted-input threat model in the backlog. The mechanism is fixed but the trigger hasn't fired. Leave open.

**004/005 artifacts**: Archaeologist recommended classifying as "misclassified." Challenger pushed back — moving creates churn and breaks references. Agreed: leave in place.

**Codex cross-family perspective**: Project is at a **validation gate**, not a feature gate. Feature-complete MVP needs proof-of-use before Phase 2. Documentation volume is appropriate for a knowledge-capture tool (eating its own dog food) but requires cleanup. The 2-week forced-use scorecard from discussion 013 has not been initiated.

## Summary

The project has completed 4 full plan cycles (MVP → Close the Loop → Phase 1.1 → Search Quality Fixes), all reviewed PASS. But the docs haven't kept pace:

- **8 pipeline fields** are stale (discussions/plans stuck at old states)
- **6 discussions** are analyze-only stubs that will never progress to discuss/plan (findings already in backlog)
- **1 ID collision** (discussion 003)
- **4 CLAUDE.md inaccuracies** (project status, rmcp version, fts5 flag, VectorStore trait)
- **2 backlog items** resolved but not marked

The codebase itself is sound — the drift is entirely in docs/metadata.

## Possible Next Steps

1. **Fix now** — update the 8 stale pipeline fields, plan 001 status, CLAUDE.md project status + technical drift. This is a mechanical cleanup, ~30 minutes.
2. **Decision needed** — what to do with the 6 analyze-only discussions (defer/close) and the ID collision (renumber).
3. **Validation gate** — the logical next project step is the 2-week forced-use scorecard from discussion 013, not more features.

Ready for `/ae:discuss docs/discussions/014-progress-audit/` to resolve the decision items, or proceed directly to cleanup if the path is clear.
