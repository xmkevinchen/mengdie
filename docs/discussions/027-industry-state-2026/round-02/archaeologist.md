# Round 2 — archaeologist cross-exam findings

> Context: five specific verification tasks assigned by team-lead.
> All findings are code-grounded with file:line evidence.
> Peer citations use `[agent name] file:line` format.

---

## Verification Task 1 — Synthesis `embedding=None` impact

### Findings

**Synthesis rows are stored with no embedding at creation:**

`src/core/dreaming.rs:569-570`:
```rust
embedding: None,
embedding_dim: None,
```

**No re-embedding pass exists anywhere in the codebase:**

- `rg "re.embed|reembed|embed.*synth|synth.*embed"` across all of `src/` — zero
  hits.
- `src/core/vector.rs` exports `store_embedding(db, id, embedding)`, but the
  only callers are in the test suite (`vector.rs` internal tests). No production
  caller re-embeds any row after creation.
- `src/core/dreaming.rs` does not call `store_embedding` anywhere.
- `src/bin/cli.rs` does not expose a `re-embed` or `embed-missing` subcommand.

**Downstream effects — clustering exclusion:**

`src/core/clustering.rs:71-79`: `cluster_memories` loads candidates via:
```sql
WHERE embedding IS NOT NULL AND embedding_dim = ?3
```
Synthesis rows (`embedding IS NULL`) are unconditionally excluded from every
clustering pass. This means synthesis rows can never be inputs to a future
synthesis pass — the synthesis pass cannot build higher-order syntheses on
top of lower-order ones.

**Downstream effects — vector search exclusion:**

`src/core/search.rs`: vector search path loads embeddings from the DB and
computes cosine similarity. Rows with `embedding IS NULL` have no vector to
compare against; they fall out of the vector-ranked result set. They can still
appear in FTS5 results, so they are not invisible — but their RRF rank will be
lower than equivalent non-null-embedding rows that score on both legs.

**Structural gap assessment:**

This is a **hard structural gap**, not a configuration issue:
1. No code path creates or stores an embedding for synthesis rows after creation.
2. The gap silently widens over time: every `mengdie dream --synthesize` run
   creates more embedding-null rows that fall out of future clustering passes.
3. Fixing it requires adding a `(re)embed_missing_rows` step either inside
   `run_synthesis_pass` (immediately after write) or as a separate CLI/cron
   pass. Neither exists today.

**Impact on Topic 2 decisions:**

Any reflection trigger design that builds on "density of synthesis rows"
(e.g., synthesis-count threshold for triggering the next pass) is undermined
because the previous synthesis output never re-enters the clustering pool.
Salience-threshold trigger variants that depend on vector similarity between
new ingests and existing syntheses are equally undermined.

**Impact on Topic 5 decisions:**

If the loop-closure signal includes "synthesis rows recalled per week" or
"synthesis-influenced search rate" (system-architect), the measurement will
under-count because synthesis rows are harder to surface via vector search.
Minimal-change-engineer's server-side audit approach (Topic 5) is unaffected
by this gap — the audit table records whatever the search engine returns, so
it correctly reflects the current search behavior, which *includes* the
synthesis under-representation.

### Agreements

- [system-architect] `round-01/system-architect.md` (no specific line cited for
  this gap — system-architect did not raise it): no position to agree with.
- [minimal-change-engineer] `round-01/minimal-change-engineer.md`: no position
  on embedding gap raised.
- [ai-engineer] `round-01/ai-engineer.md`: no position raised. The `ReflectionTrigger`
  trait proposal is technically compatible with fixing the gap separately.
- [challenger] `round-01/challenger.md`: did not mention this gap, but Goodhart's
  Law argument against synthesis-count metrics is indirectly supported by this
  finding — synthesis-count is doubly gameable if those rows don't even re-enter
  clustering.

### Disagreements

None directly — no peer agent addressed the synthesis embedding gap in Round 1.
The gap is a new archaeological finding. Indirectly, **system-architect's T5
proposal** ("synthesis-influencing-search rate") at `round-01/system-architect.md`
implicitly assumes synthesis rows are reachable via search — this finding shows
the reachability is partial (FTS5 only, not vector). The metric is still
computable but will systematically under-represent synthesis influence.

