# Backlog 004: Findings from 5 ae:analyze Sessions

Consolidated actionable findings from analyses 006-010 (2026-04-05).
Source: Step 6 Phase A validation of plan 002 (Close the Loop).

## Fix Now (in progress)

| ID | Finding | Source | Status |
|----|---------|--------|--------|
| 004-01 | Remove `image-models` Cargo feature (unused bloat) | 007 Embedding Model | done |
| 004-02 | `with_show_download_progress(false)` for MCP path (corrupts stdio) | 007 Embedding Model | done |
| 004-03 | Wire `is_longterm` score boost into search.rs (LONGTERM_BOOST=1.2) | 009 Dreaming/Promotion | done |
| 004-04 | AE Knowledge Capture: entity tags must be compound/specific (reduce FP conflicts) | 008 Contradiction + observed | done (AE repo) |
| 004-05 | AE Knowledge Capture: emit conflict summary in closing output | 008 Contradiction | done (AE repo) |

## Backlog (trigger-based)

| ID | Finding | Source | Trigger | Action |
|----|---------|--------|---------|--------|
| 004-06 | Store embedding model name in DB schema | 007 Embedding Model | Before any model change | Add `embedding_model TEXT` column; warn on mismatch |
| 004-07 | Upgrade to BGE-small-en-v1.5-Q (INT8) | 007 Embedding Model | Retrieval quality insufficient | Drop-in swap, same 384d, requires full re-embed |
| 004-08 | Add batch embedding path | 007 Embedding Model | Bulk import becomes real use case | Use fastembed batch API |
| 004-09 | Add `memory_resolve_conflict` MCP tool | 008 Contradiction | Phase 2 (resolution workflow) | Set valid_until on old + superseded_by on new |
| 004-10 | Remove or use `valid_from` (currently dead code) | 008 Contradiction | Phase 2 | Either use in queries or remove from "temporal validity" narrative |
| 004-11 | Add DB index for contradiction scan | 008 Contradiction | >1K memories | `CREATE INDEX ON memory_entries(project_id, valid_until)` |
| 004-12 | Calibrate contradiction thresholds empirically | 008 Contradiction | 100+ ingested memories | Measure FP rate, adjust EVOLUTION_SIMILARITY_THRESHOLD and RECENT_CONFLICT_SIMILARITY_FLOOR |
| 004-13 | Add session-day dedup for recall_count | 009 Dreaming/Promotion | Phase 2 | Skip recall_count increment if already recalled same calendar day |
| 004-14 | Replace AND gate with composite scoring | 009 Dreaming/Promotion | After score boost wired + empirical data | `score = w1*recall + w2*relevance + w3*recency` |
| 004-15 | Add global knowledge tier (scope field) | 010 Cross-Project | Phase 2 | `scope: "global"` on ingest; always include global memories in search |
| 004-16 | Add provenance labels to cross-project search results | 010 Cross-Project | Phase 2 | Show project name in SearchResult |
| 004-17 | Fix FTS5 IDF contamination | 010 Cross-Project | 5+ projects with overlapping vocab | Per-project FTS5 tables or include project_id in FTS5 |
| 004-18 | Add explicit project name override | 010 Cross-Project | Multi-machine or monorepo use | `--project-name` flag, store human-readable name |
| 004-19 | Wrap DB calls in spawn_blocking | 006 SQLite Concurrency | File watcher on same runtime | Or migrate to tokio-rusqlite |
| 004-20 | Migrate vector search to sqlite-vec | 006 SQLite Concurrency | >10K memories | Already in VectorStore interface |
| 004-21 | ~~Wire knowledge capture into ae:plan, ae:review, ae:retrospect, ae:think~~ | PRD Phase C | ✅ done | Completed in Plan 003 Steps 3–6 (ae:plan, ae:review, ae:retrospect, ae:think skills all wired) |
