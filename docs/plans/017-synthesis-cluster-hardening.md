---
id: "017"
title: "v0.8.0 synthesis cluster hardening — cluster-hash dedup + audit subcommand + source_type surfacing"
type: plan
created: 2026-04-23
status: reviewed
discussion: "docs/discussions/022-synthesis-provenance-options/"
---

# Feature: v0.8.0 synthesis cluster hardening

## Goal

Make the synthesis subsystem operationally legible by (a) replacing `content_hash` with a source-ID-derived cluster hash as the dedup key so prompt iterations update existing syntheses instead of accumulating zombie rows, (b) adding a read-only `mengdie synthesis-audit` subcommand to inspect a synthesis alongside its source memories for fidelity spot-checks, and (c) surfacing the `source_type` field in `mengdie search` / `mengdie list` output so operators can visually distinguish syntheses from primary sources.

## Background

Source: discussion 022 conclusion Topic 1 (ship Options 1 + 4 reinterpreted, reject Option 5, defer 2+3) + `BL-synthesis-dedup-key` option A (preferred per BL body). Discussion 021 Next Step 7 defined this plan's scope.

Three scope items, all in the synthesis subsystem:

1. **BL-synthesis-dedup-key (option A: cluster-hash dedup)** — replace `content_hash` with `sha256(sorted-unique(source_ids).join(","))` as the `insert_synthesis_with_links` conflict key. Requires a v5 schema migration that: (a) adds `synthesis_cluster_hash TEXT` column, (b) coalesces any legacy duplicate-cluster rows, (c) backfills existing synthesis rows (27 currently, per discussion 022 prevalence data), (d) creates a partial unique index on the new column, (e) **makes the existing `idx_memory_content_hash` partial** to exclude synthesis rows so two syntheses with different clusters but coincidentally identical content can coexist, (f) adds a CHECK constraint on `source_type` allowed values so the partial indexes' `WHERE source_type = 'synthesis'` predicate can't be escaped by a buggy UPDATE. All migration steps wrapped in a single transaction.
2. **BL-synthesis-provenance Option 1 (audit subcommand)** — flat `mengdie synthesis-audit <syn-id>` subcommand (no nested `synthesis` namespace — matches existing CLI convention). Reads a synthesis + its linked source memories via `memory_synthesis_links` and prints both so an operator can judge fidelity. Read-only. Framed as scaffolding for future Options 2/3 ship-gate data collection, not a standalone value proposition.
3. **BL-synthesis-provenance Option 4 reinterpreted (surface source_type)** — extract a testable `format_search_line` helper and add `source_type` to `cmd_search` output (`src/bin/cli.rs:610-625`) and `cmd_list` output (`src/bin/cli.rs:571`). Synthesis rows have `source_type = "synthesis"` already stored (verified at `dreaming.rs:564`, `db.rs:1064`). `SearchResultItem` (`src/core/mcp_tools.rs:131`) and `MemoryEntry` (`src/core/db.rs:32`) both already carry the field — just needs display.

**Bundle vs split rationale** — plan-review-updated: challenger correctly noted that "same files touched" is a weak unifier. The stronger justification is the invariant coupling: Step 1's migration establishes the column and partial indexes that Step 3's `insert_synthesis_with_links` semantic change depends on; Step 4's regression test proves both work together. Steps 5 (audit subcommand) and 6 (formatter) are genuinely independent but small; splitting them into a separate plan would add cross-plan coordination overhead without reducing blast radius. Bundle retained but explicitly scoped to the "synthesis cluster hardening" invariant, not file-touch overlap.

**Explicitly out of scope**:
- Option 2 (LLM verification): deferred per codex data-gating rule.
- Option 3 (downrank synthesis rows in search): deferred per 40% prevalence argument.
- Option 5 (`KnowledgeType::Synthesized` enum variant): rejected on axis discipline; architect dissent preserved.
- `BL-synthesis-preload-db-miss-edge`: descoped to `unscheduled/`.
- **`SYSTEM_PROMPT` + `EXPECTED_SYSTEM_PROMPT` unchanged** — operator workflow is invariant under this plan. A future prompt-edit PR still updates the regression constant independently; plan 017 only ensures the re-synthesis UPDATES the existing row instead of creating a zombie.
- `PRAGMA foreign_keys = ON` (dep-analyst P2) — file as separate backlog item. Current FK declarations on `memory_synthesis_links` are unenforced. Not a plan 017 blocker.
- Embedding population on synthesis rows (challenger P2) — synthesis rows ship with `embedding: None` today; that is an existing gap (`dreaming.rs:561-574`) not introduced by this plan. Document but don't fix.
- Backfill CTE optimization (gemini P2) — defer until corpus exceeds 1000 synthesis rows; the row-by-row backfill loop is acceptable for the current 27-row scale.

