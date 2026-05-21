# Changelog

All notable changes to Mengdie are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows [semantic versioning](https://semver.org/).

## Unreleased

(nothing scheduled — next sprint not started)

---

## v0.0.2 — 2026-05-19

**Theme**: entity-graph upgrade + 4 new MCP tools + retroactive test
harness. Completes the Karpathy LLMWiki Ingest/Query/Lint trio (v0.0.1
shipped Ingest + Query; v0.0.2 adds Lint).

### Added

- **Entity graph substrate.** `memory_entries.entities` (comma-separated
  TEXT) is now normalized into an `entities` table + `fact_entity`
  many-to-many link table. Ingest dual-writes atomically under
  transaction. Contradiction detection switches from `LIKE '%name%'`
  full-table scan to indexed lookup. Legacy TEXT column preserved for
  FTS5 indexing; removal deferred to a follow-up.
- **`memory_lint` MCP tool.** Three deterministic SQL-only health checks:
  (1) orphan GC across `superseded_by` / `memory_synthesis_links` /
  `audit_returned_facts`; (2) unresolved contradictions — half-resolved
  supersession state, size-2 cycles, ≥0.7 Jaccard entity-overlap pairs;
  (3) embedding drift — missing/wrong-dim embeddings, plus surface for
  synthesis rows shipped with `embedding=NULL` in v0.0.1.
- **`memory_get` MCP tool.** Fetches full `MemoryEntry` content (not a
  200-char snippet) by full UUID or 8+ char prefix. Cross-project guard
  via `scope` parameter. Direct-lookup recall bumps `recall_count` only
  (no relevance-score mix into avg_relevance EMA).
- **`memory_status` MCP tool.** Surfaces DB health to LLM callers:
  `total_entries` / `longterm_count` / `synthesis_count` /
  `by_source_type` breakdown / `last_ingest_at` / metrics counters /
  audit-pipeline view / `embedding_model` + `embedding_dim`. Read-only,
  project-scoped (or `scope: "global"`).
- **`memory_entity_facts` MCP tool.** Returns all facts tagged with a
  given entity name in sub-millisecond index lookup, ordered by
  `recall_count` desc. Uses the new `fact_entity` index.
- **Short citable docids.** `short_id` (first 8 hex chars of UUID) added
  to search results. `memory_invalidate` accepts 8+ char prefix with
  structured ambiguity-error format; full-UUID input keeps fast path.
  CLI `mengdie list` JSON output gains `short_id` alongside full `id`.

### Changed

- **`memory_search` result shape**: each item gains a `short_id: String`
  field (first 8 hex chars of `id`). Existing `id` field unchanged.
- **`memory_invalidate` input**: now accepts 8+ char prefix in addition
  to full UUID. `InvalidateOutput` gains `error: Option<String>` for
  ambiguity / not-found disambiguation.

### Internal

- Schema bumped `user_version 7 → 8` with idempotent backfill. Migration
  is one-shot, transactional, `ON CONFLICT DO NOTHING` on both upserts —
  re-runs (interrupted-recovery scenarios) produce identical state.
- Contradiction-detection candidate-finding refactored from O(N)
  scan-and-split to O(|new_entities| × index lookup). Eliminates the
  LIKE-scan path entirely.
- New MCP integration test harness under `tests/common/mod.rs` +
  `tests/mcp_integration.rs` covers `memory_lint` / short-id /
  `memory_get` dispatch paths. Retroactive coverage for tools shipped
  in earlier features.

### Cargo

- Net delta: **0 new crates**. All new functionality rides existing
  transports (rusqlite for schema migration, fastembed-rs unchanged,
  rmcp for new tools).

---

## v0.0.1 — 2026-05-10 (rebuild branch ship)

**Theme**: minimum-viable AE-brain that avoids re-inventing wheels —
narrow OSS adoption, keep working in-house code. First public-shape
ship after the v0.x → v0.0.x lineage reset.

### Added

- **sqlite-vec adoption** replaces the brute-force cosine similarity
  in `vector.rs` with `sqlite-vec` v0.1.9's `vec0` virtual table +
  triggers. fastembed-rs unit-vector invariant asserted at the
  embedder boundary so the similarity formula's unit-norm assumption
  cannot silently drift.
- **`mengdie audit-stats` CLI subcommand.** Reads `audit_count`,
  `link_count`, `oldest_row`, `newest_row`, `supersession_count_30d`,
  `audit_write_failures` from the audit tables. Emits human table or
  script-facing JSON via `--format {table,json}`. Health enum:
  `ok` / `not_yet_triggered` / `degraded` with actionable hint.
- **Persisted domain audit substrate.** Two new tables
  (`memory_search_audit` + `audit_returned_facts`) plus 3 indexes ship
  via the v6 schema migration. Every `memory_search` call writes one
  audit row capturing `(query, scope, took_ms, searched_at)` and N
  link rows mapping audit→fact for the caller-visible result set.
  Failures emit a `tracing::warn!` line and bump the
  `audit_write_failures` metric counter without affecting the search
  response. Hook fires from both the MCP tool and the CLI.
- **Retrieval & ingest layer consolidation.**
  `search::memory_search_audited(&Db, ...)` orchestrator (free
  function) replaces the duplicated audit hook in MCP and CLI search
  paths. New types in `core::search`: `FallbackPolicy` (`HybridOrError`
  / `HybridOrFtsOnly`), `SearchRoute` (`Hybrid` / `FtsOnly`),
  `FallbackReason::EmbeddingUnavailable`, `MemorySearchOutcome`.
  Per-surface defaults: MCP=`HybridOrFtsOnly`, CLI=`HybridOrError`.
  Two public ingest entries in `core::ingest`: `ingest_text` for
  plain insert + `ingest_text_with_resolves` for atomic resolve+insert.
- **Synthesis CLI structured output via claude-CLI `--json-schema`.**
  Replaces the ~30 LoC brace-depth scanner in `synthesis.rs` with
  claude-CLI's native `--json-schema` + `--output-format json` flags.
  Schema is flat-shape with `skip:bool` discriminator (top-level
  `oneOf`/`allOf`/`anyOf` not accepted by Anthropic API).
  `LlmProvider::complete_structured` added as a sibling trait method
  with default impl returning `UnknownProvider`.

### Changed

- **`memory_search` FTS-fallback scores are now [0,1]-normalized** via
  per-call min/max linear rescale. Pre-v0.0.1, FTS-fallback returned
  raw `bm25_score.abs()` values (typically 3-50), so `min_score`
  filtering was effectively a no-op in degraded mode. Operators with
  calibrated raw-BM25 thresholds will see different filtering on the
  embed-fail path. Hybrid path (RRF-normalized) is unchanged.
- **`memory_ingest` embed-fail is now a hard error.** Pre-v0.0.1,
  embedding-generation failure stored a memory with `embedding=None`
  (soft-fail). v0.0.1 converges with file-ingest's hard-error
  semantic; `memory_ingest` now returns `error: "ingestion failed"`
  with no `entry_id` when embedding generation fails.
- **`memory_ingest` entity tags are now lowercased before storage.**
  Pre-v0.0.1, the MCP path stored entities in raw case while
  file-ingest lowercased them — `check_contradictions` then failed
  to match across surfaces. Both paths now lowercase consistently.

### Internal

- `Db::search_fts` and `Db::search_vector` downgraded to `pub(crate)`.
  Direct callers would bypass the `memory_search_audited` orchestrator
  and silently break the audit invariant. No external behavior change.
- `PRAGMA foreign_keys = OFF` now written explicitly in
  `run_migrations` (was relying on the rusqlite/SQLite default).
