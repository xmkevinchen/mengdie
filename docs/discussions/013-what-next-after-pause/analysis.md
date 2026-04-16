---
id: "013"
title: "Analysis: What Next After 2-Week Pause"
type: analysis
created: 2026-04-16
tags: [project-direction, search-quality, dreaming, rrf-normalization, fts5, adoption]
---

# Analysis: What Next After 2-Week Pause

## Question
Project has been idle 2 weeks. Plans 001-003 all complete. What should we do next?

## Findings

### Prior Art from Project Knowledge Base
- **Phase 1.1 scope decision** (decisional, 012-phase-1.1-scope/conclusion.md): API contract enums + Phase C skill wiring. Valid_from 2026-04-09.
- **Hybrid search RRF correctly implemented** (factual, 005-hybrid-search-analysis/analysis.md): k=60 per Cormack 2009, brute-force cosine acceptable to ~500 memories.
- **all-MiniLM-L6-v2 adequate for MVP** (factual, 007-embedding-model-tradeoffs/analysis.md): FTS5+RRF reduces embedding quality dependence.
- **FTS5 IDF contamination across projects** (factual, 010-cross-project-sharing/analysis.md): BM25 computed over full corpus before project filter.

### Project State (Archaeologist)

| Aspect | Status |
|--------|--------|
| Last commit | 2026-04-09 (`75bf039` — ae:retrospect 002) |
| Unpushed commits | 19 (to Forgejo remote) |
| Untracked files | `docs/discussions/005-*`, `docs/milestones/` |
| src/ completeness | Fully implemented, zero stubs/TODOs |
| DB state | 46 memories, 3 projects, 464KB + 3.9MB WAL |
| MCP registration | Active, pointing to debug binary |
| Internal AE repo | Not present on this machine |

**Critical empirical data from DB:**
- All 46 memories have normalized scores clustering at 0.47-0.50
- Zero memories promoted (`is_longterm=0` on all rows)
- Top-recalled memory: 10 fetches, avg_relevance 0.48
- 14 recorded search events across 3 projects
- Source type distribution: 44 conclusion, 1 review, 1 retrospect

### Two Confirmed-Broken Subsystems

**1. Dreaming is permanently inert (all agents confirmed)**

RRF normalization divides by `RRF_MAX = 2/61` (theoretical dual-ranker maximum). Single-ranker rank-1 normalizes to ~0.5. Dreaming's promotion threshold is `avg_relevance >= 0.65`. This threshold is mathematically unreachable unless a memory appears as rank #1 in BOTH FTS5 and vector simultaneously — which the phrase-only FTS5 bug makes nearly impossible.

Result: the "spiral upward" core loop claim is broken. Memories accumulate but never get promoted. The system is a flat append-only store with a broken promotion system.

Fix options (from standards-expert):
- **Option B (correct, ~5 lines)**: Change `RRF_MAX` to use single-ranker max (`1/61`). Rescales so rank-1 single-ranker = 1.0, dual-ranker agreement exceeds 1.0 (clamped). Opens full 0-1 range.
- Option A (quick hack): Lower threshold to 0.45. Papers over the problem.
- Option C (architectural): Decouple Dreaming threshold from RRF normalization entirely.

**2. FTS5 phrase-only matching kills recall (all agents confirmed)**

Query wrapping in `""` treats all queries as exact phrases. "JWT authentication" won't match documents where those words appear in different sentences. This causes FTS5 to return 0 results for most multi-word queries, degrading hybrid search to vector-only (single-ranker). This explains the flat 0.47-0.50 score distribution — no FTS5 signal.

Fix: sanitize FTS5 operators, split on whitespace, join with AND. Fallback to OR if AND returns 0. ~20-50 lines. Discussion 005 (pending) should be closed with this decision.

### Compound Effect

These two bugs reinforce each other:
1. FTS5 phrase-only → FTS5 returns 0 for multi-word queries → search is vector-only
2. Vector-only → max normalized score = 0.5 → below Dreaming threshold (0.65)
3. Dreaming never promotes → no long-term memory → "spiral upward" loop broken
4. Flat score distribution → no discriminative signal → agents can't tell which results are most relevant

Fixing either one alone is insufficient. Both must be fixed before "use it" mode can generate meaningful validation data.

### Industry Practice Comparison (Standards Expert)

