# Changelog

All notable changes to Mengdie are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows [semantic versioning](https://semver.org/).

## Unreleased

### Added

- **F-002 (BL-006): persisted domain audit table + link table.** Two new
  SQLite tables (`memory_search_audit`, `audit_returned_facts`) plus 3
  indexes ship via the v6 schema migration. Every `memory_search` call
  now writes one audit row capturing `(query, scope, took_ms,
  searched_at)` and N link rows mapping audit→fact for the caller-visible
  result set. `Db::record_search_audit_best_effort` (and its strict
  helper) on `impl Db` provides the write path; failures emit a
  `tracing::warn!` line and bump the `audit_write_failures` metric
  counter without affecting the search response. Hook fires from both
  the MCP `memory_search` tool and `mengdie search` CLI. Discussion 029
  / plan F-002.
- **F-003 (BL-009 + BL-010): retrieval & ingest layer consolidation.**
  - `search::memory_search_audited(&Db, ...)` orchestrator (free
    function) replaces the duplicated F-002 audit hook in
    `mcp_tools::search` and `cli::cmd_search`. The audit hook now
    fires from a single location, post-`min_score` filter, with the
    F-002 contract preserved verbatim.
  - New types in `core::search`: `FallbackPolicy` (`HybridOrError` /
    `HybridOrFtsOnly`), `SearchRoute` (`Hybrid` / `FtsOnly`),
    `FallbackReason` (`EmbeddingUnavailable`), `MemorySearchOutcome`.
    Per-surface defaults: MCP=`HybridOrFtsOnly`, CLI=`HybridOrError`,
    internal/test=`HybridOrError`.
  - Two public ingest entries in `core::ingest`: `ingest_text(content,
    metadata, project_id)` for plain insert + `ingest_text_with_resolves(...,
    resolves)` for atomic resolve+insert. Shared private `prepare_memory`
    helper does the embed + entity normalization + contradiction check
    once for both surfaces. `ingest_document` becomes a thin
    file-parsing wrapper.
  - `IngestMetadata` struct introduced as the shared shape between
    file-ingest and MCP-ingest paths.
  - Discussion 001 (under F-003 feature dir) / plan F-003.

### Changed

- **F-003: MCP `memory_search` FTS-fallback scores are now [0,1]-normalized**
  via `linear_rescale_normalize` (per-call min/max linear rescale).
  Pre-F-003, FTS-fallback returned raw `bm25_score.abs()` values
  (typically 3-50). The new normalization makes `min_score`
  filtering effective in degraded mode — operators with calibrated
  raw-BM25 thresholds will see different filtering behavior on the
  embed-fail path. Hybrid path (RRF-normalized) is unchanged. Plan
  F-003 Step 4 + discussion 001 Topic 6.
- **F-003: MCP `memory_ingest` embed-fail is now a hard error.**
  Pre-F-003, embedding-generation failure stored a memory with
  `embedding=None` (soft-fail). F-003 converges with the file-ingest
  path's hard-error semantic; `memory_ingest` now returns
  `error: "ingestion failed"` with no `entry_id` when embedding
  generation fails. External MCP callers relying on soft-fail
  semantics will see a behavior change. Plan F-003 Step 6 +
  discussion 001 Topic 4.
- **F-003: MCP `memory_ingest` entity tags are now lowercased before
  storage**. Pre-F-003, the MCP path stored entities in raw case
  while the file-ingest path lowercased them, breaking
  `check_contradictions` matching across surfaces. The shared
  `prepare_memory` helper lowercases for both paths; stored entities
  are consistently `auth,middleware,jwt` regardless of how they were
  ingested. Plan F-003 Step 7.

### Internal

- **F-003: `Db::search_fts` and `Db::search_vector` downgraded to
  `pub(crate)`**. Direct callers of these primitives would bypass
  the `memory_search_audited` orchestrator and silently break the
  F-002 audit invariant. No external behavior change. Plan F-003
  Step 3.
- **F-002 / F-003: `PRAGMA foreign_keys = OFF` now written explicitly
  in `run_migrations`**. Previously relied on rusqlite/SQLite default;
  changed to be runtime-asserted at every connection-open. Closes
  BL-015 (filed at F-002 Step 1, triggered when bundled-rusqlite's
  default tripped F-002 Step 4 unit tests with FK constraint errors).

## v0.8.0 — 2026-04-24

Theme: Decay + Synthesis Hardening + CI unblock. Review-originated
follow-ups from plans 010/012/013 closed, CI expanded past
`cargo fmt`-only. 7 of 9 committed items shipped (2 descoped mid-sprint
to `unscheduled/` when their triggers did not fire).

### Added
- **Exponential decay for Dreaming** (BL-008, plan 013). Formula:
  `effective_relevance = avg_relevance × 2^(-days_since_last_recalled / 60)`
  with half-life of 60 days and a demotion floor of 0.20. Long-term
  memories whose effective relevance falls below the floor have their
  `is_longterm` flag cleared; stored `avg_relevance` is never mutated.
  The same decay multiplier is applied at search time as a post-fetch
  re-rank so stale memories rank lower before the next Dreaming pass
  demotes them. Adds `mengdie dream --decay-dry-run` for operator
  preview and 4 new counters + a `breached_ids` list on `DreamingResult`.
  Structured-JSON event emitted on stderr per pass for machine
  consumers. Operator procedure:
  [`docs/operations/dreaming-decay.md`](docs/operations/dreaming-decay.md).
  Design record: [discussion 019](docs/discussions/019-power-law-decay/conclusion.md).
- **Decay structured-event schema + verify-decay hardening**
  (BL-decay-json-schema-contract + BL-verify-decay-script-hardening,
  plan 015): locked the structured-JSON shape of the decay dry-run
  event so machine consumers can rely on it; hardened
  `scripts/verify-decay.sh` with an explicit approval gate.
- **Decay operations doc polish** (BL-decay-ops-doc-polish, plan 016):
  rewrote `docs/operations/dreaming-decay.md` — added Rollback section,
  plan 013 AC5 post-ship correction (stored avg_relevance is never
  mutated), doc-SQL drift-guard test.
- **Synthesis cluster-hash dedup** (BL-synthesis-dedup-key, plan 017):
  replaced unstable `content_hash` dedup key for synthesis rows with a
  new `synthesis_cluster_hash` column derived from sorted+deduped
  source IDs. Re-synthesis of the same cluster now UPSERTs the
  existing row instead of producing a zombie sibling. Schema bumped
  v4→v5 with 4 pre-checks, transactional migration via
  `execute_batch`, CHECK-via-trigger on `source_type` allowlist,
  PRAGMA integrity_check. `idx_memory_content_hash` is now partial
  (excludes synthesis rows).
- **Synthesis audit subcommand** (BL-synthesis-provenance, plan 017):
  `mengdie synthesis-audit <id>` prints a synthesis row alongside its
  source memories for operator fidelity spot-checks. Graceful
  placeholder for hard-deleted source memories.
- **Surface source_type in search + list** (BL-synthesis-provenance
  option 4 reinterpreted, plan 017): `mengdie search` + `mengdie list`
  output now include a `type:` column distinct from the `source:` file
  path so operators can visually distinguish syntheses from primary
  sources.

### Changed
- **CI runner env unlocked** (006-ci-runner-env-cleanup +
  BL-ci-full-clippy-test, plan 014): fixed the `.cargo/config.toml`
  `-isysroot` CFLAGS leak blocking the Forgejo runner; expanded
  `ci.yml` from `cargo fmt --check` only to full fmt + clippy + test +
  cross-check jobs. Extracted `Embed` trait + `MockEmbedder` so the
  pipeline test suite runs on any CPU without loading the fastembed
  ORT runtime (works around the Ivy Bridge runner's AVX2 SIGILL).

### Descoped (moved to `unscheduled/`, trigger not fired)
- BL-decay-dreaming-pass-optim: premature at current corpus size.
- BL-synthesis-preload-db-miss-edge: depends on a `mengdie delete` /
  `memory_invalidate` CLI subcommand that does not exist.

