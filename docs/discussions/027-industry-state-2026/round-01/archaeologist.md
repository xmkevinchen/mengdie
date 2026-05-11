# Round 1 — archaeologist findings

## Topic 1 — Ingest mechanism (v0.x baseline)

### Findings (file:line evidence)

**Push path — fully operational, MCP-wired:**
- `src/core/mcp_tools.rs:278-370`: `memory_ingest` tool exists and dispatches
  to `ingest::ingest_text` or `ingest_text_with_resolves` depending on whether
  `resolves` is non-empty. The MCP path is live in production.
- `src/bin/mcp_server.rs:38`: server starts with `MengdieServer::new(db, embedder,
  project_id)` — no watcher daemon anywhere in this binary's startup sequence.
- `src/bin/cli.rs:607-628`: CLI `search` command also calls
  `memory_search_audited`; the CLI `import` path exists (walkdir-based bulk
  import via `ingest_file`).

**Pull path — library complete, daemon completely unwired:**
- `src/core/watcher.rs:20-63`: `start_watcher(dirs)` is fully implemented —
  debounce 500ms, filters to `is_ingestable` files, returns `(Receiver<FileEvent>,
  Debouncer)`.
- `src/core/watcher.rs:65-75`: `watch_loop<F>` callback loop is implemented.
- **Zero call sites outside tests**: `rg start_watcher` and `rg watch_loop`
  across the entire `src/` tree returns only hits inside `watcher.rs` itself
  (module declaration in `mod.rs` plus in-file tests). Neither `bin/mcp_server.rs`
  nor `bin/cli.rs` nor any other module imports or calls these functions.
- `src/core/mod.rs:19`: `pub mod watcher;` is re-exported from the library
  crate, so the API surface is public but has zero consumers outside the module.

**What `is_ingestable` accepts:**
- `src/core/parser.rs:159-180`: blocklist approach — any `.md` file passes
  UNLESS it matches `index.md`, `readme.md`, `round-*.md`, `summary.md`,
  `analysis.md`, `api.md`, or has `.swp`/`~` suffix. This is intentionally
  broad: `conclusion.md`, `review.md`, `plan.md`, `retrospect.md` all pass,
  but so does `notes.md`, `BL-007.md`, `topic-04-foo.md`, etc.

**Source type enum — `mcp_tools.rs:38-58`:**
```
Conclusion | Review | Plan | Retrospect | Synthesis
```
The `IngestParams` enforces these via serde `rename_all = "lowercase"`. No
`"analysis"` type is accepted by the MCP tool even though `schema.rs:16-23`
lists `"analysis"` in `ALLOWED_SOURCE_TYPES`.

**Cold-start (bulk import):**
- `src/bin/cli.rs:729-742`: `walkdir` helper walks a directory tree.
- CLI has a bulk import path that calls `ingest_file` per discovered file.
- Push-only means cold-start must use this CLI path explicitly; there is no
  "catch-up" scan that fires automatically.

### Agreements
(placeholder — to be filled after cross-agent round)

### Disagreements
(placeholder)

### Open Questions
- The watcher library is complete and tested but has never run in production.
  What was the original reason it wasn't wired? The git log shows it was built
  as "library ready, daemon integration deferred to Phase 2" (CLAUDE.md).
  No code comment explains the deferral rationale.
- `is_ingestable` is intentionally broad (blocklist, not allowlist). If the
  watcher were wired to `docs/` with a broad path, it would ingest `BL-*.md`,
  `topic-*.md`, and other structural files unless the path configuration is
  restricted. The allowlist vs blocklist tradeoff is not resolved in the code.

---

## Topic 2 — Reflection trigger model (v0.x baseline)

### Findings (file:line evidence)

**Cron trigger — the ONLY production trigger:**
- `resources/com.mengdie.dream.plist:8-19`: macOS launchd plist fires
  `mengdie dream` at 03:00 daily (`StartCalendarInterval Hour=3 Minute=0`).
  The plist is a template (`<!-- Update this path to your built binary -->`
  at line 9), not a system-installed unit. Whether it is actually loaded
  in launchd on the operator's machine is not verifiable from code.
- `src/core/dreaming.rs:72-74`: `Db::run_dreaming(project_id)` is the
  production entry point — no trigger logic inside; it calls
  `run_dreaming_with_config` with `DreamingConfig::default()` and
  `write_demotions=true`.

