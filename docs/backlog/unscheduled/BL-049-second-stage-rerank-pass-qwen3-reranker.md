---
id: BL-049
title: "Second-stage rerank pass — qwen3-reranker after RRF for precision@5 improvement"
status: open
created: 2026-05-18
origin: "v0.0.2 positioning discussion 2026-05-18 (QMD borrow #1 / Tier C, deferred): QMD uses qwen3-reranker as second-stage rerank after BM25+vec+RRF. Mengdie ends at RRF. Adding a rerank pass on top of RRF top-K improves precision@5."
size: M
depends_on: []
v_target: "vNext (deferred from v0.0.2 — needs local oMLX or Candle dep + dogfood evidence)"
---

# BL-049 — Second-stage rerank pass — qwen3-reranker after RRF

## Origin

QMD search pipeline:

```
query → BM25 + vec → RRF merge → top-K (say, 20)
                                → qwen3-reranker → top-N (say, 5)
```

Mengdie pipeline:

```
query → FTS5 + vec → RRF merge → top-K → (done)
```

The reranker is a cross-encoder trained specifically to score (query, doc) pairs, far more accurately than dual-encoder embedding cosine. Pays a latency cost (~50-200ms for qwen3-reranker on 20 candidates) but lifts precision@5 substantially.

## Scope

### Pipeline change

Add optional rerank stage in `search.rs`:

```rust
fn search_hybrid_with_rerank(query: &str, k: usize, n: usize) -> Vec<SearchResult> {
    let candidates = rrf_merge(fts5(query, k), vec(query, k));
    if let Some(reranker) = self.reranker.as_ref() {
        reranker.rerank(query, candidates).truncate(n)
    } else {
        candidates.truncate(n)
    }
}
```

### Model dependency

- **Path A — local oMLX** (recommended): qwen3-reranker GGUF served by oMLX at `http://127.0.0.1:8000`. Mengdie calls via OpenAI-compatible API. **Zero new Rust dep** beyond `reqwest` (already transitive via fastembed?). Operator runs `omlx serve` separately; mengdie tolerates absence.
- **Path B — Candle local**: load qwen3-reranker GGUF directly via `candle` crate. **Pure Rust** but ~50-100MB dep bloat + GPU/CPU runtime concerns. Don't pick this unless oMLX path fails.
- **Path C — claude CLI rerank prompt**: reuse `ClaudeCliProvider` to score (query, doc) pairs via prompt. Higher latency, but zero new infra. Lower quality than purpose-trained reranker.

Pick Path A first; fall back to C if oMLX isn't operator-ready.

### Config

```toml
[search.rerank]
enabled = false                              # off by default
provider = "omlx"                            # "omlx" | "claude_cli"
model = "qwen3-reranker-0.6b"
endpoint = "http://127.0.0.1:8000/v1"
candidates_k = 20                            # RRF top-K to rerank
return_n = 5                                 # how many to return after rerank
timeout_ms = 500                             # fall back to RRF order on timeout
```

## Acceptance criteria

1. Rerank stage opt-in via config; disabled = current behavior unchanged
2. Path A (oMLX) provider implemented; absence/timeout falls through to RRF order with `degraded: "rerank_unavailable"` annotation
3. End-to-end latency: ≤ 200ms p95 added on top of base search (so total p95 ≤ 250ms)
4. Benchmark: precision@5 ≥ 80% of operator manual-labeled "relevant" on a 20-query benchmark set (informal eval; build during dogfood)
5. Audit row records `rerank_used: bool` + `rerank_latency_ms` for ops visibility
6. Reranker scores included in returned `SearchResult` (alongside RRF score) for debugging

## Trigger

**Deferred to vNext.** Build when:
- Operator dogfood reveals top-5 results frequently include 1-2 obvious noise hits despite RRF tuning, **AND**
- Local oMLX is operationally stable in the operator's setup (per CLAUDE.md cross-family setup note)

Until both: capture but don't promote. Reranker is a quality-of-search upgrade, not a correctness fix.

## Non-goals

- Training a custom reranker on mengdie-specific data — use stock qwen3-reranker
- GPU optimization paths — local oMLX manages this
- Reranking synthesis rows differently — same scoring path
- Cross-encoder for full-text similarity (e.g., semantic dedup at ingest time) — different problem, separate BL

## Cost note

If operator runs Pro flat-fee plan, oMLX local = $0 marginal. claude-CLI rerank path costs ~100 tokens per (query, doc) pair × 20 candidates = 2000 tokens per search — significant on metered plans, acceptable on flat-fee. Config flag lets operator pick.
