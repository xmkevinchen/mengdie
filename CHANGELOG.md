# Changelog

All notable changes to Mengdie are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project follows [semantic versioning](https://semver.org/).

## Unreleased

(nothing scheduled — next sprint not started)

---

## v0.0.1 — 2026-05-10 (rebuild branch ship)

**Branch**: `feature/v0.0.1-rebuild` (73 commits 2026-04-30 → 2026-05-10,
including this CHANGELOG entry).
**Theme**: minimum-viable AE-brain that avoids re-inventing wheels —
narrow OSS adoption, keep working in-house code (per
`docs/v0.0.1-rebuild-plan.md` thesis and `docs/discussions/026-rust-
oss-survey/analysis.md` 14-library scorecard).

### Added

- **F-006 (BL-026): sqlite-vec adoption.** Replaces the 264 LoC
  full-table-scan brute-force cosine similarity in `src/core/vector.rs`
  with `sqlite-vec` v0.1.9's `vec0` virtual table + triggers for the
  vector index. Loads via `rusqlite` extension API at DB-open time.
  fastembed-rs unit-vector invariant asserted at the embedder boundary
  (`assert!`, not `debug_assert!`) so the similarity formula's
  unit-norm assumption can never silently drift. **Cargo.toml net
  change: +1 line** (`sqlite-vec = "0.1.9"`).
- **F-005 (BL-014): `mengdie audit-stats` CLI subcommand.** Reads
  `audit_count`, `link_count`, `oldest_row`, `newest_row`,
  `supersession_count_30d`, `audit_write_failures` from the F-002
  audit tables. Emits human table + script-facing JSON via
  `--format {table,json}`. Health enum: `ok` / `not_yet_triggered` /
  `degraded` with actionable hint. Note: shipped as `audit-stats`,
  not `doctor` (working name in `docs/v0.0.1-rebuild-plan.md`) —
  operator convention favored aligning with the underlying
  audit-table feature.
- **Plan 019 (BL-027 Path B): synthesis CLI structured-output.**
  Replaces ~30 LoC brace-depth scanner in `src/core/synthesis.rs` with
  claude-CLI's native `--json-schema` + `--output-format json` flags.
  Schema lives in `resources/synthesis-output-schema.json` (embedded
  via `include_str!`), **flat-shape with `skip:bool` discriminator**
  (the originally-planned `oneOf` was rejected mid-execution —
  Anthropic API does not accept top-level `oneOf`/`allOf`/`anyOf` in
  tool `input_schema`; see `docs/spikes/019-rate-limit-measurement.md`
  schema-shape post-mortem). `LlmProvider::complete_structured` added
  as a sibling trait method with default impl returning
  `UnknownProvider`. New error variants `StructuredOutputMissing` +
  `StructuredOutputWrapperInvalid` both carry "(verify claude >=
  2.1.138 supports --json-schema)" diagnostic suffix. Production run
  on personal KB: 5/5 syntheses, 0 errors, ~$0.40 USD-equivalent,
  ~275K tokens (Pro flat-fee actual: $0). **Cargo.toml net change: 0
  lines** (CLI flags, no new deps).
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
- **Plan 019: brace-depth scanner deleted from `src/core/synthesis.rs`**.
  `extract_first_json_object` (~30 LoC byte-scanner with string-state
  tracking) replaced by direct `serde_json::from_str` against
  claude-CLI's `--json-schema` structured-output payload. 7 obsolete
  unit tests removed (preamble/postamble/markdown-fence/escape-quote/
  inner-brace cases — all structurally impossible under token-decode
  constraint); 1 test repurposed with audit-trail comment preserving
  the original design-intent name.

### Retrospective (v0.0.1 cycle close)

Plan 019 final review captured three process-level findings worth
recording at the version boundary:

1. **Plan-review must run actual API probes for provider-specific
   schema assumptions.** The 9-reviewer plan-review for BL-027 endorsed
   a `oneOf` schema design that Anthropic API later rejected at runtime;
   no reviewer had probed the actual API subset. Citation alone is
   sufficient for plain object schemas; **probe required for
   `oneOf`/`anyOf`/`allOf`/`const`/`additionalProperties:false`/
   conditional required/wrapper shape/error-shape assumptions**. Costs
   ~5 min upfront; saved hours mid-execution would-have-been.
2. **Reject path-out-of-scope arguments framed by phantom metered cost
   when the operator runs on flat-fee subscription.** Plan 019's "Out
   of scope" rejected Path C (direct Anthropic HTTP API) citing
   per-token cost — irrelevant under Claude Code Pro. Correct
   rejection rationale would have been "preserves credential-delegation
   privacy posture" (architectural), not "$0.24 per call" (false under
   the deployment target).
3. **Anchor BL triggers to code artifacts, not external events.**
   BL-039 originally specified "when second LLM provider lands" — a
   vague human-readable event. Sharpened at v0.0.1 ship to three
   concrete code-artifact tripwires (second `build_provider` match arm
   / second `impl LlmProvider for X` / Cargo.toml non-claude LLM dep)
   plus an inline NOTE comment at the `build_provider` site itself —
   the comment IS the operational tripwire, no external review cadence
   needed.

These findings live in `docs/reviews/019-synthesis-cli-json-schema.md`
"Retrospective findings" + Mengdie memory (3 captures, IDs in the same
review file). Forward sprints should consult them.

### Branch state at v0.0.1 cut

- 73 commits ahead of pre-rebuild `main` (2026-04-30 → 2026-05-10).
- 6 features done (F-001 spike + F-002 audit substrate + F-003
  retrieval/ingest consolidation + F-004 doc structure + F-005
  audit-stats CLI + F-006 sqlite-vec adoption) + plan 019 (BL-027
  Path B synthesis structured-output).
- Cargo.toml net delta: +1 line (`sqlite-vec = "0.1.9"`).
  Within the +1~+3 budget set by `docs/v0.0.1-rebuild-plan.md`.
- 13 deferred BLs filed under `docs/backlog/unscheduled/` (BL-029 ~
  BL-041); all carry trigger conditions and are explicit defer-not-now.

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