**On-demand trigger — also present, CLI-only:**
- `src/bin/cli.rs:207`: `Commands::Dream` dispatches dreaming via CLI.
- `mengdie dream --synthesize` is the command that produced the 13 syntheses
  (CLAUDE.md Project Status). This is an explicit operator invocation, not
  cron-driven. The first real run was manual on-demand.

**Salience / composite / debounced triggers — not present:**
- No entropy computation exists anywhere in `src/`.
- No conflict-density metric exists (contradiction detection fires per-ingest
  but produces no aggregate counter that feeds a trigger threshold).
- No in-process write-event queue or debounce executor exists.
- `src/core/metrics.rs` tracks `search_count`, `search_nonempty_count`,
  `ingest_count`, `conflict_count`, `audit_write_failures` — none of these
  is wired to trigger dreaming.

**Synthesis pass implementation — `src/core/dreaming.rs:399-578`:**
- `run_synthesis_pass(db, project_id, provider, threshold, min_size, max_cluster_size, dry_run)`
- Calls `cluster_memories` → builds prompts per cluster → LLM call per
  cluster via `LlmProvider::complete` → writes `NewMemory` with
  `source_type="synthesis"`, `is_longterm=false`.
- Syntheses earn long-term status through the dreaming promotion pass, not
  by construction (`is_longterm: false` at line 571).
- Content-hash dedup (`ON CONFLICT DO UPDATE`) makes re-runs idempotent.

**What "13 syntheses" actually produces:**
- Synthesis rows stored in `memory_entries` with `source_type="synthesis"`.
- Linked back to source memories via `memory_synthesis_links` table.
- Not queryable via MCP `memory_search` as a separate surface — they are
  ordinary `memory_entries` rows returned by the same hybrid FTS5+vector
  search. No dedicated MCP tool surfaces synthesis provenance.
- `is_longterm=false` at creation: these 13 rows are NOT in the promoted
  tier unless they subsequently met the dreaming promotion thresholds
  (recall_count >= 3, avg_relevance >= 0.45, last_recalled within 14 days).

**DreamingConfig defaults — `src/core/dreaming.rs:13-37`:**
```
DEFAULT_MIN_RECALL: 3
DEFAULT_MIN_RELEVANCE: 0.45
DEFAULT_WINDOW_DAYS: 14
```

### Agreements
(placeholder)

### Disagreements
(placeholder)

### Open Questions
- Is the launchd plist actually loaded on the operator's machine? Code cannot
  confirm. If not, cron is the documented-but-inactive trigger and on-demand
  is the only actual trigger used in production.
- Synthesis pass embeddings: `new_mem.embedding = None` at line 569 — synthesis
  rows are stored WITHOUT an embedding at creation time. This means they cannot
  be clustered or found via vector search until a re-embedding pass runs. No
  such re-embedding pass exists in the code. This is a material gap for Topic 2
  (if we want synthesis rows to participate in future clustering passes, they
  need embeddings).

---

## Topic 3 — Cross-project scope (v0.x baseline)

### Findings (file:line evidence)

**project_id resolution — `src/core/project.rs:10-21`:**
Resolution order:
1. `.mengdie.toml` in current dir or any ancestor up to git root → `project.name`
2. Git remote URL (FNV-1a hash of normalized URL) → `proj_<16-hex>`
3. Canonical path hash → `proj_<16-hex>`

The hash is deterministic: SSH and HTTPS for the same repo normalize to the
same string before hashing (`project.rs:103-121`).

**Storage — global, project-tagged:**
- `src/core/schema.rs:108`: `project_id TEXT NOT NULL` column on every
  `memory_entries` row. Global DB at `~/.mengdie/db.sqlite`.
- No separate per-project DB files; the per-project scoping is purely a
  query parameter.

**Search scope — `src/core/mcp_tools.rs:189-195`:**
```rust
let project_id = match params.scope.as_deref() {
    Some("global") => None,    // cross-project
    _ => Some(pid),            // per-project default
};
```
Per-project default is implemented as a simple `None` vs `Some(pid)` branch
in the search query. The `scope` parameter is optional in `SearchParams`
(`Option<String>`). Passing `scope: "global"` is the only way to cross-project
search today.

**Cost of changing the default:**
- Architectural cost: zero. The storage is already global; the per-project
  filter is a single conditional in `mcp_tools.rs:192-195`. Changing the
  default means changing `None` to `Some(pid)` vs. the reverse — a one-line
  diff.
- API cost: `scope` parameter already exists in `SearchParams`. AE plugin
  callers that want per-project would pass `scope: "project"` (or whatever
  the new keyword would be); callers that want global omit or pass `"global"`.
  No breaking change is required.

