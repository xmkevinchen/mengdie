---
id: "010"
title: "BL-007 — Dream Synthesis (first caller of BL-005 + BL-006)"
type: plan
created: 2026-04-18
status: reviewed
discussion: "docs/discussions/016-dreaming-evolution/"
---

<!--
Plan review summary (bl007-plan-review team, 2026-04-18):
  Reviewers: architect, dependency-analyst, cross-family-fallback
  (claude sonnet via challenger kit — codex account-limited, gemini API
  key invalid this session).

  Must Fix (all applied):
    - NewMemory.is_longterm field gap → added as bool field, threaded
      through insert_memory + insert_memory_resolving.
    - main() sync + nested Runtime::new block_on → switched to
      #[tokio::main(flavor = "current_thread")] + async cmd_dream.
    - LlmProvider::complete return-type description → corrected to
      LlmFuture<'a>.
    - raw.find('{')/rfind('}') parser → replaced with brace-depth
      counter + added inner-braces regression test.
    - --synthesize default true → flipped to opt-in (--synthesize or
      --dry-run required to trigger LLM calls).

  Consider (applied):
    - insert_memory_with_links helper declared required (atomic tx).
    - get_memories_by_ids bulk helper declared required.
    - max_cluster_size=20 guardrail added to bound prompt token budget.
    - is_longterm=1 default reversed to is_longterm=0 — syntheses earn
      promotion via dreaming pass.
    - AC5 added: empirical-results writeback to
      BL-clustering-validation.md after first real dream run.
    - --threshold help string references BL-clustering-validation.
    - Non-goals note: re-running is safe+idempotent via content_hash.
    - Parallel strategy: Steps 1 and 2 explicitly parallel, Step 3 after.

  Consider (rejected):
    - Move SourceType to a types.rs module — later refactor, not BL-007
      scope.
-->


# Feature: BL-007 — Dream Synthesis

## Goal

After the existing promotion pass in `mengdie dream`, cluster each project's
memories via `cluster_memories` (BL-006) and pass each cluster to the
`ClaudeCliProvider` (BL-005) to produce one consolidated "synthesis" memory.
This is the first caller that validates BL-006's design bets (threshold,
seed-ordering, residuals) and BL-005's provider shape.

## Scope boundaries

- **In**: Schema migration v4 (synthesis link table + `SourceType::Synthesis`),
  synthesis orchestration in `dream` command, pure prompt-building and
  response-parsing seams, CLI flags for `--synthesize`/`--no-synthesize`/
  `--dry-run`/`--threshold`, dedup via re-using existing content_hash
  ON CONFLICT, opt-in end-to-end test against a live `claude` CLI.
- **Out**: Power-law decay (keep for a sibling plan BL-008 — same discussion,
  simpler surface), daemon / job queue (Phase 2.2), MCP tool for
  on-demand synthesis (batch only today), OpenAI/Codex providers
  (LlmProvider trait allows them, not this plan), cross-project
  synthesis, schema migration for richer residuals (stays `Vec<String>`
  per BL-006 contract; validate policy here).
- **First caller rationale**: BL-005 and BL-006 were intentionally landed
  as "dead code" modules that needed the first real consumer to validate
  their design bets. BL-007 is that consumer. Findings from this plan
  may feed back into BL-006 / BL-005 via fixup plans.

## Prior Art (from Mengdie KB + project docs)

- `[plan]: expose residuals alongside clusters; don't silently drop`
  (plans/009, 2026-04-18) — residuals policy is deferred to "first caller".
  This plan decides: **residuals are skipped for MVP synthesis**. Policy
  can evolve when we see real dream output.
- `[analyze]: is_longterm flag has zero effect on search` (discussions/009,
  2026-04-06) — cross-family review argued against hardcoding
  `is_longterm = 1` on synthesis rows. This plan defaults
  `is_longterm = false`; synthesis rows earn long-term status through
  the normal dreaming pass (recall + relevance) just like originals,
  rather than being promoted by construction. If BL-009 later gives
  `is_longterm` semantic weight in search, that promotion is a
  quality signal, not a rubber stamp.
- `[analyze]: MCP tool descriptions need 3-4 sentences` (discussions/011,
  2026-04-06) — not applicable directly (no new MCP tool), but informs
  the tone of synthesis content: self-contained, 3-4 sentences,
  attribution-ready.