| Aspect | Mengdie Current | Industry Standard | Verdict |
|--------|----------------|-------------------|---------|
| RRF k=60 | Correct | Cormack 2009 standard | No gap |
| Score normalization | /RRF_MAX (2/61) — ceiling 0.5 | Raw scores or empirically calibrated | **Broken** |
| FTS5 query mode | Phrase-only | Term AND/OR (default in Lucene/ES) | **Broken** |
| Promotion threshold | 0.65 (unreachable) | Empirically calibrated from score distribution | **Miscalibrated** |
| Snippet length | 200 chars | 300-500 tokens | Gap (not blocking) |
| Sub-document chunking | Whole file | 150-500 token chunks | Deferred correctly |
| Vector search | Brute-force O(n) | HNSW at scale | Acceptable at 46 entries |

### Challenges & Disagreements

**Priority order debate (challenger vs standards-expert vs product-strategy):**

- Challenger: Dreaming threshold first → use it → FTS5 only if recall problems emerge
- Standards-expert (revised): RRF normalization first → FTS5 → validate → wire more skills
- Product-strategy: FTS5 first → usage mode (FTS5 is the "trust gate" for adoption)
- Codex-proxy: FTS5 first → retrieval validation → redefine Dreaming signal

TL resolution: Both fixes are small (~5 lines + ~30 lines), non-conflicting, and both confirmed broken by empirical data. **Do both before entering usage mode.** The sequencing debate is moot — the combined effort is under 50 lines of code.

**Circular validation (challenger raised, valid):**

All existing validation tests Mengdie on itself. The loop validation ran ae:analyze sessions about Mengdie and checked whether prior Mengdie knowledge appeared. This is circular. Real validation needs ae:analyze on a non-Mengdie project to test whether injected context changes output quality.

**Sub-document chunking (challenger defended deferral, correct):**

Standards-expert recommended chunking. Challenger correctly notes: explicitly deferred in backlog 001, trigger condition not met (46 memories of structured AE output, all under 2K tokens). Premature.

**Snippet length (standards-expert raised, challenger requested behavioral trigger):**

200 chars is 10-12x below industry standard (500 tokens). But no behavioral evidence that agents fail due to short snippets. Add to backlog with trigger: "agent fails to use relevant memory because snippet was insufficient."

**Two-repo split (challenger raised):**

Internal AE repo not on this machine. If AE skill wiring lives there, the write side of the loop may be disconnected. Source type distribution (44 conclusion, 1 review, 1 retrospect) suggests ingestion happened during validation sessions, not from ongoing pipeline operation. This is operational friction, not a code bug.

### Codex DX Assessment

Codex identifies the gap between "working MVP" and "daily reliance":
- Write path: live and reliable (44/46 memories auto-ingested)
- Read path: episodic (14 searches over 2 weeks, validation-driven, not habitual)
- Adoption blocked on search trust — phrase-only FTS means early searches missed results, eroding habit formation
- Recommends target metric: "10 real AE sessions/week invoke Mengdie, 70% produce useful top-3"

### Product Strategy (Gemini fallback)

"Fix the ONE blocking issue then switch to usage mode" — but with empirical data showing TWO blocking issues (FTS5 + normalization), fix both.

Key insight: **silent failures erode trust faster than visible errors**. The FTS5 bug produces plausible-looking results (not errors), causing invisible quality loss. For a single-user tool where the builder is the user, this leads to "I never search it anymore" — total product death.

## Summary

The project is in a clean, stable state with all 3 plans executed successfully. But two critical subsystems are empirically confirmed broken:

1. **RRF score normalization** caps all scores at ~0.5, making Dreaming's 0.65 threshold permanently unreachable
2. **FTS5 phrase-only matching** kills multi-term recall, reducing hybrid search to vector-only

These compound: FTS5 broken → vector-only → scores cap at 0.5 → Dreaming inert → no long-term memory → core loop broken.

**Both fixes are small** (~50 lines total) and should be done before entering "use it" mode. After fixes, validate on a non-Mengdie project to break the circular validation pattern.

## Possible Next Steps

1. `/ae:discuss 013` — decide on exact fix approach for both issues (RRF normalization Option B + FTS5 AND-term matching)
2. `/ae:plan` — implement both fixes + validation protocol
3. Push 19 unpushed commits to Forgejo
4. Add snippet length (200 chars → 500 tokens) to backlog with behavioral trigger