**project_id in mcp_server.rs — `src/bin/mcp_server.rs:32-34`:**
```rust
let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
let project_id = infer_project_id(&cwd);
```
The project_id is inferred once at server startup from the cwd where the MCP
server was launched. If Claude Code launches the server from project dir A,
all searches default to project A regardless of which directory the user later
opens. This is a documented single-startup assumption; not a bug, but a
constraint on the semantics of "current project."

**Clustering is project-scoped by default — `src/core/clustering.rs:71-79`:**
```rust
pub fn cluster_memories(db, project_id: Option<&str>, ...) -> ClusteringResult
```
`project_id: Some(pid)` limits clustering to one project. Synthesis pass
inherits this — syntheses are created with `project_id = memories[0].project_id`
(`dreaming.rs:459`). Cross-project synthesis is not possible today.

### Agreements
(placeholder)

### Disagreements
(placeholder)

### Open Questions
- The `project_id` at search time is the cwd at `mcp_server` startup, not the
  cwd at query time. For operators who open multiple projects in the same Claude
  Code session without restarting the MCP server, the default project_id is
  stale after the first project. This is not documented as a known limitation
  anywhere in the code or comments.

---

## Topic 4 — Ingest source boundary (v0.x baseline)

### Findings (file:line evidence)

**Sources v0.x DOES accept today:**

1. **AE pipeline files (file-ingest path):**
   - `src/core/parser.rs:16-65`: `parse_ae_file` reads any `.md` file,
     extracts YAML frontmatter, infers `source_type` from filename.
   - `src/core/parser.rs:136-154`: `infer_source_type` maps:
     - `*conclusion*` → `"conclusion"` → `knowledge_type="decisional"`
     - `*review*` → `"review"` → `knowledge_type="experiential"`
     - `*plan*` → `"plan"` → `knowledge_type="decisional"`
     - `*retrospect*` → `"retrospect"` → `knowledge_type="experiential"`
     - anything else → `"unknown"` → `knowledge_type="factual"`
   - CLI bulk import uses this path for any matching `.md` in a directory tree.

2. **Ad-hoc MCP ingest (push path):**
   - `src/core/mcp_tools.rs:79-99`: `IngestParams` accepts caller-specified
     `title`, `content`, `source_type` (enum), `knowledge_type` (enum), `entities`,
     `source_file` (optional), `project_id` (optional), `resolves` (optional).
   - The caller controls all fields. There is NO constraint that content comes
     from an AE pipeline file. Any agent or tool can call `memory_ingest` with
     arbitrary content.

3. **Synthesis (internal ingest):**
   - `src/core/dreaming.rs:561-573`: synthesis rows are written directly via
     `db.insert_synthesis_with_links(new_mem, &draft.source_memory_ids)` with
     `source_type="synthesis"`. This bypasses the MCP ingest tool entirely.

**Source provenance tracking in DB:**
- `source_file TEXT NOT NULL` (schema.rs:109): stores the originating file path
  for file-ingest, or the caller-supplied string for MCP-ingest, or a
  `synthesis/<uuid>.md` synthetic path for synthesis rows.
- `source_type TEXT NOT NULL` (schema.rs:110): one of the enum values
  (`conclusion`, `review`, `plan`, `retrospect`, `synthesis`, `analysis`).
- No `ingest_method` column distinguishing file-ingest from MCP-ingest from
  synthesis. Provenance is inferrable from `source_type=="synthesis"`, but
  file-ingest vs MCP-direct-ingest are indistinguishable in the DB.

**The "AE-only" commitment is a policy, not an enforcement:**
- The MCP `memory_ingest` tool accepts any text content from any caller. There
  is no gate that checks "did this come from an AE pipeline file?" The AE-only
  boundary in CLAUDE.md is an architectural intent, not a technical constraint.

### Agreements
(placeholder)

### Disagreements
(placeholder)

### Open Questions
- `infer_source_type` returns `"unknown"` for non-matching filenames and
  `knowledge_type="factual"`. Files named `notes.md` or `BL-007.md` would
  be ingested as `source_type="unknown"`, `knowledge_type="factual"`. The
  schema's `ALLOWED_SOURCE_TYPES` trigger (v5 migration) would reject
  `"unknown"` — this is a latent bug in the file-ingest path for non-AE files
  that `is_ingestable` passes through.

---

## Topic 5 — F-002 audit table (v0.x baseline)

### Findings (file:line evidence)

**Audit table schema — `src/core/schema.rs:277-302` (migration v6):**

