---
id: "009"
title: "Analysis: Dreaming/Promotion Tuning"
type: analysis
created: 2026-04-05
tags: [dreaming, promotion, recall-count, relevance, long-term-memory, scoring]
---

# Analysis: Dreaming/Promotion Tuning

## Question

How effective is mengdie's Dreaming promotion logic? Are the recall_count + avg_relevance thresholds well-calibrated, and does promotion produce meaningful behavioral change?

## Findings

### Prior Art from Project Knowledge Base

- **[analyze]: Mengdie hybrid search (RRF k=60) correctly implemented** (factual, `docs/discussions/005-hybrid-search-analysis/analysis.md`): RRF merges FTS5 + vector by rank with k=60. Relevant because avg_relevance tracks normalized RRF scores.
- **[analyze]: all-MiniLM-L6-v2 is weakest candidate but adequate** (factual, `docs/discussions/007-embedding-model-tradeoffs/analysis.md`): Embedding quality affects the cosine component of RRF, which feeds into avg_relevance.
- **[analyze]: search_vector O(N) cosine loop acceptable up to ~10K** (factual, `docs/discussions/006-sqlite-concurrency-mcp/analysis.md`): Search latency context for understanding when promotion-based filtering might help performance.

### Relevant Code

- **`src/core/dreaming.rs:9-13`**: Hardcoded defaults — `MIN_RECALL=3`, `MIN_RELEVANCE=0.65`, `WINDOW_DAYS=14`.
- **`src/core/dreaming.rs:51,74-81`**: Single SQL `UPDATE` sets `is_longterm=1` for all qualifying rows. Criteria: `recall_count >= min_recall AND avg_relevance >= min_relevance AND last_recalled >= cutoff AND valid_until IS NULL AND is_longterm = 0`.
- **`src/core/db.rs:168-181`**: `record_recall(id, relevance_score)` — increments `recall_count`, updates running average `avg_relevance`, sets `last_recalled`.
- **`src/core/search.rs:122-127`**: Score passed to `record_recall` is normalized RRF: `(score / RRF_MAX).min(1.0).max(0.0)` where `RRF_MAX = 2.0/61.0`.
- **`src/bin/cli.rs:27-39`**: CLI flags `--min-recall`, `--min-relevance`, `--window-days`. Runs globally (no per-project scoping).
- **`resources/com.mengdie.dream.plist`**: macOS launchd template, daily at 03:00. Not auto-installed; binary path is placeholder.
- **Schema**: `recall_count INTEGER`, `avg_relevance REAL`, `last_recalled TEXT`, `is_longterm INTEGER` — all on `memory_entries`.
- **Tests**: 5 unit tests + `test_record_recall` + `test_memory_search_updates_recall`. No test for 14-day recency window.

### Architecture & Patterns — The Critical Finding

**`is_longterm` has zero effect on search behavior.**

Confirmed by code inspection: `is_longterm` does not appear in `search.rs` or `mcp_tools.rs`. It is not used as a filter, boost, ranker, or any retrieval signal. The only consumers are:
- `mengdie list` display (LT: Y/N column)
- `mengdie stats` count
- Dreaming pass exclusion filter (`is_longterm = 0`)

**The entire Dreaming subsystem — thresholds, config, scheduling, CLI, tests — produces a flag that changes nothing in the retrieval pipeline.** This is the single most important finding of this analysis.

### Signal Quality Issues

Even if `is_longterm` were wired into search:

1. **`recall_count` measures retrieval frequency, not utility.** Every search hit increments recall regardless of whether the result was useful. A generic memory matching common queries accumulates high recall without being genuinely valuable.

2. **No session deduplication.** If `ae:analyze` runs 10 searches in one session that all return the same memory, `recall_count` increments 10 times. Should count as 1 session-day of engagement.

3. **`avg_relevance` is circular.** It tracks normalized RRF score — which measures "how well did this rank in search results." Promoting based on search rank, then boosting by promotion status, creates a self-reinforcing loop with no external validation signal.

4. **Hard AND gate is too restrictive.** Industry systems use weighted composite scores (recency + importance + relevance). Mengdie's AND logic means a memory with recall_count=2 and avg_relevance=0.95 fails promotion — strong on relevance but one recall short.

5. **14-day recency window breaks cold start.** Batch-imported memories can't be promoted until searched organically within 14 days. At MVP scale with few searches, most memories will never qualify.