### Open Questions

- Should the fix land in `run_synthesis_pass` (embed immediately after write) or
  as a separate maintenance pass? Embedding inline would add ~2-10ms per synthesis
  row but keeps the data consistent from the start.
- Does `fastembed`'s blocking embed call in `spawn_blocking` already make the
  synthesis pass async-safe for inline embedding? (Yes — `ingest.rs` does it
  this way already. The synthesis pass would just need to replicate the
  `embedder.embed_one(content)?` call before building `NewMemory`.)

---

## Verification Task 2 — 028 "no ACK feedback" lock: exact language

### Findings

**Exact lock text — `docs/discussions/028-v0.0.1-architecture-design/conclusion.md:22-27`:**

```
MCP `memory_search` ACK feedback channel — NO in v0.0.1 contract.
challenger's argument (Round 2): the "used" signal is ambiguous (agent
received ≠ agent cited ≠ human approved). Implementing ACK creates an
implicit contract that mengdie will act on it, which requires a feedback
processor that doesn't exist. Deferred.

All Topic 4 triggers must be server-side observable from the persisted
domain audit table.
```

The lock is explicit and scoped: no ACK feedback on `memory_search` in
v0.0.1. The constraint is that all triggers must be derivable from the audit
table alone — no new signal channel from the caller.

**What this locks out:**

1. **gemini-proxy T5 (thumbs up/down per result)** —
   `round-01/gemini-proxy.md`: "a lightweight quality signal: thumbs up/down
   on every returned result." This is a per-result ACK channel. Directly
   violates the 028 lock.

2. **codex-proxy T5 (search-result-cited rate)** —
   `round-01/codex-proxy.md`: "search utilization rate = search-result-cited
   rate." The "cited" signal requires the caller to report back which results
   it used. This is an ACK channel. Directly violates the 028 lock.

3. **codex-proxy T5 (ACK via ae:analyze provenance marking)** —
   `round-01/codex-proxy.md`: proposes marking which memories influenced the
   ae:analyze output as a provenance signal. Even if implemented as AE plugin
   behavior rather than a new MCP tool parameter, this creates a feedback path
   from caller to mengdie. Violates the 028 lock in spirit even if not in the
   strict MCP tool signature.

**What the lock does NOT prohibit:**

- Server-side metrics derived entirely from existing audit rows (query
  frequency, nonempty-result rate, fact re-occurrence across audit rows).
- New CLI tools that read `memory_search_audit` / `audit_returned_facts`.
- Inference about whether a synthesis row "influenced" a search by comparing
  audit timestamps to synthesis creation timestamps (fully server-side).

### Agreements

- [minimal-change-engineer] `round-01/minimal-change-engineer.md`: "028
  conclusion locks out any per-result feedback — ACK field on the search tool
  would violate the no-ACK contract." Confirmed correct.
- [system-architect] `round-01/system-architect.md`: cites `028/conclusion.md:22-32`
  for the no-ACK lock. Citation is accurate; the exact text is at lines 22-27
  (slight line number drift but same passage).

### Disagreements

- [gemini-proxy] `round-01/gemini-proxy.md`: thumbs up/down proposal violates
  028 lock. gemini-proxy did not acknowledge this constraint in Round 1.
- [codex-proxy] `round-01/codex-proxy.md`: "search-result-cited rate" and
  "ae:analyze provenance marking" both violate the 028 lock. codex-proxy
  either missed the constraint or proposes overriding it — the round-01 text
  does not mention the 028 decision.

---

## Verification Task 3 — F-002 extension: does `cited_at` violate the no-ACK lock?

### Findings

**Schema extension in question:** Adding `cited_at TIMESTAMP NULL` to
`audit_returned_facts` (or a separate `cited_facts` table) where the AE plugin
calls back to mengdie to mark which returned facts it used.

**Technical cost:** Trivial. Schema migration v7: one `ALTER TABLE` or one new
table. The Rust side adds an optional `cited_at` field to `AuditReturnedFact`.
No architectural change.

