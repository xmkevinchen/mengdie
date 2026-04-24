# Plan 017 — Step Summaries

## Steps 1+2 — v5 schema migration + cluster-hash dedup semantic change (commit: TBD)

**Combined into one commit**: Steps 1 and 2 are not independent in test-suite terms. Step 1 alone (making `idx_memory_content_hash` partial to exclude synthesis rows) removes the existing synthesis dedup guarantee; Step 2 restores it via `idx_synthesis_cluster`. The existing `test_synthesis_rerun_is_idempotent` test fails with Step 1 alone and requires Step 2 to pass — so splitting would leave CI red mid-commit.

**Decisions**:
- **Transaction approach via `execute_batch`** (plan 017 review adversarial P1): `run_migrations` takes `&Connection`; `Connection::transaction()` requires `&mut self`. Rather than ripple the signature change to `Db::open` + `Db::open_in_memory` + every test, used SQL-level `BEGIN TRANSACTION` / `COMMIT` / `ROLLBACK` via `execute_batch`. On error mid-migration, best-effort ROLLBACK runs; original error propagates.
- **Return type changed from `rusqlite::Result<()>` to `anyhow::Result<()>`**: `ModuleError(String)` variant (for custom migration abort messages) is feature-gated behind `vtab` — not in our build. `anyhow::bail!` is simpler. Callers (`Db::open` / `Db::open_in_memory`) already use `.with_context(...)?` / `?` with anyhow, so no change needed there.
- **Column-add idempotence via `column_exists` guard** (for the existing `test_migration_from_v3_preserves_data` test that downgrades and replays v4): SQLite has no `ADD COLUMN IF NOT EXISTS`, so skip the ALTER when the column is already there. Matches the v2 migration's existing pattern.
- **Partial index approach over cluster-hash-in-content-hash alternative** (plan 017 review architect + dep-analyst convergent P1): `idx_memory_content_hash` is now `WHERE source_type != 'synthesis'`; `idx_synthesis_cluster` is `WHERE source_type = 'synthesis' AND synthesis_cluster_hash IS NOT NULL`. Two cleanly-scoped dedup domains. Cost: `insert_memory` and `insert_memory_resolving` ON CONFLICT clauses needed the WHERE predicate spelled out (SQLite upsert with partial index requires matching the partial index WHERE).
- **Triggers (not CHECK constraint) for source_type allowlist**: SQLite doesn't support `ALTER TABLE ... ADD CONSTRAINT CHECK`. BEFORE INSERT + BEFORE UPDATE OF source_type triggers RAISE(ABORT) on invalid values. Allowlist centralized in `ALLOWED_SOURCE_TYPES` constant.
- **`seed_v4_db` helper disables `PRAGMA foreign_keys` explicitly**: dep-analyst said FKs aren't enforced, but rusqlite's `bundled` feature may compile SQLite with FK enforcement ON by default. Orphan-link pre-check tests need to insert intentionally-inconsistent links, so the helper turns FK off for seed realism.

**Rejected**:
- Changing `run_migrations` signature to `&mut Connection` — architect called it "ripples to all call sites"; adversarial flagged the compile error. `execute_batch` transaction is the idiomatic Rust-agnostic fix.
- Using `rusqlite::Error::ModuleError` for custom abort messages — feature-gated, not compiled in.
- Splitting Steps 1 and 2 into separate commits — breaks CI in the middle (test `test_synthesis_rerun_is_idempotent` fails between Step 1 and Step 2).
- `INSERT OR REPLACE` semantics for synthesis UPSERT — would delete+reinsert, losing the id. ON CONFLICT DO UPDATE preserves the id (implementation detail, but used by existing tests).

**Cross-step deps**:
- `compute_synthesis_cluster_hash` helper in `src/core/schema.rs:57` — used by `insert_synthesis_with_links` (Step 2), the v5 migration backfill, the legacy-duplicate coalesce pre-check, and all Step 2 unit tests.
- `ALLOWED_SOURCE_TYPES` constant in `schema.rs:11-18` — used by the triggers + the invalid-source_type pre-check. Any future source_type addition must update this constant AND the trigger bodies (the triggers interpolate the list at install time via `format!`).
- Step 3 (audit subcommand) will need to query synthesis sources via the existing `memory_synthesis_links` table + the new `synthesis_cluster_hash` is informational not used in audit.
- Step 5 (regression test) semantic invariant is `COUNT=1-per-cluster-per-project` — the unit tests in this commit already establish it at the `db.rs` unit-test level; Step 5 adds the integration-level mirror in `tests/dream_synthesis.rs`.