### Industry Practice Comparison

**Production AI memory systems:**
- **Zep**: Temporal Knowledge Graph with three tiers (episodic → semantic → community). Promotion changes where and how memories are accessed — retrieval-privileged.
- **MemGPT/Letta**: Two-tier hierarchy (working ↔ archival). Promotion changes context availability. Agent-directed, not algorithmic.
- **Generative Agents (Park 2023)**: `score = recency + importance + relevance` (weighted sum, not AND gate). Importance is LLM-assigned at ingestion. No binary promotion — continuous scoring at retrieval time.
- **Mem0**: Async extraction + consolidation. Long-term entries retrieved preferentially. 91% lower p95 latency, 26% accuracy boost.

**Key finding**: No production system treats promotion as a write-only label. In every system, "long-term" means "retrieval-privileged" — it changes how the memory is accessed, ranked, or presented.

**Batch daily pass**: Biologically motivated (sleep consolidation), avoids session spike inflation. This is a valid pattern, not novel but sound.

**SM-2/FSRS (spaced repetition)**: Not used in any production AI agent memory system. Frequency + recency heuristics dominate. Not recommended for mengdie.

**Corpus size for promotion to matter**: Research shows clear wins at ~1-5M tokens (10K+ entries). Below that, keep-all is simpler.

### Challenges & Disagreements

**Challenger's position: Dreaming is a complete feature shaped like a real feature with no behavioral effect.**

Core argument: `is_longterm` is write-only. Thresholds are configurable but unmeasurable (no feedback loop). recall_count inflates from session bursts. avg_relevance is circular. The 14-day window prevents cold-start promotion. The entire subsystem costs code complexity and ingest latency for zero retrieval benefit.

**Standards-expert's response: This is an implementation gap, not a design flaw.**

The Dreaming architecture is sound and matches industry patterns (batch consolidation, frequency + recency + relevance signals). The gap is that `is_longterm` was never wired into search. The fix is small:

**Option A (recommended, ~3 lines in search.rs):** Score boost for long-term memories in RRF post-merge:
```rust
let boost = if is_longterm { 1.2 } else { 1.0 };
let final_score = normalized_rrf * boost;
```

**Option B:** Retrieval guarantee — always include matching long-term memories in results regardless of rank.

**Option C:** Separate search tier — return long-term results in a distinct "core knowledge" section.

**Cross-family (Codex):** Confirmed daily batch is a valid pattern. Frequency + recency + relevance is the right signal combination. SM-2/FSRS not validated for agent memory. Promotion becomes meaningful at ~1-5M tokens / 10K+ entries.

**Consensus:** The Dreaming infrastructure is architecturally correct. The critical gap is the missing wire from `is_longterm` to search behavior. Fix that first, then address signal quality (session dedup, composite scoring).

## Summary

**Dreaming's promotion logic is well-designed but disconnected from retrieval.** The `is_longterm` flag is set but never read by search. This is the highest-priority fix — without it, the entire Dreaming subsystem produces no behavioral change.

**Priority fixes:**

| Fix | Impact | Effort |
|---|---|---|
| Wire `is_longterm` into search as score boost | Enables Dreaming's entire value proposition | ~3 lines in search.rs |
| Add session deduplication for recall_count | Prevents session bursts from inflating promotion signal | Small — track last_recalled date, skip if same day |
| Add recency window test | 14-day cutoff logic is untested | 1 test case |

**Backlog items:**

| Item | Trigger | Action |
|---|---|---|
| Composite scoring (weighted sum) replacing AND gate | After score boost is wired and empirical data exists | Replace AND with `score = w1*recall + w2*relevance + w3*recency` |
| Importance signal at ingestion | Cold start remains a problem after 50+ imports | Add LLM-assigned or source-type-based importance at ingest time |
| Demotion mechanism | Not industry standard; explicit invalidation is correct for now | Revisit only if promoted set inflates without contradiction-based cleanup |
| Per-project dreaming scope in CLI | Multi-project usage | Add `--project` flag to `mengdie dream` |

## Possible Next Steps

- **Wire `is_longterm` into search** → direct code fix, no plan needed (Option A: score boost)
- If signal quality needs deeper redesign → `/ae:discuss` composite scoring vs AND gate
- Otherwise → backlog and proceed with other analyses
