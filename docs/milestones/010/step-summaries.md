# Plan 010 (BL-007 Dream Synthesis) — Step Summaries

## Step 1 — schema v4 + NewMemory.is_longterm + SourceType::Synthesis (commit: 7b3a876)

**Decisions**:
- Picked path (a) from the plan's MF-1 choice: added `is_longterm: bool` directly as a field on `NewMemory` (not a post-insert UPDATE). Threaded through the SQL INSERT columns, VALUES placeholder, and ON CONFLICT DO UPDATE SET clause in both `insert_memory` and `insert_memory_resolving`. Single atomic write — no race windows.
- Stored as `mem.is_longterm as i64` to match the existing INTEGER NOT NULL DEFAULT 0 schema column. Round-trips correctly via `get_memory` (bool ← i64).
- `is_longterm: false` default at every pre-existing `NewMemory { ... }` callsite: ingest.rs, mcp_tools.rs's ingest tool, plus 5 test-only modules (clustering, contradiction, dreaming, search, vector). Drift approved in the commit — unavoidable compile-cascade from adding a required struct field.

**Rejected**:
- Post-insert `UPDATE memory_entries SET is_longterm = 1 WHERE id = ?` — would have created a two-call race window between memory insert and the flag update. Inline field is cleaner.
- `#[serde(default)] is_longterm: bool` — `NewMemory` isn't directly Deserialize'd today; the serde plumbing would be dead code. If that changes in BL-007 Step 3 (unlikely), revisit.

**Cross-step deps**:
- `NewMemory.is_longterm: bool` is the field Step 3 will set to `false` on every synthesis insert (reversed from the plan's original `is_longterm=1` per cross-family review).
- `SourceType::Synthesis` variant is available to Step 3 for the synthesis insert path.
- `memory_synthesis_links` table exists after migration v4; Step 3 will insert rows here via the atomic `insert_synthesis_with_links` helper.

**Actual files**: `src/core/schema.rs`, `src/core/db.rs`, `src/core/mcp_tools.rs`, `src/core/ingest.rs`, plus test-only drift in `src/core/{clustering,contradiction,dreaming,search,vector}.rs`.

## Step 2 — pure synthesis seam: prompt + brace-depth parser (commit: 7427b27)

**Decisions**:
- Brace-depth counter extractor for JSON object (plan's explicit Must-Fix replacement for `find('{') + rfind('}')`). Single O(n) pass over bytes — safe because `{` and `}` are ASCII single-byte. The counter correctly handles inner braces in the content field (regression-tested).
- Strict schema via `RawJson { title: Option<String>, content: Option<String>, entities: Option<Vec<String>> }`. Entities-as-objects (`[{"tag":"x"}]`) correctly surfaces as `InvalidJson` at the serde layer.
- 200-char title hard cap applied post-parse via silent truncation — matches plan's "in case LLM ignores the 80-char soft cap."
- `const SYSTEM_PROMPT` lifted verbatim from the plan; regression test compares against a duplicate `const EXPECTED_SYSTEM_PROMPT` literal so any accidental edit to the prompt fails the test with a clear diff.

**Rejected**:
- JSON-string-aware escape tracking in the brace counter — overkill. If the counter miscounts (rare: content contains unbalanced `{` or `}` inside a string literal), serde_json's own parse rejection is the correct fallback (counted as `InvalidJson` by callers).
- `tests/clustering_db.rs` style external test file — co-located `#[cfg(test)] mod tests` keeps the parser and tests in one reviewer's field of view.

**Cross-step deps**:
- Public surface (`SynthesisInput`, `SynthesisDraft`, `SynthesisError`, `build_synthesis_prompt`, `parse_synthesis_response`) is what Step 3's `run_synthesis_pass` imports.
- `SynthesisInput` borrows `MemoryEntry` from `crate::core::db` — Step 3's bulk `get_memories_by_ids` helper must return `Vec<MemoryEntry>` to feed this directly.

**Actual files**: `src/core/synthesis.rs` (new), `src/core/mod.rs` (registration).

## Step 3 — run_synthesis_pass + CLI wiring + e2e test (commit: 6d52bda)

**Decisions**:
- Synthesis pass lives in `src/core/dreaming.rs` alongside the existing promotion pass (rather than splitting to `dream_pipeline.rs`). File is ~580 lines after Step 3, still manageable; splitting would just fragment the "dream" concept across two files for no readability gain.
- `insert_synthesis_with_links` is a new Db method, not a free function — it needs `conn.transaction()` scoped under the existing `Arc<Mutex<Connection>>`, which only Db owns. Same rationale for `get_memories_by_ids` (one lock acquisition per call is a Db-method pattern).
- `count_synthesis_links` added as a public helper so external integration tests (tests/dream_synthesis.rs) can verify link-row presence without needing `lock_conn` access — that method is `pub(crate)` and external tests can't reach it.
- `fn main` switched to `#[tokio::main(flavor = "current_thread")]` per the plan review. Current-thread scheduler keeps the single-threaded cost profile of the previously-proposed nested-runtime approach without double-init risk. Other sync subcommands unchanged — they just don't `.await`.
- `--dry-run` implies `--synthesize` (documented in the help string) — running `mengdie dream --dry-run` alone should show prompts; forcing the user to pass both flags would be annoying UX. The opt-in default of `--synthesize` still holds for non-dry-run runs.

**Rejected**:
- Per-call retry inside `run_synthesis_pass` — explicit non-goal per plan. One attempt per cluster per pass; re-runs are safe via content_hash dedup.
- Auto-invalidation of source memories after synthesis — non-goal per plan. Originals stay valid; synthesis is additive.
- Streaming LLM output to stdout during dry-run — plan just says "print prompt, skip LLM." Kept simple: println + tracing::info.
- Using `list_memories(Some(...))` inside `run_synthesis_pass` — `get_memories_by_ids` is more precise (one IN-query vs full project scan) and the plan explicitly required the bulk helper.

**Cross-step deps**:
- `SynthesisResult` struct is `#[derive(Debug, Default, PartialEq, Eq)]` so future tooling can compare expected-vs-actual counts in integration or ae:review harnesses.
- The Db helpers (`get_memories_by_ids`, `insert_synthesis_with_links`, `count_synthesis_links`) are public and reusable for BL-012 (RAG) and any future batch-consolidation feature.
- `cmd_dream`'s new argument list is large (10 params) — kept as flat args (with `#[allow(clippy::too_many_arguments)]`) rather than bundling into a struct, matching the existing `cmd_rename` style. If BL-008 adds more flags, revisit.

**Actual files**: `src/bin/cli.rs`, `src/core/db.rs` (approved drift — plan required the Db helpers but didn't list db.rs in Expected files), `src/core/dreaming.rs`, `tests/dream_synthesis.rs`.

**Test count**: 175 passed + 5 ignored (up from 169 + 4). Includes 6 new tokio::test synthesis pass unit tests + 1 new #[ignore] e2e test.

**AC5 reminder**: After the first real `mengdie dream --synthesize` run on production mengdie data, remember to append a `## BL-007 empirical results` section to `docs/backlog/BL-clustering-validation.md` with (1) threshold bucket + cluster count, (2) cluster quality good/mixed/poor, (3) which deferred triggers fire. See plan 010 AC5.