**Test coverage added (16 new tests)**:
- `test_schema_version_is_v5_on_fresh_db` (renamed from v4)
- `test_compute_synthesis_cluster_hash_order_independent`
- `test_compute_synthesis_cluster_hash_dedups_input`
- `test_compute_synthesis_cluster_hash_known_value` (locks `sha256("a,b") = 1eb7c54d...`)
- `test_migration_v4_to_v5_happy_path`
- `test_migration_v4_to_v5_rejects_orphan_links`
- `test_migration_v4_to_v5_rejects_zero_link_synthesis`
- `test_migration_v4_to_v5_rejects_pre_existing_invalid_source_type`
- `test_migration_v4_to_v5_coalesces_legacy_duplicate_clusters`
- `test_trigger_rejects_invalid_source_type_on_insert`
- `test_trigger_rejects_invalid_source_type_on_update`
- `test_idx_memory_content_hash_is_partial_after_v5`
- `test_idx_synthesis_cluster_is_partial_with_not_null_guard`
- `insert_synthesis_with_links_upserts_on_same_cluster`
- `insert_synthesis_with_links_different_clusters_coexist`
- `insert_synthesis_with_links_source_id_order_independent`
- `insert_synthesis_with_links_rejects_empty_source_ids`

**Actual files**: `src/core/schema.rs`, `src/core/db.rs`

---

## Step 3 — synthesis-audit subcommand (commit: TBD)

**Decisions**:
- `SynthesisAudit { id: String }` flat subcommand at `src/bin/cli.rs:145` — matches existing flat-Commands convention. Usage: `mengdie synthesis-audit <id>`.
- New `Db::get_synthesis_with_sources(id)` helper at `src/core/db.rs:398` joins `memory_entries` with `memory_synthesis_links`. Returns `(synthesis, sources)`. Errors with clear messages on unknown id and non-synthesis id. For a linked source memory that's been hard-deleted, returns a placeholder `MemoryEntry` with title `"<deleted: {id}>"` rather than aborting — graceful degradation.
- Output format: `=== Synthesis ===` header + ID/Title/Project/Entities/Recalled/Long-term + indented Content, then per-source `--- Source N/M ---` + ID/Type/Title/Preview (first 200 chars with `[…]` truncation indicator).
- Integration tests append to existing `tests/dream_synthesis.rs` (file already covers `e2e` with `#[ignore]`; new tests are NOT ignored — they seed via `insert_*` directly, no LLM required — dep-analyst Q4 confirmed).

**Rejected**:
- Nested `Synthesis { subcommand: SynthesisSubcommand }` pattern — architect Q4; no nesting precedent, single leaf command today.
- Abort on hard-deleted source memories — chose placeholder instead so audit output is still usable on a corrupted DB.

**Cross-step deps**:
- `get_synthesis_with_sources` helper could be reused by Step 4's source_type formatter if it ever shows linked sources (not currently — Step 4 is just displaying `source_type` on the row itself).
- Integration test fixture pattern (tempfile + `env!("CARGO_BIN_EXE_mengdie")` + subprocess) available as template for Step 5's regression tests.

**Real-world signal captured during manual smoke test**: the user's production DB at `~/.mengdie/db.sqlite` has an existing synthesis row (`529d3212-e809-4b81-a1f5-e15143df5128`) with zero entries in `memory_synthesis_links`. Running `./target/debug/mengdie synthesis-audit <any-id>` triggers the v5 migration on that DB, and the zero-link pre-check fires with a clear error. This is the migration safety net working exactly as designed — documented in the commit message as a real-world datum for the user to resolve (delete the orphan synthesis or restore its links) before the production migration runs.

**Actual files**: `src/core/db.rs` (helper), `src/bin/cli.rs` (subcommand + cmd function), `tests/dream_synthesis.rs` (3 integration tests)

---

## Step 4 — Surface source_type in mengdie search + list (commit: TBD)

**Decisions**:
- Extracted `format_search_result(r, rank) -> String` in `src/bin/cli.rs` (pub(crate) for testability — codex P2b mandatory). Returns 3-line joined string; cmd_search loops and `println!`s + blank line.
- Search output metadata line: `type: {source_type} | source: {source_file} | entities: {e} | recalled: Nx`. `type:` (enum) deliberately distinct label from `source:` (file path) — avoids operator confusion per dep-analyst Q5.
- `cmd_list` table gains an "Origin" column (source_type) and renames the ambiguous "Type" header to "Knowledge" (knowledge_type). Width bumped from 90 → 100 dashes. Columns: `ID | Title | Knowledge | Origin | Recall | LT | Source`.
- 4 unit tests on `format_search_result`: `synthesis` surfaces as `type: synthesis`, `conclusion` surfaces identically, `type:` distinguishes from `source:`, snippet stays capped at 100 chars + single-line.

**Rejected**:
- Shared `format_memory_row` helper covering both cmd_search and cmd_list — they use different types (`SearchResult` vs `MemoryEntry`) and different output shapes (prose vs table). Extracting common code would bloat for marginal reuse; each formatter stays independent.
- Hardcoded `[SYN]` prefix on title per the BL's literal Option 4 — rejected in discussion 022 Round 2; reinterpretation chosen (surface source_type field via dedicated label).

**Cross-step deps**: none downstream. Step 5's regression test covers different invariants (cluster dedup) and doesn't touch CLI output.

**Actual files**: `src/bin/cli.rs`
