---
id: "005"
title: "Analysis: Hybrid Search Architecture (FTS5 + Vector + RRF)"
type: analysis
created: 2026-04-05
tags: [search, fts5, vector, rrf, hybrid-search, embeddings, score-normalization]
---

# Analysis: Hybrid Search Architecture (FTS5 + Vector + RRF)

## Question
How does mengdie's hybrid search (FTS5 + vector + RRF) work and what are its current limitations?

## Findings

### Prior Art from Project Knowledge Base
- **Tech Stack Selection — Conclusion** (decisional, 003-tech-stack/conclusion.md): Rust chosen for compiler guardrails, single binary, sub-5ms startup. SQLite + fastembed for local-first operation.
- **MVP Phase 1 — Conclusion** (decisional, 002-mvp-phase1/conclusion.md): Hybrid search design with FTS5 + vector + RRF (k=60), metadata-in-chunk encoding, app-level cosine as primary vector path.

### Relevant Code

The search pipeline lives in 4 files:

| File | Component | Key Functions |
|------|-----------|---------------|
| `src/core/search.rs` | FTS5 search + RRF merge + orchestration | `search_fts`, `rrf_merge`, `memory_search` |
| `src/core/vector.rs` | Brute-force cosine scan | `search_vector` |
| `src/core/embeddings.rs` | Embedding generation + metadata-in-chunk | `embed_text`, `embed_with_context`, `cosine_similarity` |
| `src/core/mcp_tools.rs` | MCP tool wrapper + FTS fallback | `search` (MCP handler) |

**Pipeline flow** (`memory_search`, search.rs:99):
1. `search_fts(query, project_id, limit*3)` — BM25 candidates via FTS5
2. `search_vector(query_embedding, project_id, limit*3)` — cosine candidates via brute-force scan
3. `rrf_merge(fts, vec, k=60.0)` — Reciprocal Rank Fusion combines by rank position
4. Take top `limit` results, hydrate full entries
5. Normalize score: `raw / RRF_MAX` clamped to [0,1] where `RRF_MAX = 2/61`
6. `record_recall(id, normalized_score)` — update Dreaming stats

### Architecture & Patterns

**RRF Implementation** (search.rs:144-166): Standard `score(d) = Sigma 1/(k + rank_i(d))` with k=60. Rank-based fusion that avoids score scale mismatch between BM25 (negative, unbounded) and cosine similarity (0-1). Documents appearing in both rankers get higher combined scores. This is the exact formula from Cormack et al. (2009) and matches Elasticsearch/OpenSearch defaults.

**Score Normalization** (search.rs:120-133): Divides raw RRF by the theoretical maximum `2/61` (both rankers rank the document #1). This is a **non-standard but internally consistent** design choice for the Dreaming subsystem. Key implication: single-ranker matches cap at 0.5 normalized, requiring dual-ranker agreement for high scores. Dreaming's promotion threshold (avg_relevance >= 0.65) therefore implicitly requires cross-signal validation.

**Metadata-in-Chunk Encoding** (embeddings.rs:44-54): Documents are embedded with a structured prefix `[knowledge_type] [entities: ...] [project: ...] Title: ... --- content`. Query embeddings use plain text (no prefix). This asymmetric approach matches Anthropic's contextual retrieval recommendation and qmd learnings. The prefix "colors" the embedding space so that type-aware and entity-aware queries benefit from semantic proximity.

**FTS5 Query Escaping** (search.rs:43): Wraps query in double-quotes and escapes internal quotes. This treats the query as a phrase literal, preventing FTS5 operator injection (AND, OR, NOT, NEAR). Safe but restrictive — kills multi-term OR-style matching.

### Industry Practice Comparison

| Aspect | Mengdie | Industry Standard | Gap |
|--------|---------|-------------------|-----|
| RRF k parameter | k=60 | k=60 (Cormack 2009) | None |
| RRF formula | Exact standard | Exact standard | None |
| Vector search | Brute-force O(n) cosine | HNSW at scale (pgvector, Chroma) | Acceptable; sqlite-vec deferred correctly |
| Score normalization | /RRF_MAX (fixed denominator) | Raw RRF scores typically | Custom for Dreaming; internally consistent |
| Metadata encoding | In-chunk + SQL filter | Both approaches used in industry | Better than most — combines hard filter + semantic coloring |
| Text ranking | FTS5 BM25 | BM25 dominant | No stemming (FTS5 default tokenizer) |
| Fusion method | RRF | RRF preferred for heterogeneous rankers | None; LLM re-ranking deferred to 500+ memories |

### Challenges & Disagreements

**Challenger raised 6 issues:**

1. **FTS5 phrase-only matching** (confirmed by all): wrapping query in `""` prevents multi-term recall. "JWT authentication" won't match documents with these words in different sentences. This is the highest-impact quality limitation.

2. **Single-ranker score ceiling at 0.5** (confirmed by Codex + Gemini): memories matching only one ranker at rank 1 always normalize to 0.5. Dreaming's threshold of 0.65 means single-signal memories cannot be promoted. This is by-design (favors agreement) but should be documented as a Dreaming calibration coupling.

3. **Metadata-in-chunk asymmetry unvalidated** (partially valid): no empirical test that all-MiniLM-L6-v2 benefits from bracketed prefixes. Standards-expert notes this matches Anthropic's contextual retrieval pattern. The concern about token budget consumption is valid for long entity lists but unlikely to matter at current entity counts (3-5 per document).

4. **RRF_MAX hardcoded for 2 rankers** (valid, low risk): adding a third ranker would require updating the constant. The k=60 parameter and RRF_MAX are coupled but not co-located. Flag for future.

5. **FTS5 query escaping edge cases** (minor): backslash-in-query and whitespace-only queries not tested. Functional correctness is fine; these are defensive edge cases.

6. **One-ranker-empty degradation** (valid): when FTS returns 0 results and vector returns results, all vector results normalize to ~0.5 regardless of their actual cosine similarity. The normalized score loses the within-ranker quality signal. No test covers this case.

## Summary

Mengdie's hybrid search is **correctly implemented** and **well-aligned with industry best practices** for its target scale (10-1000 memories per project). The RRF fusion with k=60, brute-force cosine scan, and metadata-in-chunk encoding are all appropriate choices.

**Three key limitations to address:**

1. **FTS5 phrase-only matching** — the query escaping kills multi-term recall. Consider tokenizing multi-word queries into separate FTS5 terms joined with AND/OR instead of phrase wrapping.

2. **Single-ranker score ceiling** — normalized scores from single-ranker matches are capped at 0.5, creating an implicit coupling with Dreaming's 0.65 promotion threshold. Document this and consider whether single-ranker memories of high quality should have a promotion path.

3. **Brute-force vector scan** — O(n) is fine at current scale but will degrade linearly. The sqlite-vec integration path is already identified in the architecture.

## Possible Next Steps
- `/ae:discuss` to decide on FTS5 query tokenization strategy (phrase vs. term matching trade-off)
- Backlog items for single-ranker score calibration and vector indexing