```sql
CREATE TABLE memory_search_audit (
    id          INTEGER PRIMARY KEY,
    query       TEXT NOT NULL,
    scope       TEXT,            -- "global" | NULL (per-project)
    took_ms     INTEGER NOT NULL,
    searched_at TEXT NOT NULL    -- RFC3339 timestamp
);

CREATE TABLE audit_returned_facts (
    audit_id INTEGER NOT NULL,
    fact_id  TEXT NOT NULL,
    rank     INTEGER NOT NULL,   -- 0-indexed position in result list
    PRIMARY KEY (audit_id, fact_id),
    FOREIGN KEY (audit_id) REFERENCES memory_search_audit(id),
    FOREIGN KEY (fact_id)  REFERENCES memory_entries(id)
);
```

**What is recorded per search call:**
- `query` text, `scope` (global or per-project), `took_ms`, `searched_at`.
- Per-call: the ranked list of fact IDs returned (`audit_returned_facts`).
- What is NOT recorded: whether the caller used the results; any "useful /
  not useful" feedback; which result the agent cited in its output;
  whether the search was triggered by ae:analyze or by an operator's manual
  query.

**Write path — `src/core/db.rs:289-316`:**
- `record_search_audit` writes one audit row + N link rows in one transaction.
- `record_search_audit_best_effort` (db.rs:329-353) is the production wrapper;
  swallows errors with `tracing::warn!` and bumps `METRIC_AUDIT_WRITE_FAILURES`.

**Call sites — two:**
1. `src/core/search.rs`: `memory_search_audited` function (called from both
   MCP and CLI).
2. `src/bin/cli.rs:607-628`: CLI search explicitly passes `audit_start` timer.
3. `src/core/mcp_tools.rs:186-223`: MCP search passes `audit_start` timer.
   Both surfaces use the same `memory_search_audited` orchestrator function.

**What `memory_search_audited` does — `src/core/search.rs`:**
- Accepts a pre-call `Instant` for timing.
- Runs hybrid (FTS5+vector) or FTS-only fallback.
- Fires `record_search_audit_best_effort` exactly once post-filter.
- Returns `MemorySearchOutcome { results, route }`.

**Audit table queryable from MCP tool surface? NO.**
- `src/core/mcp_tools.rs`: three tools exposed — `memory_search`,
  `memory_ingest`, `memory_invalidate`. No `memory_audit_query` or
  `memory_stats` tool.
- The audit table is readable from CLI (`mengdie stats` at `cli.rs:675`) but
  `cmd_stats` only reads the `metrics` table counters (total, valid, longterm,
  recalled, search_count, ingest_count, conflict_count) — it does NOT query
  `memory_search_audit` or `audit_returned_facts` directly.
- There is no CLI command that dumps audit rows by date range, query text, or
  fact ID. Audit data is written but not surfaced via any existing user-facing
  interface.

**What is missing for loop-closure signal:**
- `audit_returned_facts.rank` is recorded but "no v0.0.1 query reads it"
  (schema.rs:284 comment).
- No "was this fact cited by the agent?" column. The audit records WHAT was
  returned, not WHAT was used.
- No `source` column distinguishing ae:analyze injections from operator manual
  queries — both produce identical audit rows. Cannot measure "injection rate
  from ae:analyze" vs "operator search rate" from the current schema.
- No contradiction event table. `METRIC_CONFLICT_COUNT` in `metrics` is a
  running total but not time-series data; cannot measure whether contradiction
  rate trends down over time.
- `metrics` table has `updated_at` but stores running totals, not per-event
  timestamps. Cannot compute per-day injection rates from it.

**`metrics` table columns (from `src/core/metrics.rs`):**
Tracks: `search_count`, `search_nonempty_count`, `ingest_count`,
`conflict_count`, `audit_write_failures`. All are lifetime totals, no daily
breakdown.

### Agreements
(placeholder)

### Disagreements
(placeholder)

### Open Questions
- The `rank` column in `audit_returned_facts` is described as "reserved for
  downstream consumers" with no v0.0.1 consumer. The data is there but nothing
  reads it.
- There is no mechanism for the operator to see what was injected in a given
  ae:analyze session — the Round 0 block shows it in-session but it is not
  persisted as a separate "injection event" record. The only durable trace is
  the `memory_search_audit` row + the associated `audit_returned_facts` rows.
  Matching these to "ae:analyze Round 0" requires knowing the query text and
  timestamp, which is not automated.
- Any future loop-closure signal that requires "was this fact cited?" would need
  a new ingest event from the AE plugin side — mengdie has no way to observe
  what Claude does with search results after they are returned.