**Lock violation assessment:**

Adding `cited_at` **would violate the 028 lock** because:

1. It creates a new write path from the caller (AE plugin) back to mengdie
   *after* the search response. This is precisely the ACK channel the 028 lock
   prohibits: "MCP `memory_search` ACK feedback channel — NO in v0.0.1 contract."
2. Even if implemented as a separate MCP tool (e.g., `memory_mark_cited`), the
   semantic is identical — the caller signals which results it used. The 028
   decision's stated rationale applies: "the 'used' signal is ambiguous (agent
   received ≠ agent cited ≠ human approved). Implementing ACK creates an
   implicit contract that mengdie will act on it."
3. The 028 lock's constraint is: "All Topic 4 triggers must be server-side
   observable from the persisted domain audit table." A `cited_at` column
   populated by caller callback is *not* server-side observable — it depends
   on caller cooperation.

**What CAN be added without violating the lock:**

- Server-inferred re-occurrence: if fact F appears in audit rows for queries Q1
  (Monday) and Q3 (Wednesday), that co-occurrence is a server-side signal
  computable entirely from existing `audit_returned_facts` rows. No new ACK.
- Per-synthesis retrieval rate: compare synthesis creation timestamps to
  subsequent audit rows that returned the synthesis row. Server-side observable.
- Query-fact co-occurrence graph: build a `(fact_id, fact_id)` adjacency from
  shared audit appearances. Zero new schema columns required.

### Agreements

- [minimal-change-engineer] `round-01/minimal-change-engineer.md`: "BL-014
  audit-stats CLI reads the existing table — no new signal channels." Consistent
  with this finding: reading existing data is fine; the constraint is on adding
  caller-written feedback columns.

### Disagreements

- [codex-proxy] `round-01/codex-proxy.md`: implied that "cited rate" is the
  right T5 metric. Adding the infrastructure for that metric requires an ACK
  channel, which violates the 028 lock. The metric is not achievable within
  the v0.0.1 constraint without either overriding the 028 decision or accepting
  a proxy metric.

---

## Verification Task 4 — `mengdie import` cold-start path at `cli.rs:361`

### Findings

**`cmd_import` function — `src/bin/cli.rs:361-424`:**

```rust
async fn cmd_import(db: &Db, dir: &PathBuf, dry_run: bool) -> anyhow::Result<()>
```

Implementation:
1. Calls `walkdir(dir)` helper (`cli.rs:729-742`) — recursive directory walk.
2. Filters with `is_ingestable(path)` (`parser.rs:159-180`) — passes `.md`
   files not on the blocklist.
3. Per accepted file: calls `ingest_file(db, embedder, path, project_id)`.
4. Dedup: `is_unique_violation` detects SQLite UNIQUE constraint errors and
   skips already-ingested files without error.
5. Dry-run mode: prints files that *would* be ingested, makes no DB writes.

**Does it work for cold-start bulk import without AE plugin re-runs?** Yes:

- `ingest_file` reads the `.md` from disk, calls `parse_ae_file` (YAML
  frontmatter extraction + source type inference), then `ingest_document`
  → `ingest_text`. This path reads and processes the file content directly
  from the filesystem. It does not require the AE plugin to be running.
- `infer_project_id` in `cmd_import` is called from the directory passed as
  `--dir`, not from the MCP server's startup cwd. Cold-start import from
  `docs/discussions/` would tag all ingested rows with the git-remote-derived
  project_id for that repo.
- The 13 syntheses (CLAUDE.md) were produced from memories that were bulk-
  imported or MCP-ingested before the synthesis pass ran. This confirms the
  path is exercised in real operator use.

**Caveat — `infer_source_type` unknown bug still present:**

As noted in Round 1: files that do not match `*conclusion*`, `*review*`,
`*plan*`, `*retrospect*` will produce `source_type="unknown"`. The v5 schema
trigger rejects `"unknown"`. This means `mengdie import docs/backlog/` would
fail silently (unique-violation path) or hard-error for `BL-*.md` files.
Cold-start with `docs/discussions/` is safe only if the AE pipeline filename
conventions are followed.

