---
source: mengdie memory_search
query: "power-law decay dreaming promotion demotion avg_relevance effective_relevance forgetting"
retrieved: 2026-04-20
---

# Prior Art from Project Knowledge Base

Background context retrieved from Mengdie before Round 1. Agents MAY use
these as starting hypotheses but MUST verify against current code —
memories can be stale. Not prescriptive; not a filter on what agents
should research.

## Relevant prior findings

### 1. `recall_count` inflates from intra-session bursts
- **Title**: recall_count inflates from session bursts — needs session-day deduplication
- **Type**: factual
- **Source**: `docs/discussions/009-dreaming-promotion/analysis.md`
- **Valid from**: 2026-04-06
- **Snippet**: record_recall increments recall_count on every search hit with
  no session deduplication. If ae:analyze runs 10 searches in one session
  returning the same memory, recall_count increases by 10 instead of 1.
- **Relevance to BL-008**: a decay-over-time mechanism layered on an inflated
  base signal may over-retain burst-recalled memories. Decay design should
  acknowledge this as a precondition, not silently assume `recall_count` is
  clean.

### 2. Dreaming promotion was inert pre-plan-004 (RRF score ceiling)
- **Title**: FTS5 phrase-only + RRF_MAX compound bug renders Dreaming
  promotion permanently inert
- **Type**: experiential
- **Source**: `docs/discussions/013-what-next-after-pause/analysis.md`
- **Valid from**: 2026-04-19
- **Snippet**: Empirical analysis of 46 memories confirmed all normalized
  RRF scores cluster at 0.47–0.50; threshold `avg_relevance >= 0.65` was
  mathematically unreachable. Fixed by lowering threshold to 0.45 (plan 004).
- **Relevance to BL-008**: the *distribution* of `avg_relevance` is
  compressed near 0.5 (not uniform on [0,1]). A decay threshold written as
  "effective < 0.01" needs this distribution assumption checked — a narrow
  base range makes even mild decay cross the floor.

### 3. `is_longterm` flag has zero effect on search ranking today
- **Title**: is_longterm flag has zero effect on search — Dreaming subsystem
  is disconnected from retrieval
- **Type**: factual
- **Source**: `docs/discussions/009-dreaming-promotion/analysis.md`
- **Valid from**: 2026-04-06
- **Snippet**: `is_longterm` is set by Dreaming but never read by `search.rs`
  or `mcp_tools.rs`. Appears only in `mengdie list` display + stats.
- **Relevance to BL-008**: demotion's user-visible effect depends on
  `is_longterm` being read by SOMETHING downstream. If still unwired at
  ship time, demotion is a no-op for end-user experience. Either (a)
  decide demotion's effect is internal to Dreaming bookkeeping (fine — but
  name that explicitly), or (b) wire `is_longterm` into search ranking as
  part of or alongside BL-008.

### 4. Dreaming threshold calibration was the most recent successful tuning
- **Title**: Lower Dreaming threshold from 0.65 to 0.45, keep RRF_MAX=2/61 —
  Option B destroys dual-signal semantics
- **Type**: decisional
- **Source**: `docs/discussions/013-what-next-after-pause/conclusion.md`
- **Valid from**: 2026-04-16
- **Snippet**: Chose to lower threshold rather than change normalization,
  because changing RRF_MAX to 1/61 clamps both single- and dual-ranker
  scores to 1.0 and destroys the dual-signal semantics.
- **Relevance to BL-008**: precedent for "change the threshold, not the
  scoring function." A decay design that replaces or heavily rewrites
  `avg_relevance` would re-open this debate. A design that adds a time
  factor on top preserves the invariant plan 004 committed to.

### 5. Backlog items must have explicit trigger conditions
- **Title**: Backlog items that aren't "later cleanup" — file them with
  explicit trigger conditions tied to future plans
- **Type**: experiential
- **Source**: `docs/reviews/007-embedding-clustering.md`
- **Valid from**: 2026-04-18
- **Relevance to BL-008**: any issues this discussion surfaces but defers
  (e.g., tuning the exact exponent, wiring `is_longterm` into search
  ranking) need triggers — not "later".

_No conflicts or degradation reported by memory_search._