## Steps

### Step 1: v5 schema migration — column + partial indexes + CHECK + helper + transactional backfill (AC1, AC2)

This is the heaviest step. All SQL wrapped in one transaction (gemini P1.1): either the entire migration succeeds or it rolls back cleanly. The `compute_synthesis_cluster_hash` helper is defined HERE (not in a later step) so the backfill can call it directly — resolves challenger's step-ordering chicken-and-egg.

- [x] **Define helper**: add `pub fn compute_synthesis_cluster_hash(source_ids: &[String]) -> String` in `src/core/schema.rs` (colocated with existing `compute_content_hash` — codex P3a consistency). Implementation:
  ```rust
  let mut ids: Vec<String> = source_ids.to_vec();
  ids.sort();            // lexicographic on String — stable + deterministic
  ids.dedup();           // handle duplicates in input defensively
  let joined = ids.join(",");
  // sha256 hex of joined, same pattern as compute_content_hash
  ```
  Add a unit test at the bottom of `schema.rs` asserting a known input/output pair (e.g., `["a", "b"]` → specific hex string). Also assert `["b", "a"]` produces the same hash (order independence) and `["a", "a", "b"]` produces the same hash as `["a", "b"]` (dedup). Reject empty input with a `debug_assert!` or document the convention.
- [x] **Bump `SCHEMA_VERSION`** from 4 to 5 in `schema.rs:4`.
- [x] **Wrap v4→v5 block in an explicit transaction via `execute_batch`** (gemini P1.1 + adversarial P1 compile fix): `run_migrations(conn: &Connection)` takes an immutable reference; `Connection::transaction()` requires `&mut self` and is NOT usable without a signature change that ripples to all call sites. Use SQL-level transaction control instead: `conn.execute_batch("BEGIN TRANSACTION;")?;` at the top of the v4→v5 block, `conn.execute_batch("COMMIT;")?;` at the bottom. On any `?` short-circuit during the block, the outer `Result<()>` error propagates; wrap the body in a closure or use `(|| -> Result<()> { ... })()` so that errors trigger `conn.execute_batch("ROLLBACK;")` before returning. Pattern:
  ```rust
  conn.execute_batch("BEGIN TRANSACTION;")?;
  let migration_result = (|| -> rusqlite::Result<()> {
      // all v4→v5 sub-steps here, using ? freely
      Ok(())
  })();
  match migration_result {
      Ok(()) => conn.execute_batch("COMMIT;")?,
      Err(e) => {
          // ROLLBACK best-effort; if ROLLBACK itself fails, still surface the original error
          let _ = conn.execute_batch("ROLLBACK;");
          return Err(e);
      }
  }
  ```
  This keeps the existing `&Connection` signature intact; no ripple to `Db::open` / `Db::open_in_memory` / tests.