### Agreements

- [system-architect] `round-01/system-architect.md`: cites `cli.rs:361` for
  cold-start bulk-import. Confirmed: the path is real, functional, and does
  not require AE plugin re-runs. Citation is accurate.

### Disagreements

None.

---

## Verification Task 5 — project_id staleness on cwd-switch: effect on Topic 3 ratify decision

### Findings

**The staleness mechanism:**

`src/bin/mcp_server.rs:32-34`:
```rust
let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
let project_id = infer_project_id(&cwd);
```
`project_id` is captured once at server startup. If Claude Code launches the
MCP server from project dir A, all subsequent `memory_search` calls default
to project A's project_id even if the user switches to project B in the same
Claude Code session.

**Is this a concern for Topic 3's ratify decision?** It depends on which option
is chosen:

**If Topic 3 ratifies per-project default (status quo):**
Staleness is an active defect for multi-project workflows. Operator opens project
A → searches correctly → opens project B in same session → searches incorrectly
(project A's filter). No warning is emitted. This warrants a backlog item.
The fix requires either (a) restarting the MCP server per project, or (b) passing
`project_id` in `SearchParams` instead of relying on startup inference.

**If Topic 3 ratifies cross-project default (challenger's proposal):**
Staleness becomes irrelevant. With `project_id = None` as the search default,
the startup-captured `project_id` is only used to tag newly-ingested memories
(which is correct — memories ingested while in project B should be tagged B,
not A). Search staleness disappears as an issue.

**Conclusion for Topic 3 ratify:**

Staleness is a secondary argument that *slightly favors* cross-project default,
but it is not a decisive factor:
- The status quo (per-project) has the staleness defect but it's fixable by
  adding `project_id` as an explicit `SearchParams` field.
- Cross-project default avoids the problem but trades it for "noise from unrelated
  projects" for operators with many repos.
- Challenger's core argument (`round-01/challenger.md`) stands on its own
  (single-operator ergonomics, low cross-project noise for a personal tool)
  without relying on the staleness issue.
- Staleness does not change the binary choice; it is a tiebreaker at most.

**challenger cites staleness as supporting cross-project** —
`round-01/challenger.md` touches on this in its Topic 3 section. The
archaeologist finding confirms the staleness is real and the fix cost is low
either way (a one-line default change either way at `mcp_tools.rs:192-195`).

### Agreements

- [challenger] `round-01/challenger.md`: cross-project-as-default is compatible
  with the storage model (global DB, project_id is just a filter). Confirmed:
  changing the default is a one-line diff at `mcp_tools.rs:192-195`.
- [minimal-change-engineer] `round-01/minimal-change-engineer.md`: per-project
  default maintains status quo. Confirmed: per-project IS the current code
  behavior; changing it is one line.

### Disagreements

None on facts. The disagreement between challenger and minimal-change-engineer
on which default is *correct* is a design debate, not a factual dispute.
The staleness finding is a new fact neither agent raised in Round 1.

---

## Cross-topic open questions (archaeological)

1. **Synthesis re-embedding**: the gap (Task 1) is independent of Topic 2 trigger
   design but affects any metric that counts synthesis rows retrieved. Should
   be filed as a BL before finalizing T5 metric choice.

2. **028 lock scope**: the lock text says "v0.0.1 contract." Does this mean
   ACK is acceptable for v0.0.2+? If so, codex-proxy and gemini-proxy proposals
   are deferred rather than rejected. The lock text does not say "permanent" —
   only "deferred."

3. **`ingest_method` provenance gap**: file-ingest vs MCP-direct-ingest are
   indistinguishable in the DB. For T5 measurement, if the AE plugin calls
   `memory_ingest` directly (push path), the audit table records the search but
   there is no column distinguishing "ae:analyze triggered" vs "operator manual."
   Any loop-closure signal that needs to measure "AE-driven injection rate"
   cannot be derived from current schema without adding an `ingest_source`
   column — which would be a schema extension, not an ACK channel, and would
   NOT violate the 028 lock.
