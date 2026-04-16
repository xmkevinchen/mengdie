---
id: "002"
title: "Deferred review findings"
status: open
created: 2026-04-05
tags: [review, deferred, performance, architecture]
---

# Deferred Review Findings

| ID | Status | Issue | Source | Fix | Trigger |
|----|--------|-------|--------|-----|---------|
| 002-1 | open | `Arc<std::sync::Mutex<Connection>>` blocks tokio executor threads during DB ops. Concurrent MCP requests serialize on the lock. | Doodlestein + Codex (Step 1+2) | Fetch rows into Vec, drop lock before compute. Or `tokio::sync::Mutex` / connection pool. | Phase 2 concurrent requests |
| 002-2 | open | Metadata-in-chunk query asymmetry: docs get metadata prepended before embedding but queries do not. By design (qmd uses same pattern). | Doodlestein (Step 2) | Prepend metadata to query embeddings if quality degrades. | Search quality tests show metadata-enriched docs rank lower than expected |
| 002-3 | ✅ fixed | Hardcoded limit=10, no score threshold. | Doodlestein (Step 2) | Added `limit` + `min_score` to MCP and CLI. See 002-12 for score scale issue. | — |
| 002-4 | open | FTS5 syntax abuse: query passed directly to MATCH. Operators (OR, NOT, NEAR) can alter semantics. Trusted agents only for now. | Codex (Steps 3-4) | Sanitize/escape FTS5 operators in query. | Exposed to user-facing or untrusted input |
| 002-5 | ✅ fixed | Hand-rolled debounce in watcher; `notify_debouncer_mini` in Cargo.toml but unused. | Doodlestein + Code-reviewer (Step 5a) | Replaced with `notify-debouncer-mini` 0.7 + `path.exists()` guard. | — |
| 002-6 | ✅ fixed | Contradiction detection magic numbers: extracted to named constants (`EVOLUTION_SIMILARITY_THRESHOLD`, `RECENT_CONFLICT_SIMILARITY_FLOOR`, `RECENT_CONFLICT_WINDOW_DAYS`). | Doodlestein (Step 5b) | — | — |
| 002-7 | open | Contradiction full table scan O(n) per ingest. No index on entity overlap. | Code-reviewer (Step 5b) | Add entity index or pre-filter query. | Ingestion latency exceeds 100ms per file |
| 002-8 | ✅ fixed | Dreaming thresholds now configurable via CLI: `--min-recall`, `--min-relevance`, `--window-days`. Defaults unchanged (3, 0.65, 14). | Doodlestein (Step 6) | — | — |
| 002-9 | ✅ fixed | Stats shows 0s on cold start. | Code-reviewer (Step 6) | Shows "no searches yet" / "no ingestions yet". | — |
| 002-10 | open | E2e test bypasses organic scoring: manually stuffs 9x `record_recall(0.9)` to overpower tiny RRF score. Tests plumbing, not realistic promotion. | Doodlestein (Step 7) | Add separate test with realistic data volumes and organic search patterns. | Dreaming thresholds tuned from real usage |
| 002-11 | ✅ fixed | CLI import discards contradiction results. | Doodlestein (Step 7) | `ingest_file` returns `IngestResult{entry_id, conflicts}`. CLI prints conflicts. | — |
| 002-12 | ✅ fixed | `min_score` doc says 0.0-1.0 but RRF scores are ~0.01-0.03 and FTS fallback uses `abs(bm25_score)` (unbounded). | Codex (backlog cleanup review) | Resolved by Plan 002 Step 1 score normalization (commit 299b4e6). | — |
| 002-13 | open | `check_contradictions` + `insert_memory` not in a transaction. Concurrent ingestions can race, missing conflicts. Advisory-only for MVP. | Codex (backlog cleanup review) | Wrap check + insert in single DB transaction. | Conflicts drive automated actions or concurrent ingestion supported |
