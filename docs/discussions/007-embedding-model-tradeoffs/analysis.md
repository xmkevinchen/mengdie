---
id: "007"
title: "Analysis: Embedding Model Tradeoffs"
type: analysis
created: 2026-04-05
tags: [embeddings, fastembed, all-minilm, vector-search, onnx, rrf]
---

# Analysis: Embedding Model Tradeoffs

## Question

Is all-MiniLM-L6-v2 the right embedding model for mengdie? What are the alternatives, and what tradeoffs matter at MVP scale?

## Findings

### Prior Art from Project Knowledge Base

- **[analyze]: Mengdie hybrid search (RRF k=60) correctly implemented** (factual, `docs/discussions/005-hybrid-search-analysis/analysis.md`): RRF merges FTS5 + vector by rank, not score. Brute-force cosine acceptable up to ~50K entries.
- **[analyze]: search_vector O(N) cosine loop acceptable up to ~10K memories** (factual, `docs/discussions/006-sqlite-concurrency-mcp/analysis.md`): At 1000 entries with 384d vectors, cosine takes ~0.2ms. sqlite-vec is the architectural fix for scale.
- **Tech Stack Selection** (decisional): Rust chosen for agent-centric guardrails. fastembed-rs selected for local ONNX inference.

### Relevant Code

- **`src/core/embeddings.rs:20-27`**: `AllMiniLML6V2` model, 384 dimensions. Init with `with_show_download_progress(true)`.
- **`src/core/embeddings.rs:31-38`**: Single-item `embed(vec![text], None)` — no batch path.
- **`src/core/embeddings.rs:44-54`**: Metadata-in-chunk encoding: prepends `[knowledge_type] [entities: ...] [project: ...]\nTitle: ...\n---\n{content}` before embedding. Document-side only (asymmetric).
- **`src/core/embeddings.rs:63-82`**: `embedding_to_blob` / `blob_to_embedding` — IEEE 754 LE f32, 1,536 bytes/entry at 384d.
- **`src/core/vector.rs:43-107`**: Brute-force cosine similarity over all embeddings. Filters by `embedding_dim`.
- **`src/core/schema.rs:38-39`**: Schema stores `embedding BLOB` + `embedding_dim INTEGER`. No model name/version stored.
- **`src/core/mcp_tools.rs:143-148, 258-263`**: `spawn_blocking` wraps embedding inference for async MCP tools.
- **`Cargo.toml:10`**: fastembed features include `image-models` (unused).

### Architecture & Patterns

**Current model characteristics:**
- all-MiniLM-L6-v2: 384 dimensions, ~90MB ONNX model, 2-10ms inference, 2019 architecture
- MTEB rank 158 (score 56.09) — weakest in the candidate set of small models
- English-only, no CJK support

**Candidate alternatives (fastembed-rs supported):**

| Model | Dims | Size | MTEB Rank | Notes |
|---|---|---|---|---|
| all-MiniLM-L6-v2 (current) | 384 | ~90MB | 158 (56.09) | Familiar default |
| BGE-small-en-v1.5 | 384 | ~130MB | 85 (61.76) | Same dims, much better quality |
| BGE-small-en-v1.5-Q (INT8) | 384 | ~33MB | ~85 | Quantized, smaller than current |
| gte-small | 384 | ~67MB | 92 (61.36) | Smallest model, good quality |
| BGE-base-en-v1.5 | 768 | ~440MB | Higher | 2x dims, best EN quality |
| nomic-embed-text-v1.5 | 768 | ~270MB | High | Long context (8192 tok), MRL-trained |
| BGE-M3 | 1024 | ~570MB | SOTA | Multilingual, dense+sparse+colbert, heavy |

**Key architectural facts:**
- Storage at 50K entries: 384d = ~75MB, 768d = ~150MB — both negligible
- FTS5+RRF hybrid means vector is a **tie-breaker**, not primary retrieval — BM25 does the heavy lifting for structured, entity-tagged content
- Metadata-in-chunk encoding partially compensates for model quality gaps by injecting searchable structure into embeddings
- Model change requires full re-embed — no zero-cost migration