- [x] **Pre-check 1: orphan links** (dep-analyst P1 Q1): `SELECT COUNT(*) FROM memory_synthesis_links l LEFT JOIN memory_entries e ON e.id = l.source_memory_id WHERE e.id IS NULL;`. If non-zero, log `tracing::warn!` with the count, skip backfill for any synthesis whose source set contains an orphan, and abort migration with a clear error. (Do NOT silently produce a degenerate hash — the whole point is to catch corruption loudly.)
- [x] **Pre-check 2: zero-link synthesis rows** (dep-analyst P1): `SELECT id FROM memory_entries WHERE source_type = 'synthesis' AND id NOT IN (SELECT DISTINCT synthesis_memory_id FROM memory_synthesis_links);`. If any row returned, abort migration with an error naming the row IDs. Zero-link syntheses would backfill to `sha256("")` which silently conflates all of them — unacceptable.
- [x] **Pre-check 3: legacy duplicate clusters** (codex P1 + adversarial: spell out the SQL since the plan asks for computation the schema doesn't yet support). Approach: compute each synthesis's cluster in Rust from its link rows, build an in-memory `HashMap<cluster_key, Vec<syn_id>>`, and look for entries with `len() > 1`. Concrete:
  ```rust
  // Fetch all (syn_id, source_id) pairs
  let pairs: Vec<(String, String)> = conn
      .prepare("SELECT synthesis_memory_id, source_memory_id FROM memory_synthesis_links")?
      .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
      .collect::<Result<_, _>>()?;

  // Group source_ids by synthesis_id
  let mut by_syn: std::collections::HashMap<String, Vec<String>> = Default::default();
  for (syn_id, src_id) in pairs { by_syn.entry(syn_id).or_default().push(src_id); }

  // For each synthesis, compute its cluster hash
  let mut by_cluster: std::collections::HashMap<String, Vec<String>> = Default::default();
  for (syn_id, src_ids) in by_syn {
      let hash = compute_synthesis_cluster_hash(&src_ids);
      by_cluster.entry(hash).or_default().push(syn_id);
  }

  // Any cluster with > 1 synthesis is a legacy duplicate
  let duplicates: Vec<(String, Vec<String>)> = by_cluster
      .into_iter()
      .filter(|(_, syn_ids)| syn_ids.len() > 1)
      .collect();
  ```
  For each duplicate cluster: fetch the synthesis rows from `memory_entries`, sort by `created_at` DESC, keep the first (newest), invalidate the rest via `UPDATE memory_entries SET valid_until = ?, invalidation_reason = 'merged by plan 017 cluster-hash migration' WHERE id = ?`. Log each coalesce at `tracing::info!` level with the kept/invalidated IDs. **Cover with a migration test** (see test list below). Heuristic accepted-risk per plan 017 Doodlestein regret: `created_at` is a proxy for "better row" that may not hold under a test-run-then-real-run scenario; at current 27-row corpus scale the probability of actual legacy duplicates is low, and the manual escape hatch is `mengdie invalidate` post-migration.
- [x] **ALTER TABLE** to add `synthesis_cluster_hash TEXT` (nullable — primary sources will have NULL).
- [x] **Make `idx_memory_content_hash` partial** (architect + dep-analyst P1): `DROP INDEX IF EXISTS idx_memory_content_hash; CREATE UNIQUE INDEX idx_memory_content_hash ON memory_entries(project_id, content_hash) WHERE source_type != 'synthesis';`. This exempts synthesis rows from content_hash dedup — they dedup on cluster_hash instead. Primary-source behavior unchanged.
- [x] **Backfill existing synthesis rows** (row-by-row loop — batched CTE deferred per gemini P2): for each row with `source_type = 'synthesis' AND synthesis_cluster_hash IS NULL`, query its `source_memory_id` list from `memory_synthesis_links`, call `compute_synthesis_cluster_hash`, UPDATE the row. Use a prepared statement to minimize overhead.
- [x] **Create partial unique index with `IS NOT NULL` guard** (gemini P1.2): `CREATE UNIQUE INDEX IF NOT EXISTS idx_synthesis_cluster ON memory_entries(project_id, synthesis_cluster_hash) WHERE source_type = 'synthesis' AND synthesis_cluster_hash IS NOT NULL;`. The `IS NOT NULL` clause prevents SQLite's multi-NULL-are-unique behavior from allowing duplicate null-hash rows.
- [x] **Add CHECK on `source_type` allowed values via trigger** (challenger + gemini P2, adversarial P3 pre-existing-data-safety): SQLite does NOT support `ALTER TABLE ... ADD CONSTRAINT CHECK` on existing tables — confirmed. The fallback is a BEFORE INSERT + BEFORE UPDATE trigger raising `RAISE(ABORT, ...)` on invalid values. **Pre-check required before installing the trigger** (adversarial P3): `SELECT DISTINCT source_type FROM memory_entries WHERE source_type NOT IN ('conclusion', 'review', 'plan', 'analysis', 'retrospect', 'synthesis');` — if non-empty, the migration must abort and surface the offending distinct values (production DB has data the allowlist doesn't cover). On clean pre-check, install:
  ```sql
  CREATE TRIGGER IF NOT EXISTS memory_entries_source_type_check
  BEFORE INSERT ON memory_entries
  FOR EACH ROW
  WHEN NEW.source_type NOT IN ('conclusion', 'review', 'plan', 'analysis', 'retrospect', 'synthesis')
  BEGIN
      SELECT RAISE(ABORT, 'invalid source_type');
  END;
  -- Parallel trigger on UPDATE to prevent mutation to invalid values
  CREATE TRIGGER IF NOT EXISTS memory_entries_source_type_update_check
  BEFORE UPDATE OF source_type ON memory_entries
  FOR EACH ROW
  WHEN NEW.source_type NOT IN ('conclusion', 'review', 'plan', 'analysis', 'retrospect', 'synthesis')
  BEGIN
      SELECT RAISE(ABORT, 'invalid source_type');
  END;
  ```
  Note: the triggers prevent setting `source_type` to a value outside the allowlist but do NOT prevent a synthesis row's `source_type` from being UPDATED to another valid value (e.g., "conclusion"). Full write-once enforcement is deferred; the partial-index caveat is documented in the helper comment and Known Risk below.
- [x] **PRAGMA integrity_check**: run `PRAGMA integrity_check;` before committing the transaction. If it returns anything other than `ok`, roll back and abort migration with the reported errors.
- [x] **Commit transaction + set PRAGMA user_version = 5**.
- [x] **Tests** (all in `src/core/schema.rs` test module):
  - `test_schema_version_is_v5_on_fresh_db` (pattern match at `schema.rs:188`): fresh DB ends at v5.
  - `test_migration_v4_to_v5_happy_path`: seed DB at v4 with 3 synthesis rows + their links (overlapping and non-overlapping source sets), run migration, assert v5, assert each row's `synthesis_cluster_hash` = expected value from helper.
  - `test_migration_v4_to_v5_rejects_orphan_links`: seed with a link pointing to a missing source memory, run migration, assert it aborts with orphan-link error.
  - `test_migration_v4_to_v5_rejects_zero_link_synthesis`: seed with a synthesis row that has zero links, run migration, assert it aborts.
  - `test_migration_v4_to_v5_coalesces_legacy_duplicates`: seed with two synthesis rows covering the same source set (different content_hash, different created_at), run migration, assert older row is invalidated (`valid_until IS NOT NULL`, `invalidation_reason` set) and newer row has the `synthesis_cluster_hash`.
  - `test_schema_v5_partial_indexes_applied`: post-migration, `EXPLAIN QUERY PLAN` for `SELECT * FROM memory_entries WHERE project_id = ? AND content_hash = ? AND source_type != 'synthesis'` hits `idx_memory_content_hash`; same for synthesis via `idx_synthesis_cluster`.
  - `test_check_constraint_rejects_invalid_source_type`: attempt to insert a row with `source_type = 'junk'`, expect trigger ABORT with "invalid source_type" message. Parallel test for UPDATE to invalid value.
  - `test_migration_v4_to_v5_rejects_pre_existing_invalid_source_type`: seed a v4 DB with a row whose `source_type = 'notes'` (outside allowlist), run migration, assert it aborts with the offending value surfaced.

Expected files: `src/core/schema.rs`

### Step 2: Switch `insert_synthesis_with_links` conflict key from `content_hash` to `synthesis_cluster_hash` (AC3)

- [x] **Rewrite `insert_synthesis_with_links`** at `src/core/db.rs:332-389`:
  - Compute `synthesis_cluster_hash` via the Step 1 helper at the top of the function (`use crate::core::schema::compute_synthesis_cluster_hash;` if needed).
  - Declare and document the `source_ids` invariant (codex P2): non-empty + caller-managed uniqueness; the helper's internal `sort + dedup` handles duplicates defensively, but an empty input is a caller bug. Add a runtime check: `anyhow::ensure!(!source_ids.is_empty(), "insert_synthesis_with_links requires at least one source_id");` at the top.
  - Change the INSERT to include `synthesis_cluster_hash` column and the value.
  - Change `ON CONFLICT(project_id, content_hash)` → `ON CONFLICT(project_id, synthesis_cluster_hash)`. DO UPDATE body unchanged — on cluster re-synthesis, update title/entities/embedding/is_longterm, preserve recall stats and id.
  - Add a code comment above the function documenting the `source_type` immutability convention (challenger + gemini P2): "This function writes `source_type = 'synthesis'`. Downstream code must not reclassify synthesis rows — the partial unique indexes `idx_synthesis_cluster` (WHERE source_type = 'synthesis') and `idx_memory_content_hash` (WHERE source_type != 'synthesis') both depend on this invariant. The CHECK constraint on `source_type` values limits the blast radius but does not fully enforce per-row immutability."
- [x] **Unit tests** in `src/core/db.rs` test module:
  - `insert_synthesis_with_links_upserts_on_same_cluster`: call twice with same source_ids, different content text; assert exactly one synthesis row exists post-run (`SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis' AND project_id = ?` = 1), content is the latest value, id stable across calls.
  - `insert_synthesis_with_links_different_clusters_coexist`: call with `[a,b]` then `[b,c]`; assert both rows exist (different cluster_hashes). This also exercises the `idx_memory_content_hash` partial-index fix — if the two syntheses happen to produce identical content text (unlikely but the test can construct this), they coexist.
  - `insert_synthesis_with_links_source_id_order_independent`: call with `[a,b,c]` then `[c,b,a]` same text; assert exactly one row, same id.
  - `insert_synthesis_with_links_empty_source_ids_errors`: call with `vec![]`; assert error per the ensure! guard.

Expected files: `src/core/db.rs`

### Step 3: Add `mengdie synthesis-audit <syn-id>` CLI subcommand (AC4)

- [x] **Add `SynthesisAudit { id: String }` to `Commands` enum** at `src/bin/cli.rs:24` (flat, not nested — architect Q4). User syntax: `mengdie synthesis-audit <id>`.
- [x] **Add `get_synthesis_with_sources(syn_id: &str)` helper** in `src/core/db.rs`. Signature: `fn get_synthesis_with_sources(&self, syn_id: &str) -> anyhow::Result<(MemoryEntry, Vec<MemoryEntry>)>`. Returns the synthesis memory + its N source memories. Errors:
  - `syn_id` not found → `anyhow::bail!("synthesis id not found: {syn_id}")`.
  - Row exists but `source_type != "synthesis"` → `anyhow::bail!("id {syn_id} is not a synthesis row (source_type = {actual})")`.
  - Any linked source memory has been hard-deleted → include a placeholder `MemoryEntry` with title like `"<deleted: id>"` and continue (do NOT abort; matches dep-analyst Q2 graceful-handling note for unenforced FKs).
- [x] **Add `cmd_synthesis_audit(db: &Db, id: &str)`** function following the pattern of other `cmd_*` functions. Output format:
  ```
  === Synthesis ===
  Title: <syn.title>
  Content:
    <syn.content, indented, full text>
  Sources (N):

  --- Source 1/N ---
  Title: <src.title>
  Type: <src.source_type>
  Content: <first 200 chars of src.content, indented>

  --- Source 2/N ---
  ...
  ```
- [x] **Integration test** at `tests/dream_synthesis.rs` (file exists; append to it — new tests NOT `#[ignore]` per dep-analyst Q4): `synthesis_audit_subcommand_prints_synthesis_and_sources`. Pattern from `tests/decay_contract.rs`: seed via `Db::open` + `insert_memory` (for sources) + `insert_synthesis_with_links` (for the synthesis), hold `NamedTempFile` past `Command::output()` (lifetime trap), invoke binary with `synthesis-audit <id>`, assert stdout contains `=== Synthesis ===`, synthesis title, both source titles. Also add a negative-path test: `synthesis_audit_subcommand_errors_on_non_synthesis_id` — call with a primary-source id, assert non-zero exit + clear error message.

Expected files: `src/core/db.rs` (new helper), `src/bin/cli.rs` (subcommand + command function), `tests/dream_synthesis.rs` (2 integration tests)

### Step 4: Surface `source_type` in `mengdie search` and `mengdie list` output + testable formatter (AC5)

- [x] **Extract `format_search_line` helper** (codex P2b — make AC4 verification automated, not manual): move the formatting logic currently inline at `src/bin/cli.rs:610-625` into a `fn format_search_line(r: &SearchResultItem, index: usize) -> String` that returns the 3 lines as one `\n`-joined string. `cmd_search` calls it in a loop and `println!`s the result.
- [x] **Modify the formatter** to include `source_type`: current secondary line is `   source: {file} | entities: {entities} | recalled: {N}x`. New: `   type: {source_type} | source: {file} | entities: {entities} | recalled: {N}x`. The `type:` prefix distinguishes from `source:` (which is the file path).
- [x] **Modify `cmd_list` output** at `src/bin/cli.rs:571` similarly. Note dep-analyst Q5: `cmd_list` uses `MemoryEntry` (not `SearchResultItem`); the field name and access are the same but they're different struct types on different code paths. Either extract a shared `format_memory_row` helper covering both (clean), or touch each formatter independently (simpler). Pick the simpler option unless the formatter logic is substantial.
- [x] **Unit tests** in the test module at the bottom of `cli.rs`:
  - `format_search_line_includes_source_type_synthesis`: construct a `SearchResultItem` with `source_type = "synthesis"`, assert output contains `"type: synthesis"`.
  - `format_search_line_includes_source_type_conclusion`: same for primary source.
  - If `format_memory_row` extracted: parallel tests for `cmd_list`'s formatter.
- [x] **Spot-check** (manual, complementary to tests): `cargo run -- search "test" 2>&1 | head -20` shows the new `type:` column on each result's secondary line.

Expected files: `src/bin/cli.rs`

### Step 5: Regression test — COUNT-based invariant (AC6)

The Step 2 unit tests cover the UPSERT invariant directly. This step adds an INTEGRATION-level regression test that simulates the real prompt-change workflow end-to-end, using the `COUNT=1-per-cluster-per-project` invariant (challenger P1) rather than id-equality (which is implementation-specific to `ON CONFLICT DO UPDATE RETURNING id`).

- [x] **Integration test** at `tests/dream_synthesis.rs` (append, not `#[ignore]`): `cluster_hash_dedup_survives_prompt_change_integration`. Pattern:
  - Seed 3 short-term memories.
  - Call `insert_synthesis_with_links(NewMemory { content: "V1", ... }, source_ids)`.
  - Assert `SELECT COUNT(*) FROM memory_entries WHERE source_type = 'synthesis' AND project_id = ?` returns 1.
  - Call `insert_synthesis_with_links` again with the same source_ids but `content: "V2"`.
  - Re-assert COUNT = 1 (not 2). Fetch the synthesis row, assert `content = "V2"` (latest wins).
  - Assert `memory_synthesis_links` has exactly 3 rows for this synthesis (INSERT OR IGNORE handles repeat source edges).
- [x] **Cross-cluster coexistence test**: `insert_synthesis_with_links_different_source_sets_with_identical_content_coexist`. Construct two synthesis calls with DIFFERENT source sets but IDENTICAL content text (manufacture this — normally the LLM wouldn't produce identical text for different inputs, but the test enforces that `idx_memory_content_hash` partial-index fix from Step 1 actually works). Assert COUNT = 2, different ids, different cluster_hashes.
- [x] **Order-independence test** (restated at integration level): `cluster_hash_stable_across_source_id_order_integration`. Two calls with `[a,b,c]` vs `[c,a,b]` same set; COUNT = 1.

Expected files: `tests/dream_synthesis.rs`

### Step 6: BL close-out + in-place BL body updates (AC7)

- [x] Update `.ae/backlog/v0.8.0/BL-synthesis-dedup-key.md` frontmatter: `status: done`, `closed: 2026-04-23`, `closed_by: plan-017`. Body top: "Shipped in plan 017" blockquote summarizing option A (cluster-hash dedup) was taken; note the idx_memory_content_hash partial-index fix that plan 017 review surfaced.
- [x] Update `.ae/backlog/v0.8.0/BL-synthesis-provenance.md` frontmatter: `status: done`, `closed: 2026-04-23`, `closed_by: plan-017`. Body top blockquote: "Options 1 + 4 (reinterpreted as 'surface source_type') shipped in plan 017 per discussion 022 conclusion. Option 2 (LLM verification) + Option 3 (downrank) deferred pending failure-rate data. Option 5 (new enum variant) rejected on axis discipline. Architect dissent preserved."
- [x] Do NOT `mv` the closed BL files — same convention as plans 015/016 Step 6. File-move belongs to `/ae:roadmap close v0.8.0`.
- [x] Edit `.ae/roadmaps/v0.8.0.md` "Items" table: mark both BL rows `status: done`. **Optional — skip if `/ae:roadmap` will regenerate immediately** (architect Q5 clarification). Skipped: table is `ae:roadmap managed` with explicit do-not-hand-edit marker; regenerates from BL frontmatter on next `/ae:roadmap` invocation.
- [x] **Backlog item filing** (dep-analyst Q2 follow-up): write `.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md` proposing `PRAGMA foreign_keys = ON` in `Db::open` + `Db::open_in_memory`. Trigger: first observed production data corruption traceable to unenforced FK, or next time the schema adds a new FK-bearing table.
- [x] **Backlog item filing** (Doodlestein strategic follow-up): write `.ae/backlog/unscheduled/BL-audit-collection-discipline.md`. Problem statement: plan 017 ships the `synthesis-audit` subcommand as "scaffolding for Options 2/3 ship-gate data collection" (discussion 022 conclusion), but ships zero collection discipline — no cadence, no metric, no named instrument. Without a defined trigger, the audit subcommand is zero-probability data collection. Proposal: add a shell one-liner (e.g., `for id in $(sqlite3 ~/.mengdie/db.sqlite "SELECT id FROM memory_entries WHERE source_type='synthesis';"); do mengdie synthesis-audit "$id"; done`) to ops doc OR a `mengdie synthesis-audit --all` mode. Trigger: operator has audited ≥50% of synthesis rows and recorded results to file (or next time Option 2/3 comes up in a sprint, whichever is first). Baseline: 10/27 manually reviewed per BL-clustering-validation.md.

Expected files: `.ae/backlog/v0.8.0/BL-synthesis-dedup-key.md`, `.ae/backlog/v0.8.0/BL-synthesis-provenance.md`, `.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md` (all local-only, `.ae/` gitignored)

## Acceptance Criteria

### AC1: v5 migration is transactional + idempotent + passes safety guards

**Verification** (all test-based, review-time runnable):
- `cargo test` — the 5 Step 1 migration tests all pass: happy-path v4→v5, orphan-link rejection, zero-link rejection, legacy-duplicate coalescing, CHECK constraint rejection.
- Fresh-DB test confirms `PRAGMA user_version` returns 5.
- Idempotence: running `run_migrations` twice on a fresh DB (simulates re-open) does not error and does not re-run v5 operations.

### AC2: Partial indexes correctly scoped

**Verification**:
- `EXPLAIN QUERY PLAN SELECT * FROM memory_entries WHERE project_id = ? AND content_hash = ? AND source_type != 'synthesis'` hits `idx_memory_content_hash`.
- `EXPLAIN QUERY PLAN SELECT * FROM memory_entries WHERE project_id = ? AND synthesis_cluster_hash = ? AND source_type = 'synthesis'` hits `idx_synthesis_cluster`.
- Two synthesis rows with different cluster_hashes but identical content can INSERT without UNIQUE constraint violation (Step 5's cross-cluster coexistence test exercises this).

### AC3: Re-synthesis of the same cluster updates in place (COUNT=1 invariant)

**Verification**:
- Step 5 integration test `cluster_hash_dedup_survives_prompt_change_integration` passes: two calls with same source_ids and different content yields `COUNT(*) FROM memory_entries WHERE source_type='synthesis' AND project_id=?` = 1, latest content wins.
- Step 5 order-independence test passes.
- Step 2 unit test `insert_synthesis_with_links_upserts_on_same_cluster` passes.
- `source_ids` empty-input guard: `insert_synthesis_with_links_empty_source_ids_errors` passes.

### AC4: `mengdie synthesis-audit <syn-id>` prints synthesis + all source memories; error on non-synthesis id

**Verification**:
- `cargo test --test dream_synthesis synthesis_audit_subcommand_prints_synthesis_and_sources` passes: subprocess invocation via `env!("CARGO_BIN_EXE_mengdie")`, stdout contains `=== Synthesis ===`, synthesis title, both source titles.
- `cargo test --test dream_synthesis synthesis_audit_subcommand_errors_on_non_synthesis_id` passes: non-zero exit + clear error message on primary-source id.

### AC5: `mengdie search` and `mengdie list` output includes `source_type` (formatter unit-tested)

**Verification**:
- `format_search_line_includes_source_type_synthesis` unit test passes: asserts output string contains `"type: synthesis"`.
- `format_search_line_includes_source_type_conclusion` unit test passes.
- Manual spot-check: `cargo run -- search "test"` output shows the `type:` column on each result.

### AC6: COUNT-based regression invariant holds across prompt-change simulation

**Verification**: Step 5 integration tests all pass (noted under AC3 but AC6 is the named invariant). Specifically, the invariant tested is `exactly-one-row-per-cluster-per-project` via `SELECT COUNT(*)`, not `same-id-returned-by-UPSERT`. This makes the test robust against future refactors that might change ON CONFLICT DO UPDATE to INSERT OR REPLACE (which would give a new id but still satisfy the COUNT invariant).

### AC7: BL bodies closed + close-out note + FK backlog filed

**Verification** (review-time checkable):
- `.ae/backlog/v0.8.0/BL-synthesis-dedup-key.md` frontmatter has `status: done` and body has "Shipped in plan 017" blockquote.
- `.ae/backlog/v0.8.0/BL-synthesis-provenance.md` frontmatter has `status: done` and body has the 4-option disposition blockquote.
- `.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md` exists with frontmatter + trigger condition.
- `git diff --name-only` on the plan-017 range returns only: `src/core/schema.rs`, `src/core/db.rs`, `src/bin/cli.rs`, `tests/dream_synthesis.rs`, plus the plan/milestone/step-summaries files.

## Step Dependency Graph

```
Step 1 (v5 migration + helper + partial indexes + CHECK + backfill)
  ├─→ Step 2 (insert_synthesis_with_links uses cluster_hash) — HARD: column + helper exist
  ├─→ Step 5 (COUNT-based regression) — HARD: needs Step 2 semantic change + migration-level coexistence
  └─ helper is also usable by any future synthesis-related code

Step 2 (dedup key change + unit tests)
  └─→ Step 5 (integration test exercises the production call path)

Step 3 (synthesis-audit subcommand) — INDEPENDENT of Steps 1/2:
  adds new surface; doesn't depend on migration. Serialize anyway for /ae:work drift hygiene.

Step 4 (surface source_type) — INDEPENDENT:
  cli.rs-only; could ship before Step 1.

Step 6 (BL close-out + FK backlog) — depends on Steps 1-5 all done.
```

**Serial execution order**: Step 1 → Step 2 → Step 3 → Step 4 → Step 5 → Step 6.

Steps 3 and 4 both touch `src/bin/cli.rs` in non-overlapping regions (Step 3 adds a subcommand enum variant + new function; Step 4 modifies existing display functions). Serialization avoids `/ae:work` drift-detection noise.

## Parallel Strategy

None — 6 serial steps.

## Out of Scope

(Listed in Background section above; enumerated here for reviewer ease.)

- Option 2 (LLM verification) — deferred per codex data-gating rule.
- Option 3 (downrank) — deferred per 40% prevalence.
- Option 5 (`KnowledgeType::Synthesized`) — rejected on axis discipline.
- `BL-synthesis-preload-db-miss-edge` — descoped to `unscheduled/`.
- `SYSTEM_PROMPT` + `EXPECTED_SYSTEM_PROMPT` regression constants — unchanged; operator workflow invariant.
- `PRAGMA foreign_keys = ON` — filed as separate backlog item in Step 6 (dep-analyst Q2 follow-up).
- Embedding population on synthesis rows — pre-existing gap (`dreaming.rs:561-574`), documented but not fixed.
- Backfill CTE optimization — deferred until corpus exceeds 1000 synthesis rows.

## Known Risk / Review Focus

- **Schema v5 is the highest-risk migration in the codebase so far**. Three pre-checks + transactional wrap + PRAGMA integrity_check + 5 tests mitigate most failure modes, but a real production DB may have state none of the tests anticipate. Review focus: read the pre-check logic carefully; verify the abort paths trigger on constructed adversarial seeds. The coalesce behavior for legacy duplicates (Step 1 Pre-check 3) is the most novel piece — its correctness depends on "newer row by `created_at` is the better row" which may not always hold. If the operator disagrees, manual `mengdie invalidate` can override post-migration.
- **Partial unique index SQL syntax**: `CREATE UNIQUE INDEX ... WHERE <expr>` requires SQLite ≥ 3.8. Bundled SQLite in rusqlite v0.39 is well above that, but verify at code time via `conn.execute("CREATE UNIQUE INDEX ... WHERE 1=1", [])` as a smoke check during schema.rs development. If unsupported, fall back to BEFORE INSERT trigger.
- **ALTER TABLE ADD CONSTRAINT CHECK**: SQLite does not support this on existing tables. The plan anticipates a trigger-based fallback. Implementor should check SQLite docs + test with a throwaway DB before committing to the approach in the migration code.
- **Determinism of sort-dedup-sha256**: the helper uses `Vec::sort()` (lexicographic) + `dedup()` (consecutive-only — relies on prior sort). The unit test with `["a", "a", "b"]` covers this. Future refactors must not change the sort key to anything other than default String ordering; the unit test and a code comment lock this.
- **Partial-index source_type stability** (challenger P2 + gemini P2): the CHECK constraint limits allowed values but does not prevent a synthesis row's `source_type` from being UPDATED to another valid value (e.g., "conclusion"). The idx_synthesis_cluster partial index would silently exclude such a row, and a second synthesis for the same cluster could then be inserted without conflict. No current code path does this, but the plan's documentation in Step 2 and a code comment in `insert_synthesis_with_links` warn against it. A trigger could fully enforce "source_type is write-once for synthesis rows" — deferred to a separate plan if this becomes a real concern.
- **Audit subcommand value framing** (challenger P3): the subcommand is deliberate scaffolding for future Options 2/3 ship-gates (codex data-gating rule), not a standalone utility. If Options 2/3 are never triggered, the subcommand is modest-value operator CLI wallpaper. Accepted cost per discussion 022 conclusion.
- **Synthesis rows ship with NULL embeddings** (challenger P2): pre-existing gap in `dreaming.rs:561-574`. Plan 017 does not fix this. Impact: synthesis rows cannot participate in vector similarity search, only FTS. The audit subcommand in Step 3 does not perform search and is unaffected; the formatter in Step 4 does not either. If a future plan needs synthesis rows to be search-visible via vector, it must embed them at insert time.
- **The test coverage expanded significantly** from the first draft (5 migration tests + 4 unit tests in db.rs + 4 integration tests + formatter unit tests). Review focus: are all tests necessary, or is there redundancy that could be trimmed? The answer is "yes necessary" given the number of safety guards being validated, but a reviewer should sanity-check that the tests aren't over-testing the same invariant from multiple angles.
