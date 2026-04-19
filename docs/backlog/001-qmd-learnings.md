---
id: "001"
title: "Learnings from qmd (tobi/qmd)"
status: closed
closed: 2026-04-19
closed_reason: "Phase 1 items all shipped: RRF (plan 002), score normalization (plan 002 Step 1 commit 299b4e6), metadata-in-chunk encoding (plan 002). Phase 2+ items triggers NOT firing at 238 memories: #4 LLM re-ranking (500+ memory trigger), #5 smart chunking (longer-docs trigger), #6 query expansion (explicit skip). File kept as-is for doc history; no action pending."
source: "Discussion 003 tech stack evaluation"
created: 2026-04-04
tags: [search, hybrid-search, qmd, prior-art]
---

# Learnings from qmd

[qmd](https://github.com/tobi/qmd) is an on-device hybrid search engine (BM25 + vector + LLM reranking) for markdown/docs. TypeScript/Node.js, MCP server, SQLite. Evaluated during Discussion 003.

**Verdict**: Different product (general doc search vs knowledge lifecycle). No integration. Three techniques worth adopting.

---

## Phase 1: Adopt Now

### 1. Reciprocal Rank Fusion (RRF) for hybrid search merging

qmd merges BM25 + vector results using RRF (k=60) instead of naive linear score blending. RRF is rank-based, not score-based — avoids scale mismatch between FTS5 BM25 scores and cosine similarity.

```
score(d) = Σ 1 / (k + rank_i(d))  for each ranker i
```

- ~20 lines of Rust, zero dependencies
- Implement as part of `memory_search` hybrid search
- Well-studied in IR literature; consistently outperforms linear blending

### 2. Score normalization to 0-1 range

qmd normalizes raw scores before merging:
- FTS5 BM25: `abs(score)` normalized to 0-1
- Vector: `1 / (1 + cosine_distance)` → already 0-1
- Position-aware blending: top-3 preserve 75% retrieval confidence, 4-10 use 60%

Second Brain needs this for:
- Dreaming's `avg_relevance` field — must be comparable across search types
- Consistent score interpretation for contradiction detection thresholds

### 3. Metadata-in-chunk encoding

qmd prepends document context (folder path, title, tags) to chunk text before generating embeddings. This "colors" the embedding with semantic metadata without requiring metadata-aware retrieval logic.

For Second Brain: prepend structured metadata to memory text before embedding:
```
[decisional] [entities: auth, middleware, compliance] [project: second-brain]
Title: Auth middleware rewrite decision
---
<actual memory content>
```

This improves embedding quality for free — no schema change, no retrieval logic change. Just prepend at ingestion time.

---

## Phase 2+: Defer / Revisit

### 4. LLM re-ranking (DEFER — revisit at 500+ memories)

qmd uses a 640MB cross-encoder reranker to re-score top candidates. Cross-encoders consistently outperform bi-encoder similarity for ranking.

**Why not Phase 1**: Structured AE output + entity filtering already high-signal. At 10-50 memories per project, FTS5 + vector + RRF is recall=100%. Reranking reshuffles top-10 — marginal gain at 10x resource cost.

**Revisit trigger**: If `memory_search` quality degrades after 500+ cross-project memories, or if user reports "I know this decision exists but search missed it."

**Rust path**: `ort` crate can load cross-encoder GGUF models; `candle` also viable.

### 5. Smart chunking with boundary detection (DEFER — revisit if ingestion expands)

qmd chunks at 900 tokens with 15% overlap, detecting boundaries at headings/code-fences/paragraphs.

**Why not Phase 1**: AE output (conclusions, reviews, plans) are structured documents typically <2K tokens. Document-level embedding is sufficient. Chunking would fragment entity coherence (e.g., splitting Decision Summary table from its rows).

**Revisit trigger**: Phase 2 ingestion of longer documents (GitHub discussions, Slack threads, meeting transcripts).

**Rust path**: `pulldown-cmark` for markdown boundary detection; `tree-sitter` bindings for code-aware chunking.

### 6. Query expansion (SKIP — unlikely to need)

qmd uses a 1.1GB model to generate 2-3 query variants before search. Useful when users phrase queries inconsistently.

**Why skip**: Second Brain queries come from AI agents (ae:analyze), not humans typing terse searches. Agent queries are already semantically rich.

**Revisit trigger**: Only if ae:analyze consistently fails to find relevant memories despite them existing.

---

## Explicitly Not Adopting

| qmd Feature | Why Skip |
|-------------|----------|
| LLM cache | No LLM inference in Phase 1 = nothing to cache |
| AST-aware chunking | Ingesting AE markdown, not code files |
| Hierarchical context metadata | Already have richer: knowledge_type + entities + temporal validity |
| HTTP daemon (model warming) | Phase 2 axum daemon already planned; fastembed-rs 90MB load is fast enough for stdio |
| Integration with qmd | TypeScript + 2GB models + 3-4GB RAM inverts Rust rationale |

---

## Key Insight

qmd optimizes for noisy, diverse, unstructured documents (notes, transcripts, codebases). Second Brain ingests structured, multi-agent-validated AE output — signal-to-noise is already high. Heavy reranking/expansion solves a problem we don't have.