- `BL-valid-until-boundary` + `BL-clustering-validation` (backlog) —
  this plan's output will generate the signal needed to fire the
  trigger conditions on those items.

## Synthesis Flow (at a glance)

```
mengdie dream [--project PID] [--synthesize | --dry-run]
              [--threshold 0.75] [--min-cluster-size 3] [--max-cluster-size 20]
  │
  ├─ run promotion pass (existing)                              ← no change
  │
  └─ if --synthesize or --dry-run:
       ├─ cluster_memories(db, project, threshold, min_size)    ← BL-006
       ├─ for each Cluster in result.clusters:
       │    ├─ truncate cluster.memory_ids to max_cluster_size
       │    ├─ get_memories_by_ids(cluster.memory_ids)          ← new bulk helper
       │    ├─ build_synthesis_prompt(cluster, memories)
       │    │    → (system_prompt, user_prompt) [pure fn]
       │    ├─ if --dry-run: print prompt, skip LLM, skip writes
       │    ├─ else: provider.complete(system, user).await      ← LlmFuture
       │    ├─ parse_synthesis_response(raw)                    ← brace-depth parser
       │    │    → SynthesisDraft { title, content, entities } [pure fn]
       │    └─ insert_synthesis_with_links(NewMemory{
       │           source_type=Synthesis, knowledge_type=Factual,
       │           is_longterm=false }, source_ids) ← atomic tx
       │
       └─ residuals logged and skipped (MVP policy)
```

## Steps

### Step 1: Schema v4 + `NewMemory.is_longterm` + `SourceType::Synthesis` (AC1)