### Industry Practice Comparison

Production local-first tools (Open WebUI, PrivateGPT, Obsidian plugins) standardize on small models (MiniLM, BGE-small, nomic) for latency/resource tradeoff. Key patterns:

- **Re-embed on model change** is standard practice — all tools document this as expected
- **Keep embedder warm** for daemon/server use cases (cold-start penalty too high for lazy loading)
- **INT8 quantization** is almost always the right choice for local CPU inference: <1% MTEB quality loss, ~3x faster, ~4x smaller memory footprint
- **fastembed-rs** remains the pragmatic Rust choice — rust-bert, ort, burn/candle exist but aren't plug-and-play for SentenceTransformer parity

### Challenges & Disagreements

**Challenger identified two concrete bugs:**

1. **`image-models` Cargo feature is active dead weight** (`Cargo.toml:10`): Pulls in CLIP model support and additional ONNX operator sets. Increases binary size and init surface. Unused. Should be removed now.

2. **`with_show_download_progress(true)` corrupts MCP transport** (`embeddings.rs:21`): Download progress goes to stderr, which IS the MCP stdio transport channel. First-run 90MB download will inject noise into Claude Code's MCP session. Must be `false` for the MCP binary path.

**Challenger's architectural challenge — is vector search earning its keep?**

The content being stored (AE pipeline output) is highly structured with explicit entity tags, knowledge_type, and titles. FTS5 BM25 handles specific term matches well. The vector leg catches paraphrases and semantic drift — but how often does AE output diverge terminologically from queries? Both are written by/for AI agents in the same project context. The marginal value of semantic search over keyword search may be low for this corpus.

Counter-argument (standards-expert): FTS5+RRF architecture means vector quality matters less (rank-based merge), but the vector leg still provides value as a tie-breaker for conceptually similar but terminologically different content. At MVP scale, the operational cost is acceptable.

**Cross-family (Codex):** Confirmed MiniLM is viable for MVP but materially weaker on 2025 benchmarks. Recommended benchmarking BGE-small-en-v1.5 or gte-small as drop-in upgrades. For code-heavy corpus, consider jina-embeddings-v2-base-code or nomic-embed-code.

**Consensus:** Keep MiniLM for MVP stability. Two bugs should be fixed now. Model upgrade is a backlog item with clear trigger.

## Summary

**all-MiniLM-L6-v2 is adequate for MVP but is the weakest model in the candidate set.** The hybrid FTS5+RRF architecture reduces dependence on embedding quality — BM25 carries the primary retrieval load for mengdie's structured, entity-tagged content. The vector leg provides meaningful but unquantified uplift as a semantic tie-breaker.

**Two bugs to fix now:**
1. Remove `image-models` feature from `Cargo.toml` — unused, adds binary bloat
2. Set `with_show_download_progress(false)` in MCP server path — current setting corrupts MCP stdio transport on first-run model download

**Three backlog items:**

| Item | Trigger | Action |
|---|---|---|
| Store model name/version in DB schema | Before any model change | Add `embedding_model TEXT` column; check on search to warn on mismatch |
| Upgrade to BGE-small-en-v1.5-Q (INT8) | Phase 1.1 or if retrieval quality is insufficient | Drop-in swap: same 384d, better quality, smaller model (~33MB vs ~90MB). Requires full re-embed. |
| Add batch embedding path | Bulk import becomes a real use case | Use fastembed batch API to amortize ONNX Runtime overhead |

## Possible Next Steps

- Fix the two bugs (image-models feature, download progress) → direct code change, no plan needed
- If model upgrade is prioritized → `/ae:discuss` to decide model + migration strategy
- If retrieval quality validation is needed → run A/B comparison (BM25-only vs hybrid) on real corpus
