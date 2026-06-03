# Changelog

All notable changes to Mengdie are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows [semantic versioning](https://semver.org/).

## Unreleased

(nothing scheduled — next sprint not started)

---

## v0.0.4 — 2026-06-02

**Theme**: a robustness + operability pass over surfaces that already
existed — no new MCP tools, no new CLI subcommands. Schema migrations
v2–v4 are now crash-safe, content-hash re-ingest stops reporting phantom
collisions against tombstoned rows, and the through-line is *no more
silent failures*: the ingest path now signals unmatched supersession
resolves and degraded conflict scans, and both claude-CLI subprocess
paths (non-zero exit, and exit-0 with malformed JSON) now log the
diagnostic they previously discarded.

### Changed

- **`memory_ingest` surfaces silent ingest failures (F-016).** The
  response now reports `unmatched_resolves` — supersession `resolves`
  IDs that matched no live row (typo / cross-project / already
  invalidated / self-supersession); the new memory is still inserted,
  but those old→new edges are not created, instead of the prior silent
  drop. A `conflict_scan_degraded` flag distinguishes an empty
  `conflicts` list that means "no conflicts" from one that means "the
  scan could not run" (DB/index error), so a true duplicate can no
  longer slip through unsurfaced.
- **claude-CLI diagnostics are no longer dropped (F-019, F-020).** When
  the subprocess exits non-zero (F-019) or exits 0 with stdout that
  fails JSON parsing (F-020), the discarded stdout — where claude-CLI's
  `--output-format json` path writes the real error (e.g. `API Error:
  400 ...`) — is now logged via `tracing::warn!` instead of vanishing
  into an empty-stderr black hole. Truncated to 1500 chars.

### Fixed

- **Content-hash re-ingest is tombstone-aware (F-017).** Re-ingesting
  content byte-identical to a previously-superseded (tombstoned) memory
  no longer reports a phantom content-hash collision against the dead
  row or fires a wasted vec0 shadow write. A schema v9 migration
  excludes tombstoned rows from the content-hash conflict check, and the
  `rename` path only treats live↔live hash matches as collisions.
- **Schema migrations v2–v4 are crash-safe (F-018).** Each of the
  v2/v3/v4 migrations now runs inside a transaction and writes
  `PRAGMA user_version` as its final step, so an interrupted upgrade can
  no longer leave the database non-atomically half-migrated with a stale
  (`0`) version stamp. Mirrors the v5–v8 pattern.

### Docs

- `docs/specs/memory_ingest.md` corrected: the `resolves` failure mode
  does NOT roll back the whole transaction (the new memory is inserted;
  unmatched IDs are reported), and the `unmatched_resolves` +
  `conflict_scan_degraded` response fields are now documented (F-016).

### Cargo

- Net delta: **0 new crates**.

---

## v0.0.3 — 2026-05-24

**Theme**: lifts previously internal design and operator references
into a public `docs/` tree — top-level architecture overview, per-tool
MCP (Model Context Protocol) specs, CLI reference, decay-pass operator
runbook, and AE-plugin integration guide — so first-time readers can
land on the repo without spelunking. Also: synthesis rows are now
embedded at creation time (closing a long-standing surface where
Dreaming-generated rows were unreachable via vector search), and the
`memory_invalidate` MCP tool gains a `project_id` override so
long-lived MCP clients can scope invalidations across project
switches.

### Added

- **`mengdie reembed-synthesis` CLI subcommand.** Backfills embeddings
  for synthesis rows whose `embedding` is `NULL` (legacy data from
  pre-fix corpora). `--dry-run` skips the embedder load entirely for
  fast preview. `--project <ID>` scopes the backfill to one project;
  omit to scan all projects in the global DB. Idempotent — re-runs
  find zero rows once fixed.
- **`memory_invalidate` `project_id` override.** New optional
  `project_id: String?` parameter on the MCP tool lets callers scope
  an invalidation to a specific project, overriding the server's
  startup-cached `default_project_id`. Useful when one long-lived
  `mengdie-mcp` instance persists across project switches in the
  host AI tool. **Restart required**: server instances started against
  a prior version will not expose the new parameter to clients until
  the upgraded binary is re-launched.

### Changed

- **Synthesis rows are now embedded at creation time.** The Dreaming
  synthesis pass takes an `Arc<Mutex<Embedder>>` and writes
  `embedding` + `embedding_dim` in the same transaction as the row,
  closing the surface where synthesis rows landed with
  `embedding = NULL` and were unreachable via vector search.
- **`memory_invalidate` cross-project guard on full-UUID branch.**
  When a caller passes a 36-char UUID that belongs to a different
  project than the resolved scope, the call now returns a structured
  error instead of silently invalidating across projects. Mirrors the
  guard already present on `memory_get`. To opt out, pass `project_id`
  explicitly set to the target memory's actual project.

### Docs

The largest visible delta for new readers — five new top-level
documents under `docs/`. Previously these references were only
distributed alongside the source tree as internal working drafts; a
new contributor or operator landing on the repo could not see them.

- **`docs/technical-design.md`** — top-level design overview. Four
  sections (Vision / Architecture / Components & Relations / Known
  Problems) with six Mermaid diagrams (spiral loop, system view,
  storage ER, ingestion pipeline, retrieval pipeline, Dreaming pass).
- **`docs/specs/`** — per-tool MCP specs (one file each for
  `memory_search`, `memory_ingest`, `memory_invalidate`, `memory_get`,
  `memory_status`, `memory_lint`, `memory_entity_facts`). Frontmatter
  / Signature / Params / Returns / Errors / Examples / Notes template.
- **`docs/cli.md`** — operator-facing CLI reference for the `mengdie`
  binary (all subcommands, flags, exit codes, error conditions).
- **`docs/operations/dreaming-decay.md`** — operator runbook for the
  decay pass: required first-run procedure, rollback SQL for
  falsely-demoted memories, metric interpretation guide.
- **`docs/ae-integration.md`** — how the AE plugin
  (`agentic-engineering`) integrates with Mengdie via MCP tools;
  describes the actual integration points already shipped in the AE
  skill set.

### Internal

- Dreaming synthesis call sites refactored to thread the shared
  `Arc<Mutex<Embedder>>` through the synthesis pipeline; CLI surface
  shared between `mengdie dream --synthesize` and the new
  `mengdie reembed-synthesis` backfill path.
- MCP integration test coverage extended with cross-project guard
  cases on `memory_invalidate` (full-UUID, prefix, blocked vs.
  allowed, override semantics).

### Cargo

- Net delta: **0 new crates**. All new functionality rides existing
  transports.

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