- [ ] Bump `SCHEMA_VERSION` in `src/core/schema.rs` from `3` → `4`.
- [ ] In `run_migrations`, add the v4 block (gated on `current_version < 4`)
  that creates the link table:
  ```sql
  CREATE TABLE IF NOT EXISTS memory_synthesis_links (
      source_memory_id     TEXT NOT NULL,
      synthesis_memory_id  TEXT NOT NULL,
      created_at           TEXT NOT NULL,
      PRIMARY KEY (source_memory_id, synthesis_memory_id),
      FOREIGN KEY (source_memory_id) REFERENCES memory_entries(id),
      FOREIGN KEY (synthesis_memory_id) REFERENCES memory_entries(id)
  );
  CREATE INDEX IF NOT EXISTS idx_syn_link_source ON memory_synthesis_links(source_memory_id);
  CREATE INDEX IF NOT EXISTS idx_syn_link_synthesis ON memory_synthesis_links(synthesis_memory_id);
  ```
  Note: SQLite enforces FK only when `PRAGMA foreign_keys=ON` is set.
  We do NOT enable that pragma today (schema hasn't needed it). The FK
  declarations are documentation + future-proofing. A synthesis row
  deleted by hand will orphan link rows, but nothing in the current code
  path deletes memories — invalidation sets `valid_until`.
- [ ] Add `Synthesis` variant to `SourceType` enum in `src/core/mcp_tools.rs`:
  ```rust
  pub enum SourceType { Conclusion, Review, Plan, Retrospect, Synthesis }
  ```
  Update the `Display` impl to emit `"synthesis"`.
- [ ] Update the parser-layer whitelist (if one exists) to accept
  `"synthesis"` — grep for callers that `match` or validate the string.
  Memory ingest via MCP will now accept `source_type: "synthesis"`; that's
  fine — it allows tests to insert synthesis rows without going through
  the dream command.
- [ ] Add `is_longterm: bool` field to `NewMemory` (`src/core/db.rs:49-59`) and
  thread it through `insert_memory`'s SQL INSERT column list + VALUES
  placeholder (line 101-131). Add `is_longterm = excluded.is_longterm`
  to the ON CONFLICT DO UPDATE SET clause. Update the ON CONFLICT
  transaction helper `insert_memory_resolving` (line 170) the same way.
  Default `is_longterm = false` at every existing callsite (grep
  `NewMemory {` — ingest.rs, tests). This closes the architect +
  dep-analyst MF-1 finding that the plan previously left the path
  ambiguous.
- [ ] Unit test (schema): fresh in-memory DB → `run_migrations` →
  `PRAGMA user_version = 4`. Second call is a no-op. `memory_synthesis_links`
  table exists (query `sqlite_master`).
- [ ] Unit test (schema): open a DB at schema v3 (manually set
  `user_version = 3`, insert a row), run migration → `user_version = 4`,
  existing row intact, new table exists.
- [ ] Unit test (db): `insert_memory` with `is_longterm = true` stores 1
  in the column; round-trip via `get_memory` returns `is_longterm: true`.
  With `is_longterm = false` (default), column is 0.
- [ ] Unit test (SourceType): `SourceType::Synthesis.to_string() == "synthesis"`.
- [ ] Unit test (SourceType): deserialize `{"source_type": "synthesis", ...}`
  via the existing IngestParams — must succeed.

Expected files: `src/core/schema.rs`, `src/core/mcp_tools.rs`, `src/core/db.rs`,
`src/core/ingest.rs` (callsite default)

### Step 2: Pure synthesis seam — prompt builder + response parser (AC2)

- [ ] Create `src/core/synthesis.rs` with two pure functions, no I/O, no DB,
  no LLM:
  ```rust
  pub struct SynthesisInput<'a> {
      pub cluster_memories: &'a [MemoryEntry], // ordered by memory_id
      pub cluster_centroid: &'a [f32],         // from Cluster, informational
      pub project_id: &'a str,
  }

  pub struct SynthesisDraft {
      pub title: String,
      pub content: String,
      pub entities: String, // comma-separated, from JSON array
      pub source_memory_ids: Vec<String>,
  }

  pub fn build_synthesis_prompt(input: &SynthesisInput) -> (String, String);
  //   → (system_prompt, user_prompt)

  pub fn parse_synthesis_response(raw: &str, source_ids: &[String])
      -> Result<SynthesisDraft, SynthesisError>;
  ```
  `SynthesisError` is a `thiserror::Error` enum covering:
  `NoJsonObject` (no `{...}` found), `InvalidJson(serde error)`,
  `MissingField { field: &'static str }`, `EmptyTitle`, `EmptyContent`.
- [ ] System prompt: short, strict JSON contract. Exact text in the plan
  (copy verbatim into code):
  > "You are consolidating related engineering memories. Output ONLY a
  > JSON object with keys title, content, entities. title ≤ 80 chars.
  > content 3–6 sentences, self-contained, cites the underlying decisions
  > without naming file paths. entities is an array of 2–6 compound tags
  > (lowercase, hyphen-separated, no spaces). No markdown, no prose
  > outside the JSON."
- [ ] User prompt: template `"Project: {project}\n\nMemories in this
  cluster ({n}):\n\n--- MEMORY 1 ---\nTitle: {t1}\nEntities: {e1}\n{c1}\n\n
  --- MEMORY 2 ---\n...\n\nWrite the synthesis JSON now."` with content
  truncation at **4000 chars per memory** (`content.chars().take(4000).collect()`),
  appending `"… [truncated]"` marker if truncated. Total prompt bound at
  ~20 memories × 4000 = 80K chars ≈ 20K tokens, safely under
  claude-sonnet context.
- [ ] Response parser: extract the first complete top-level `{...}` block
  via a **brace-depth counter** (O(n) single pass). Starting from the
  first `{`, increment depth on `{`, decrement on `}`, end at the
  matching depth-0 `}`. Correctly handles inner braces in content
  values (e.g., synthesis content like `"use Arc<Mutex<{}>>"`), which
  the naive `raw.find('{') + raw.rfind('}')` slice would over-capture.
  Cross-family review flagged this as a block-severity issue. Skip
  JSON-string-aware escape tracking — serde_json will reject if we
  miscount inside a string literal, and that failure is the correct
  fallback (counted as `InvalidJson`). Reason for brace-extraction at
  all: LLMs sometimes prepend or append commentary ("Here is the JSON:")
  even when told not to.

  Then `serde_json::from_str::<RawJson>` into a struct with
  `{ title: String, content: String, entities: Vec<String> }`. Join
  `entities` with `,`. Validate: non-empty title (≤ 200 chars, hard cap
  in case LLM ignores the 80-char soft cap), non-empty content, fill
  `source_memory_ids` from the passed-in `source_ids`.
- [ ] Unit tests (prompt builder):
  - Empty cluster (shouldn't happen — min_size ≥ 3 upstream) →
    `debug_assert`. Release-mode: still produces a valid prompt with
    `(0)` count.
  - 3 memories, titles/content as given → prompt contains each title,
    entity list is comma-separated in the prompt, truncation marker
    appears only when content > 4000 chars.
  - Content of exactly 4000 chars → no truncation marker.
  - Content of 4001 chars → truncation marker appended.
  - System prompt exact string matches a `const EXPECTED_SYSTEM_PROMPT`
    (regression guard).
- [ ] Unit tests (response parser):
  - Happy path: `{"title":"X","content":"Y.","entities":["a","b"]}` →
    `SynthesisDraft { title:"X", content:"Y.", entities:"a,b", source_memory_ids:[...] }`.
  - LLM preamble: `"Sure! Here:\n\n{\"title\":...}"` → parses cleanly.
  - LLM postamble: `{...}\n\nHope that helps!` → parses cleanly.
  - Missing title: `{"content":"Y","entities":[]}` → `MissingField{field:"title"}`.
  - Empty title: `{"title":"","content":"Y","entities":[]}` → `EmptyTitle`.
  - Empty content: `{"title":"X","content":"","entities":[]}` → `EmptyContent`.
  - Not JSON: `"I refuse."` → `NoJsonObject`.
  - Malformed JSON: `"{title: X}"` → `InvalidJson(_)`.
  - Array of entities empty → ok (rare but not fatal; caller handles
    downstream as plain content).
  - **Inner braces in content field** (regression guard for the
    brace-depth parser): `{"title":"X","content":"use Arc<Mutex<{}>>","entities":[]}`
    must parse cleanly. Naive `find('{')`/`rfind('}')` would fail here.
  - **Nested objects in entities** (adversarial): entities sometimes
    come back as `[{"tag":"x"}]` instead of `["x"]`. The parser should
    `InvalidJson` (strict schema), not silently flatten.
- [ ] Register: `pub mod synthesis;` in `src/core/mod.rs` (alphabetical).
- [ ] Verify: `cargo test --lib synthesis::` ≥ 10 tests pass. clippy +
  fmt clean. No `#[allow]`.

Expected files: `src/core/synthesis.rs`, `src/core/mod.rs`

### Step 3: `mengdie dream` synthesis orchestration + CLI flags + e2e test (AC3, AC4)

- [ ] In `src/core/dreaming.rs` (or new `src/core/dream_pipeline.rs` if
  `dreaming.rs` grows too large — judgment call during execution,
  prefer keeping it in `dreaming.rs` unless it exceeds ~400 lines):
  ```rust
  pub struct SynthesisResult {
      pub clusters_processed: usize,
      pub syntheses_created: usize,
      pub llm_errors: usize,
      pub residuals_skipped: usize,
  }

  pub async fn run_synthesis_pass(
      db: &Db,
      project_id: Option<&str>,
      provider: &dyn LlmProvider,
      threshold: f32,
      min_size: usize,
      max_cluster_size: usize,  // caps prompt size; review added
      dry_run: bool,
  ) -> anyhow::Result<SynthesisResult>;
  ```
  Note the return type: `LlmProvider::complete(system, prompt)` returns
  `LlmFuture<'a>` (= `Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send + 'a>>`,
  see `src/core/llm.rs:181`). `.await` on the call is correct — the
  trait is pinned-boxed-future, not a plain `Result`.

  Implementation: call `cluster_memories`, for each cluster truncate
  `memory_ids` to `max_cluster_size` (stable-sort by id then take first
  N to preserve determinism), load `MemoryEntry` rows via the new bulk
  helper (below), call the pure seam from Step 2, then either log
  (dry-run) or `provider.complete(...).await` + parse + atomic
  DB write.
  - **New Db helper (required, not optional)**: `fn get_memories_by_ids(&self, ids: &[String]) -> Result<Vec<MemoryEntry>>`
    — single-lock bulk fetch using `IN (?, ?, ...)`. Looping
    `get_memory` would take the mutex N times per cluster; the bulk
    helper is one acquisition.
  - **New Db helper (required for atomicity)**: `fn insert_synthesis_with_links(&self, mem: NewMemory, source_ids: &[String]) -> Result<String>`
    — one `conn.transaction()` that inserts the memory (via the
    existing INSERT logic factored into a helper or inlined) AND the
    N link rows via `INSERT OR IGNORE INTO memory_synthesis_links`.
    Commits atomically. Architect C-2 flagged this: `insert_memory`
    acquires+releases the lock; link inserts would be a second
    acquisition without atomicity.
  - **Error policy per cluster**: one LLM error (timeout, parse failure,
    etc.) increments `llm_errors` but does NOT abort the pass. Log at
    `warn` level with cluster memory_ids AND the `LlmError` variant
    name (so the operator can see if errors are systematic — 429s,
    Timeouts, parse failures).
  - **Dedup**: content_hash ON CONFLICT (existing mechanism) makes
    re-runs idempotent when the LLM produces the same content. Same
    cluster → same hash → same row updated in place. Link rows are
    `INSERT OR IGNORE` on the composite PK.
  - `is_longterm = false` for synthesis inserts — reversed from the
    plan's original default per cross-family review. Synthesis
    memories should earn long-term status through the normal dreaming
    promotion pass (recall + relevance), not by construction. Since
    synthesis rows are searchable immediately via FTS/vector (no
    `is_longterm` gate in search today), this change is a no-op for
    discovery but a meaningful signal of "not yet promoted."
- [ ] Extend `src/bin/cli.rs` `Commands::Dream` with new flags. Default
  **flipped** per cross-family review: `--synthesize` is explicit
  opt-in, not default-on. LLM-calling commands should not make
  network calls and DB writes on a bare `mengdie dream` invocation.
  ```rust
  /// Run LLM synthesis after promotion (opt-in: makes network calls and writes synthesis rows).
  #[arg(long)]
  synthesize: bool,

  /// Cluster threshold override. Default tracks
  /// clustering::DEFAULT_THRESHOLD (see
  /// docs/backlog/BL-clustering-validation.md for why 0.75).
  #[arg(long, default_value_t = mengdie::core::clustering::DEFAULT_THRESHOLD)]
  threshold: f32,

  /// Minimum cluster size for synthesis (default: 3).
  #[arg(long, default_value_t = mengdie::core::clustering::DEFAULT_MIN_SIZE)]
  min_cluster_size: usize,

  /// Maximum cluster size — caps prompt token budget by truncating
  /// oversized clusters (default: 20). Added per cross-family review
  /// to bound prompt size on CJK / code-heavy corpora where 4000
  /// chars ≠ 4000 tokens.
  #[arg(long, default_value_t = 20)]
  max_cluster_size: usize,

  /// Show what would be sent to the LLM without making calls
  /// (implies --synthesize; does not write rows or invoke the LLM).
  #[arg(long)]
  dry_run: bool,

  /// Project scope (default: all projects)
  #[arg(long)]
  project: Option<String>,
  ```
  No `--no-synthesize` needed now — the default is off.
- [ ] Convert `fn main()` in `src/bin/cli.rs:106` to
  `#[tokio::main(flavor = "current_thread")]`-annotated `async fn main()`.
  `cmd_dream` becomes `async fn`; the other subcommands stay sync and
  are called from the async `main` with no await. Reason (consolidated
  Must Fix from architect + dep-analyst): the old plan proposed nesting
  `Runtime::new()?.block_on(...)` inside sync `cmd_dream`, which works
  but is strictly worse than `#[tokio::main]` since the project already
  has tokio as a full dep. `current_thread` flavor keeps the single-
  threaded cost profile of the nested-runtime approach without the
  double-init risk.
  - Build provider via `build_provider(&cfg.llm)` (from BL-005) inside
    `cmd_dream` after the existing promotion pass.
  - If `dry_run` is set without `synthesize`, still run the synthesis
    path (dry_run implies synthesize — documented in the flag help).
- [ ] Output: print synthesis stats after promotion stats:
  `"Synthesis: N syntheses created from M clusters (K residuals skipped, E LLM errors)"`.
- [ ] Integration test (`tests/dream_synthesis.rs`, `#[ignore]`,
  same pattern as BL-005's `tests/llm_claude_cli.rs`): bootstrap an
  in-memory-file DB with 6 near-identical 384-dim embeddings in one
  project, run `run_synthesis_pass` with the real `ClaudeCliProvider`,
  assert exactly 1 synthesis row with `source_type="synthesis"` and 6
  link rows. Document at top: requires authenticated `claude` on PATH,
  opt-in via `cargo test -- --ignored dream_synthesis`.
- [ ] Unit test (no LLM): `run_synthesis_pass` with `dry_run=true` + a
  stub `LlmProvider` that panics if `complete` is called → asserts
  the stub is never called, no rows inserted, result counts match
  expected cluster count.
- [ ] Unit test (no LLM): `run_synthesis_pass` with a stub provider
  that returns a fixed JSON payload → asserts exactly N synthesis rows
  inserted (where N = cluster count from `cluster_memories`), link
  rows populated correctly.
- [ ] Unit test (error isolation): stub provider that returns `Err(Timeout)`
  for one cluster and valid JSON for another → one synthesis row, one
  LLM error counter incremented, residuals unchanged, pass completes
  without error propagation.
- [ ] Unit test (content_hash dedup): run synthesis pass twice against
  the same stub provider output → second run does not create a duplicate
  synthesis row (content_hash ON CONFLICT), does not create duplicate
  link rows (PK ON CONFLICT).
- [ ] Verify: `cargo test` (minus `#[ignore]`) clean. `cargo clippy
  --all-targets -- -D warnings` clean. `cargo fmt --all -- --check` clean.
  Manual: `cargo test -- --ignored dream_synthesis` run once with
  `claude` CLI authenticated; record outcome in the commit message.

Expected files: `src/core/dreaming.rs`, `src/bin/cli.rs`,
`tests/dream_synthesis.rs`

Parallel strategy:
- Step 1 touches `schema.rs`, `mcp_tools.rs`, `db.rs`, `ingest.rs`.
- Step 2 creates `synthesis.rs`, edits `mod.rs`.
- Zero file overlap between 1 and 2 — they MUST be landed in parallel
  during `/ae:work` to cut wall-clock time (architect recommendation,
  dep-analyst confirmed). Land each as a separate commit with a
  `TeamCreate` + two dev agents; QA runs once both are in.
- Step 3 depends on both Step 1 (new SourceType variant,
  `NewMemory.is_longterm` field) and Step 2 (`build_synthesis_prompt`,
  `parse_synthesis_response`, `SynthesisError`, `SynthesisDraft`). It
  runs sequentially after both merge.

## Acceptance Criteria

### AC1: Schema migration is safe and complete
- Fresh DB: `run_migrations` sets `user_version = 4`, creates
  `memory_synthesis_links` table with the documented columns and
  indexes.
- DB at v3 with existing data: `run_migrations` → `user_version = 4`,
  existing rows intact, new table exists.
- `SourceType::Synthesis.to_string() == "synthesis"`.
- MCP `IngestParams` accepts `source_type: "synthesis"` without error.
- `run_migrations` is idempotent — calling it twice on an already-v4 DB
  is a no-op.

### AC2: Prompt and parser are robust across realistic LLM outputs
- `build_synthesis_prompt` produces deterministic output for a given
  `SynthesisInput` (regression-tested against a `const` string for the
  system prompt and a snapshot-style assertion for the user prompt
  structure).
- Content per memory is truncated at exactly 4000 chars; truncation
  marker appended iff input > 4000 chars.
- `parse_synthesis_response` handles: clean JSON, LLM preamble, LLM
  postamble, missing fields, empty title/content, non-JSON output,
  malformed JSON — each maps to the correct `SynthesisError` variant.
- 100% of parse paths covered by ≥10 unit tests.

### AC3: `mengdie dream --synthesize` end-to-end behavior
- Dry-run (`--dry-run`): no LLM calls, no DB writes, prints cluster
  prompts, exit 0, `llm_errors == 0`.
- Stub-provider unit test: N clusters → N synthesis rows + matching
  link rows; residuals logged and skipped; `llm_errors == 0`.
- Stub-provider error-injection test: one cluster fails → one synthesis
  row, `llm_errors == 1`, other clusters still succeed.
- Re-run the same pass with the same stub-provider payload: zero
  net-new rows (content_hash dedup on synthesis row + link-table PK
  ON CONFLICT). Stub must return byte-identical JSON on both runs so
  content_hash collides. Real-LLM dedup is NOT part of this AC —
  the `#[ignore]` e2e test runs only once.
- `mengdie dream` without `--synthesize` makes no LLM calls and writes
  no synthesis rows (opt-in default).
- `--dry-run` (without `--synthesize`) implies synthesize; produces
  cluster-prompt output with no LLM calls and no writes.
- `--threshold 0.85` produces a different cluster count than
  `--threshold 0.75` on the same test fixture (validates the threshold
  knob is wired through and gives BL-clustering-validation its sweep
  mechanism).
- `--max-cluster-size 2` on a fixture of 5 near-identical embeddings
  with `min_cluster_size=3` produces 0 synthesis rows (cluster of 5
  is truncated to 2, which is below min). Validates the truncation knob.

### AC4: Opt-in integration against live `claude` CLI
- `cargo test -- --ignored dream_synthesis` with `claude` authenticated:
  creates exactly 1 synthesis row for a 6-memory tight cluster, with
  `source_type="synthesis"`, non-empty title, non-empty content,
  `is_longterm=false` (per the flipped default — syntheses earn
  long-term status via dreaming, not by construction), and 6 link
  rows. Run once manually; record PASS/FAIL + model name + first 40
  chars of the generated title in the Step 3 commit message.

### AC5: Empirical results paper trail for design-bet backlog
- After the first real `mengdie dream --synthesize` run on the
  production DB, append a `## BL-007 empirical results` section to
  `docs/backlog/BL-clustering-validation.md` with three fields:
  1. **Threshold bucket observed**: what `threshold` was used, how
     many clusters formed, whether residuals made up >50% of eligible
     memories.
  2. **Cluster quality judgment**: one of `good / mixed / poor`,
     with 1-sentence rationale from a manual scan of 3-5 synthesis
     titles+content vs. their source memories.
  3. **Trigger fires**: list which of BL-clustering-validation's three
     deferred triggers (seed-ordering / threshold / residuals) became
     actionable based on this data; file follow-up plan(s) if any did.

  Without this AC, BL-clustering-validation.md stays permanently open
  with no closure path (cross-family review #7). 5 minutes of data
  entry saves the validation-gate signal the backlog items need.

## Design Bet Validation (triggers from BL-clustering-validation.md)

This plan is the signal generator for the three deferred items in
`docs/backlog/BL-clustering-validation.md`. After Step 3 ships, we
collect data:

1. **Threshold 0.75 check**: run `mengdie dream --synthesize --dry-run`
   against the real mengdie DB. If all memories cluster into one
   massive group → threshold too loose, sweep `{0.80, 0.85, 0.90}` and
   amend the default. If zero clusters form → threshold too tight,
   sweep `{0.65, 0.70}`.
2. **Residuals policy**: log residual count per pass. If residuals
   consistently > 50% of eligible memories, this plan's "skip"
   policy is wasting signal; revisit with a "pairs" or "misc"
   sub-policy in a follow-up plan.
3. **Seed ordering quality**: review 3–5 produced syntheses manually.
   If clusters obviously split related decisions by lexicographic
   accident of memory_id, escalate to density-weighted seeding
   (the BL-clustering-validation remediation ladder).

These observations go into a retrospective note after the first real
dream run, not into the plan execution itself.

## Non-goals (explicit)

- **No power-law decay** — belongs in a sibling plan BL-008; decay
  tunes recall scoring, synthesis tunes knowledge compaction. They
  share the `dream` command but are independent features.
- **No MCP tool `memory_synthesize`** — today synthesis is invoked via
  CLI + cron. If ae:analyze ever wants on-demand synthesis, add the
  tool in a follow-up.
- **No cross-project synthesis** — per-project only (BL-006 scope).
- **No schema for residual policy data** — the MVP skips residuals; if
  BL-007 empirically shows we need richer residual info, evolve BL-006's
  `residuals: Vec<String>` then.
- **No second LLM provider** — ClaudeCliProvider is enough to validate
  the flow.
- **No auto-invalidation of source memories on synthesis** — originals
  stay valid. Synthesis is additive. A future plan may mark "absorbed"
  sources as superseded by the synthesis.
- **No retry policy on LLM errors** — one attempt per cluster per pass.
  BL-005's `ExitKind` classification is surfaced in logs; if we see
  systematic retryable errors (429, ECONNRESET), a follow-up plan adds
  backoff. **Recovery path**: re-running `mengdie dream --synthesize`
  is safe and idempotent (content_hash dedup on the synthesis row +
  link-table PK ON CONFLICT). Failed clusters get a fresh attempt on
  the next run. Cron-daily schedulers can leave this as-is.
