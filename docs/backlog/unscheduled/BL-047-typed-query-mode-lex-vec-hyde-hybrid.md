---
id: BL-047
title: "Typed query mode — `mode: lex | vec | hyde | hybrid` parameter on memory_search"
status: open
created: 2026-05-18
origin: "v0.0.2 positioning discussion 2026-05-18 (QMD borrow #6 / Tier C): QMD exposes `query` (hybrid+rerank), `search` (BM25), `vsearch` (vector) as distinct entry points. Mengdie always runs hybrid+RRF with no caller control. Diagnostic + power-use case for explicit mode."
size: S
depends_on: ["BL-048"]
v_target: "v0.0.2 — Tier C (QMD borrow)"
---

# BL-047 — Typed query mode parameter on memory_search

## Origin

`mcp_tools::memory_search` currently runs FTS5 + vector + RRF unconditionally. Callers can't:

- Diagnose query failures ("did vector match? did FTS match?")
- Force one mode for a known query shape (exact term lookup → lex-only is faster)
- Use HyDE for abstract queries (BL-048)

QMD treats query modes as first-class. Mengdie should add a `mode` parameter without changing the default.

## Scope

Add `mode` field to `SearchParams`:

```rust
pub enum QueryMode {
    Hybrid,  // default; current behavior — FTS5 + vector + RRF
    Lex,     // FTS5 only; faster for exact-term lookups
    Vec,     // vector only; useful for semantic-only queries
    Hyde,    // HyDE — see BL-048
}

pub struct SearchParams {
    // ...existing fields...
    pub mode: Option<String>,  // serializes as the enum; "hybrid" default
}
```

Wire through `search.rs`:

- `Hybrid` → existing path
- `Lex` → skip vector; return FTS5 results directly (normalize scores to [0,1])
- `Vec` → skip FTS5; return vector results directly
- `Hyde` → branch to BL-048 implementation; falls back to `Hybrid` if HyDE not implemented

## Acceptance criteria

1. `memory_search` accepts `mode` parameter; omitted = `Hybrid` (unchanged default)
2. `Lex` / `Vec` paths produce ranked results from that pipeline only
3. Score normalization consistent across modes (caller can compare `min_score` thresholds)
4. Unknown `mode` value → error with the list of valid values
5. Audit row (`memory_search_audit.scope`) records which mode was used — extend `scope` semantics OR add a `mode` column
6. **Hyde mode** behavior: if BL-048 has shipped, use it; if not, fall back to Hybrid with a `degraded` flag in the result envelope

## Trigger

Ships **with or after BL-048**. Standalone delivery without `Hyde` is allowed (Hyde mode falls back); but the value of `mode` parameter is much greater when all four modes work.

## Depends on

- **BL-048** (HyDE query mode) — for `Hyde` variant to be functional rather than fallback

## Non-goals

- Per-mode score weighting on Hybrid — that's an RRF tuning concern (BL-021 territory)
- Reranker mode (the qwen3-reranker pass) — separate BL-049
- Storage-aware modes (e.g., "only synthesis", "only longterm") — those are filter parameters, not query modes
