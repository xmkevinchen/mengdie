---
discussion: "027"
round: 1
agent: system-architect
lens: "system design — module boundaries, contract design, reversibility"
created: 2026-05-05
---

# Round 1 — system-architect findings

## Topic 1 — Ingest mechanism (open, gated by Topic 4 ratification)

### Findings (with file:line evidence)

**The two surfaces already exist asymmetrically.** Push has a wired
production path; pull has a library with no daemon shell.

- Push surface — `src/core/mcp_tools.rs:282-371` exposes
  `memory_ingest` with full plumbing: validation, project_id resolution,
  `spawn_blocking` for the embed call, routing to
  `ingest::ingest_text` vs `ingest_text_with_resolves`, conflict report
  in the response. The `resolves` parameter (`mcp_tools.rs:96-98`,
  `ingest.rs:162-176`) is push-only — it has no pull-mode analogue
  because it requires the caller to know predecessor IDs, which a
  filesystem watcher does not. This is a contract feature, not a
  detail.
- Pull surface — `src/core/watcher.rs:20-63` returns a
  `notify_debouncer_mini::Debouncer` plus an `mpsc::Receiver<FileEvent>`.
  No daemon binary, no supervisor, no restart-on-crash. The
  `watch_loop` helper (`watcher.rs:68-75`) is a synchronous `for
  event in rx` consumer; nothing in the tree drives it in production.
  The `tests/common`-style harness drives it for unit tests only.
- Cold-start replay path already exists for push — `cmd_import`
  (`src/bin/cli.rs:361-420`) walks a directory and calls
  `ingest_file` per file. Same project_id inference
  (`cli.rs:364`), same dedup behavior (`cli.rs:408-414` handles
  `is_unique_violation`). This means **push-only does NOT need a new
  bulk-import path** — it has one, in production, exercised by the CLI.

**Mechanism shapes the contract surface.**

- Push (synchronous via MCP tool call): caller sees errors directly,
  sees the `IngestOutput.error` field (`mcp_tools.rs:144-145`), sees
  conflicts in the response (`mcp_tools.rs:142,343-355`). The AE
  skill that emitted the artifact owns the failure handling. Atomic
  resolves (`resolves: Option<Vec<String>>`) can only ride this
  surface because the caller knows predecessor IDs.
- Pull (asynchronous via filesystem): writer never blocks, never sees
  errors. The watcher daemon must own a tracing surface AND a
  silent-stop detector (per Topic 1 framing's key question). neither
  exists in `watcher.rs` today; building them is non-trivial because
  "watcher silently stopped" requires either a heartbeat or a periodic
  reconciliation pass over `docs/`.
- Hybrid (both active): doubles the contract surface and the
  failure-mode set. A push-then-pull race window exists (writer
  triggers a push, watcher independently sees the file event, both
  attempt to ingest the same artifact). The current schema's
  content-hash dedup (referenced in the F-003 Step 5 docstrings at
  `ingest.rs:139-146`) makes this idempotent at the storage layer,
  but the operator now has two log surfaces to reconcile when
  debugging a missing fact.
- Event-driven (queue/bus): adds a third process. Out of scope for
  v0.0.1 — single-binary stdio MCP server constraint per blueprint
  §1, §6 and CLAUDE.md "Architecture" section.

**v0.0.1 architecture decisions in 028 reinforce push.** The 028
conclusion (`docs/discussions/028-v0.0.1-architecture-design/conclusion.md:18`)
fixes the F-002 Wave 1 audit table as the substrate for the A-MEM
trigger. Audit is only meaningful if every search/ingest is observable
via the audit table; pull-mode ingest, run by a separate watcher
process, complicates this — the audit row is then attributed to a
daemon, not to a caller skill, and the supersession join becomes a
multi-process operation. The 028 audit-table-link-table choice
implicitly assumes a synchronous caller boundary.

**Coupling with Topic 4.** Push is structurally tied to the AE plugin
being the only producer (Topic 4 ratify outcome). If the source set
broadens later — e.g., commit messages, chat summaries — the new
sources also call `memory_ingest` directly; no mechanism rework
needed. Push generalizes; pull would not (every new source produces
files in different conventions, and the watcher would need
per-source schema dispatch — exactly the typed-source-marker
forward-compat problem Topic 4 raises).

**Failure-mode visibility — push wins by a wide margin.**

| Failure | Push surfaces it | Pull surfaces it |
|---|---|---|
| Embed fails on ingest | Caller sees `error: "ingestion failed"` (`mcp_tools.rs:362-369`); ae:work or ae:analyze sees the MCP tool error reply | tracing line on stderr only; no upstream signal |
| Conflict detected | `IngestOutput.conflicts` (`mcp_tools.rs:142, 343-355`) — caller can react | tracing line; nothing acts on it |
| Daemon stopped | N/A (no daemon) | Silent until operator notices stale data; needs heartbeat or recon |
| Disk full / SQLITE_FULL | Caller error reply | tracing only |
| Schema migration mid-ingest | TX fails, caller error | TX fails, daemon log |

**Cold-start replay semantics.**

- Push + existing CLI bulk-import is replay-shape-equivalent to pull:
  walk the dir, ingest each file, dedup via content_hash. The
  difference is who runs the walk — operator-driven via
  `mengdie import` vs daemon-driven on first start.
- Pull's "naturally replays on startup" property (per Topic 1
  framing's open question) is correct only with one deficiency: it
  replays files that exist on disk at startup but does not handle the
  population gap between "AE produced and committed an artifact" and
  "watcher started after commit landed." The watcher would need a
  startup reconciliation pass over the watched dir(s) anyway — i.e.,
  a bulk import pass at boot — making "naturally replays" actually
  "implements bulk import on startup," which push already has.

### Preliminary directional verdict

**Push as v0.0.1 default. Watcher library kept as opt-in (no daemon
shipped).** Reversibility is high — the watcher library remains in
`src/core/watcher.rs`; if a daemon is later wanted, ship one binary
that wraps `start_watcher` + `watch_loop` + a call to the same
`ingest_file` path. No data migration, no contract break.

**Failure-mode argument:** push pushes errors back to the caller AE
skill which is already the responsible owner — the skill that emitted
the artifact knows what to do with a conflict or an embed-fail.
Pull-mode failures land in `tracing::warn!` lines in a daemon log
that nobody reads.

**Atomic-resolves argument:** the `resolves` parameter
(`mcp_tools.rs:96-98`) is push-only by construction. Reversing this
to pull would require a watcher that parses each artifact for "this
supersedes IDs X, Y" — adding parser surface that the producer (AE
plugin) is far better positioned to compute.

**One concrete additional design point:** the AE skill that calls
`memory_ingest` should be specified per skill, not left as
"whichever skill remembers to." Conclusion → `/ae:work` final
commit; review → `/ae:review` final verdict; plan → `/ae:plan` final
commit; retrospect → `/ae:retrospect` summary. This belongs in BL C
of the 028 Next Steps wave 1 (AE plugin Round-0 wiring) or as a
companion BL.

### Reversibility

| Direction | Cost | Trigger |
|---|---|---|
| Push → +pull (daemon shipped) | Medium — wrap existing watcher lib in a binary, write supervision (launchd plist), wire to `ingest_file` | AE plugin can't be patched in environments mengdie targets (e.g., post-v0.0.1 generic AI tool support per CLAUDE.md "post-v1 generic" thesis) |
| Push → event-driven | High — third process surface | Multi-tool-multi-mengdie scenario where one mengdie serves N AI tools concurrently — explicitly post-v0.0.1 territory |
| Push → pure pull | High — drop `resolves` atomicity, lose caller-side failure surface | Not architecturally defensible; would re-introduce the v0.x "library exists, daemon never wired" gap |

### Agreements (placeholder for Round 2)

(filled in Round 2 after seeing other agents' findings)

### Disagreements (placeholder for Round 2)

(filled in Round 2)

### Open Questions

- Should the AE plugin's per-skill push-call be specified in this
  discussion, or punted to a follow-up `/ae:plan` for the AE plugin
  side? Architecturally it belongs to the AE plugin repo, but the
  contract (which skill calls when) belongs in the mengdie blueprint.
- Does v0.0.1 commit to deleting `watcher.rs`, or keep it as a
  documented opt-in library (i.e., "if you want pull mode, here's
  the building block — ship your own daemon")? Argument for
  keeping: zero maintenance cost, optional surface. Argument for
  deletion: dead code attracts re-adoption pressure later when the
  thesis hasn't changed.

---

## Topic 2 — Reflection trigger model (open with v0.x baseline)

### Findings (with file:line evidence)

**Cron is shipped, has produced empirical output, and is the only
trigger today.** `dreaming.rs:67-313` implements `run_dreaming` and
`run_dreaming_with_config`, both invoked via the CLI subcommand
`mengdie dream` (driven by macOS launchd plist per
`resources/com.mengdie.dream.plist` referenced in CLAUDE.md). First
real `--synthesize` run produced 13 syntheses against production DB
(per CLAUDE.md "Project Status").

**The `dreaming.rs` API is clean about coupling.**

- `run_dreaming_with_config` takes `now: Option<DateTime<Utc>>` and
  `write_demotions: bool` (`dreaming.rs:85-91`) — already
  trigger-agnostic. The function is a pure pass over the DB; **how
  often it fires is not its concern.**
- Synthesis is in the same module and shares the API shape
  (`dreaming.rs:399-579` `run_synthesis_pass`); it also takes no
  trigger argument.
- The trigger is therefore an external orchestration concern, not a
  module-internal one. This is structurally clean: the trigger model
  decision does NOT modify `dreaming.rs` for cron or on-demand. It
  DOES modify `dreaming.rs` for salience / composite / debounced
  because those need new instrumentation reads.

**Per-candidate module/interface impact:**

| Trigger | New module | New interfaces | Coupling to dreaming.rs |
|---|---|---|---|
| **cron** (current) | none | none | none — already lives in CLI + plist |
| **on-demand** | none | one new MCP tool `dream_run` (or extend `mengdie dream` exposure) | none — already lives in CLI |
| **salience** | new — salience scorer per memory or per cluster, recomputed on each ingest | per-memory `salience: f64` column (schema migration) + a salience-update hook in `ingest::ingest_text` | medium — `dreaming.rs` reads salience, but the hard work is upstream in ingest |
| **composite** (SCM) | new — entropy + conflict-density computer, schedule poll | entropy: per-cluster Shannon entropy over recent embeddings (requires a "recent ingest events" stream); conflict density: per entity-tag, count of contradiction-flagged items / window | high — `dreaming.rs` needs to consult the metric store before each pass; metric store does not exist |
| **debounced** (LangMem) | new — in-process ReflectionExecutor with submit-dedupe queue | per-process executor task that wakes on ingest event; `tokio::task` lifecycle; coalesce-by-key window | medium — fires `run_synthesis_pass` from inside the MCP server process |

**Constraint check against the framing's "tractable for v0.0.1?"
question:**

- Salience: requires defining what "important" means for an AE
  artifact (no like/star/highlight signal per topic 2 framing key
  question 5). Two options surface in literature: (a) recall-count
  driven (already computed — `recall_count` column,
  `db.rs` `record_recall`), (b) entity-tag rarity driven (rare tags
  surface). Both are tractable, BUT they bias toward already-popular
  facts — a reflection pass driven by "what's been recalled often"
  re-summarizes things the operator already finds; the actual unmet
  need is "synthesize across a cluster the operator hasn't yet
  asked about" (§2 Core Promise). Salience is structurally
  misaligned with mengdie's loop. **Verdict: tractable but
  semantically wrong direction for v0.0.1.**
- Composite (SCM): entropy over what set? The cluster centroid
  embeddings change with each new ingest; recomputing entropy per
  pass is feasible but the threshold (>0.9) is paper-specific and
  has no calibration on AE corpus. Conflict density needs the
  contradiction stream (already exists — `contradiction.rs`) but
  requires a sliding-window aggregate that doesn't exist. **Verdict:
  not tractable for v0.0.1 without measurable calibration; the SCM
  paper's numbers are not transferable without a benchmark.**
- Debounced (LangMem): tractable in-process. `tokio::sync::mpsc` +
  a debouncer task. BUT — synthesis takes minutes (LLM calls per
  cluster); coalescing within a 30-second window means the executor
  is essentially a batched cron. The "every write triggers
  reflection" semantics requires a ReflectionExecutor lifecycle
  that holds across MCP server restarts (otherwise events queued
  before crash are lost). **Verdict: tractable in code, but adds an
  in-process background-task contract that the stdio MCP server
  pattern does not have today.** New surface: how does the executor
  log? How is it observed for "fired N times last hour"?
- Cron: tractable today, no new code. Operator-managed via launchd.
- On-demand: tractable today, exists in CLI. Add an MCP tool wrapper
  for symmetry with cron-from-CLI.

**The architecturally cleanest v0.0.1 trigger model.** Cron + on-demand,
both running today, with on-demand exposed as an MCP tool so AE
skills (e.g., `/ae:retrospect`) can invoke synthesis after producing
a batch of related artifacts. This costs:
- Zero new modules
- One small `dream_run` MCP tool (≈30 lines mirroring
  `mcp_tools::ingest`'s shape, calling `run_synthesis_pass` via
  `spawn_blocking`)
- Zero schema changes
- Zero new instrumentation contracts

The other three (salience / composite / debounced) all require new
instrumentation that mengdie does not yet have AND whose
calibration constants are not transferable from the literature.
Filing them as follow-up BLs with the trigger condition "if the
loop-closure measurement (Topic 5) shows cron-only is missing
N% of the synthesis opportunities" is the architecturally
defensible path.

### Preliminary directional verdict

**v0.0.1 default: cron + on-demand (both available; cron is
launchd-driven, on-demand exposed as `dream_run` MCP tool plus
existing `mengdie dream` CLI). File salience / composite / debounced
as deferred BLs with explicit triggers.** Reversibility is high —
adding a new trigger model is purely additive (new module + entry
point); none of the deferred candidates require schema migration if
filed later.

**Coupling argument:** cron + on-demand both live OUTSIDE
`dreaming.rs`. Adding salience or composite would push trigger
logic INTO `dreaming.rs` (or a new sibling module that
`dreaming.rs` consults). The 028 conclusion already chose to
defer "Reflection module consolidation" pending the sqlite-vec
spike (Topic 3 there). Adding a metric-bearing trigger now risks a
double-refactor: first add the metric scaffolding, then collapse it
when the consolidation BL fires. Cron + on-demand sidesteps this.

**Operator-visibility argument:** cron + on-demand are both
operator-controllable from the operator seat. The framing's key
question 2 ("which trigger models are observable?") favors them
strongly: the operator runs `mengdie dream` or the launchd job
fires; both produce stdout output the operator already reads. A
debounced executor running inside the MCP server is observable only
through `tracing::info!` lines on stderr that get rolled into Claude
Code's MCP log, which the operator does not naturally check.

### Reversibility

| Direction | Cost | Trigger |
|---|---|---|
| cron+on-demand → +debounced | Medium — new in-process executor; lifecycle work | Topic 5 metric shows cron lag impacts loop closure (e.g., "N% of facts ingested today were not synthesized within 24h, despite cluster-eligibility") |
| cron+on-demand → +salience | High — schema migration + ingest hook | Operator demand for "synthesize when this cluster grows hot" — not currently expressed |
| cron+on-demand → composite | High — new metric infra | SCM benchmark against AE corpus shows defensible threshold values |

### Open Questions

- Should the on-demand MCP tool (`dream_run`) be in v0.0.1 sprint
  scope, or filed as a follow-up? Argument for v0.0.1: closes the
  "trigger from skill" gap that cron alone leaves open. Argument
  against: AE skills can already shell out to `mengdie dream` via
  Bash, so MCP tool is convenience not capability.
- The `--synthesize` pass is an LLM-cost operation; on-demand-from-
  skill could create cost overruns if a skill author calls it on
  every invocation. Worth including a per-day rate limit on the
  tool? (Architecturally, this is an MCP-tool-side concern.)

---

## Topic 3 — Cross-project default retrieval scope (ratify-or-defer)

### Findings (with file:line evidence)

**§5 commitment is structurally low-cost to revisit.**

- Storage is global (`~/.mengdie/db.sqlite` per CLAUDE.md
  "Architecture"; `project_id` is a column, not a database boundary).
  The `MemoryEntry` carries `project_id` as a field; queries filter
  via `WHERE project_id = ?`. Schema-side, cross-project search is
  one parameter change in the query, not a migration.
- The current MCP `SearchParams` (`mcp_tools.rs:23-35`) already
  exposes `scope: Option<String>` with `"global"` semantics
  (`mcp_tools.rs:192-195`). Cross-project opt-in works in production
  today.
- The ingest path normalizes case but doesn't normalize project_id
  (`ingest.rs:51-108`); each ingest carries the project_id of the
  ingesting cwd. This is correct and remains correct under any
  Topic 3 outcome.

**Multi-project storage shape is invariant under any acceptable
outcome.** The decision is policy (which scope is default), not
schema. Per the framing's key question 5: "What's the simplest
implementation that lets the operator change this answer cheaply if
they're wrong?" — the answer already exists: a single line in
`mcp_tools.rs:192-195` toggles the default. Operator can override
per call via the existing `scope: "global"` parameter.

**Architectural reason to revisit?**

- **No architectural reason.** §5's commitment is policy on a query
  parameter; schema is unchanged either way.
- The actual reason to revisit would be empirical: F-002 audit data
  showing `scope: "global"` is the operator's frequent override.
  That's a Topic 5 question (loop measurement), not a Topic 3
  question.
- The framing's sub-question 1 ("policy decision or per-call
  config?") is already answered structurally: it's per-call,
  defaulted to per-project. There is no global config knob today
  and shouldn't be one — config drift between projects is exactly
  the cross-contamination failure mode CLAUDE.md §5 names.

**Per-source-type cross-project default would be a real architectural
question, BUT it is post-v0.0.1.** A pattern like "factual / Rust
idiom memories default to global; decisional memories default to
per-project" requires:

- A `default_scope` field per source_type or knowledge_type (schema
  change)
- A second-pass on every existing memory to populate the default
- A query path that selects scope per result
- An operator-discoverable explanation for why some results came
  from other projects and others didn't

None of this is justified pre-evidence. F-002 audit data must drive
this — without query-volume distribution by scope, "factual should
be global" is opinion.

### Preliminary directional verdict

**Ratify §5 unchanged for v0.0.1.** Add a deferred BL with the
trigger condition: "revisit when F-002 audit data shows
`scope: 'global'` opt-in rate ≥ 30% across a 60-day window OR ≥3
operator-reported instances of 'I had to remember to set scope
global to find this' in retrospect." Reversibility is high (one-line
default change) so the bar to revise is correspondingly low — but
needs evidence, not preference.

**Architectural rationale:** §5 is a policy on a single query
parameter; storage is global so the policy can be flipped per call
or per default with no migration. There is no module-boundary or
contract-design implication that demands resolution before P1/P2
work.

### Reversibility

| Direction | Cost | Trigger |
|---|---|---|
| Ratify → revise default to global | Trivial — `mcp_tools.rs:192-195` | F-002 audit ≥30% global-scope queries |
| Ratify → per-source-type default | Medium — schema enum + per-result attribution | Mixed-mode usage proves segmentable |
| Ratify → per-call config gates | Low — extend `SearchParams` | AE plugin needs per-skill scope discipline |

### Open Questions

- Does Topic 5 (loop measurement) explicitly cover "fraction of
  searches that used `scope: 'global'`"? If yes, Topic 3 has a
  natural feedback path. If no, the deferred BL trigger condition
  must specify that this metric be added — otherwise the trigger
  is unobservable.

---

## Topic 4 — Ingest source boundary (ratify AE-only)

### Findings (with file:line evidence)

**Storage and search are source-agnostic; the boundary is
conceptual.** This is correctly identified in the topic's framing
constraints.

- `MemoryEntry` (per `db.rs` shape, e.g., `dreaming.rs:758-786`)
  carries `source_type: String`. Source enum (`mcp_tools.rs:38-58`)
  enumerates `Conclusion | Review | Plan | Retrospect | Synthesis`.
  Synthesis is mengdie-internal output; the rest are AE-pipeline
  artifacts.
- The ingest API (`ingest.rs::IngestMetadata` and
  `mcp_tools.rs::IngestParams`) takes `source_type` as an enum
  bound to those five strings. Adding a sixth source (e.g.,
  `CommitMessage`) is an enum variant addition + display impl + a
  parser if file-path inference is wanted.
- The contradiction-detection logic (`contradiction.rs`) keys off
  `entity` overlap, not source type. Adding new sources does not
  break it; it just feeds in less-curated entity sets.

**Architectural implication of broadening source set.**

If broader sources are eventually permitted, three contracts gain
forward-compat surface area:

1. **Per-source ingest schema.** Today's `IngestParams` is one
   shape for all sources. A `source_type: CommitMessage` would
   carry no `entities` (no LLM extraction upstream) — entity tags
   would have to be derived inside mengdie or left empty. This
   forks the ingest path: AE artifacts arrive pre-extracted; raw
   sources do not. Today's blueprint §6 "LLM-mediated extraction
   at ingest, not raw chunk-and-embed" is an industry-converged
   pattern (analysis.md "Architectural patterns 2") that
   non-AE sources would violate unless they're routed through
   AE-style extraction first. This is the corpus-pollution
   failure mode the framing names.
2. **Per-source provenance contract.** Today `source_file` is a
   git-relative path that the operator can `cat` to verify. Commit
   messages lose this — the message lives in git history, not a
   readable file. The provenance-checkable invariant
   (CLAUDE.md "Why ratify topics are kept" — "future operators can
   cite by reference") would weaken. Adding source-specific
   provenance schemas (e.g., `commit_sha: Option<String>`, ad-hoc
   per source) is open-ended scope creep.
3. **Per-source contradiction-rule discipline.** Currently
   contradictions fire on entity-tag overlap. AE artifacts have
   curated entity tags; raw sources have noisy/empty ones.
   Allowing in raw sources without enforcing entity-curation
   discipline turns contradiction detection into noise generator.

**The forward-compat cost is non-trivial; YAGNI applies.**

- Today: `source_type: SourceType` enum (5 variants, all AE).
- Forward-compat option A: keep as-is; if a new source ever lands,
  add a variant + a parser. Cost on adoption: linear with sources.
- Forward-compat option B: now generalize to
  `source_type: String` and add a `source_schema_version: i64` for
  per-source dispatch. Cost on adoption: zero (just write a new
  parser). Cost today: enum guarantees lost (caller can submit
  garbage); per-source dispatch indirection added with no
  current consumer.

Option A is the v0.0.1-correct choice. The 028 architecture
conclusion already chose enum-shape decisions of this kind in
Topic 1 (free functions over `&Db`, no `Storage` trait until 2nd
impl materializes — same YAGNI argument). Topic 4 should ratify
along the same axis: AE-only NOW, add typed-source-marker /
per-source-schema when an actual second source is committed.

**The framing's "permanent identity boundary or pragmatic v0.0.1
starting point?" question answered:** there is no architectural
reason to commit to "permanent." The blueprint §1 framing
("AE-aware ingestion + cross-project + self-evolving + locally
inspectable" — the unoccupied space mengdie fills) is identity-
level; the source set is one expression of it. If a future source
genuinely fits AE-style propositional-fact discipline (commit
messages with conventional commits prefixes might qualify; bare
chat logs would not), broadening should require a discussion, not
a free-form expansion.

### Preliminary directional verdict

**Ratify AE-only as v0.0.1 boundary, with explicit re-open
condition: a new discussion is required to broaden the source set,
and the discussion must specify (a) per-source ingest schema,
(b) per-source provenance contract, (c) per-source contradiction
discipline.** This is the discipline the framing's key question 2
asks for. Reversibility is high (enum variant + parser additions
are non-breaking schema-side); the bar is process discipline, not
code cost.

**Architectural rationale:** today's contracts are
single-producer (`source_type` enum, AE-curated entities, AE-format
provenance). Forward-compatting these contracts now, without a
concrete second source, produces dead surface area. The 028
conclusion's Topic 1 YAGNI argument applies here verbatim.

### Reversibility

| Direction | Cost | Trigger |
|---|---|---|
| Ratify → broaden to commit messages | Medium — enum variant + commit-parse extractor + provenance schema | New discussion explicitly per the framing's key questions 2 + 4 |
| Ratify → broaden to chat summaries | High — chat is unstructured; LLM extraction layer needed (the AE plugin does this for AE artifacts, doesn't exist for chat) | New discussion + AE plugin extension or new producer |
| Ratify → permanent identity lock | None code-wise; would just be blueprint §9 entry | Architectural commitment that v0.x post-v1.0 generic thesis (CLAUDE.md "post-v1 generic") would have to revoke explicitly |

### Open Questions

- Does the framing's "post-v0.0.1 broadening discipline" want to be
  recorded as a v0.0.1 deferred BL, or is the discipline implicit in
  "AE-only is the boundary"? Argument for explicit BL: future
  operators / agents reading the conclusion need a concrete trigger
  for re-opening. Argument against: filing a BL for "open a
  discussion later" is process noise.

---

## Topic 5 — Loop-closure signal (open)

### Findings (with file:line evidence)

**The audit table substrate exists and is wired.**

- Schema (`schema.rs:277-298`): `memory_search_audit` (id, query,
  scope, took_ms, searched_at) + `audit_returned_facts` link table
  (audit_id, fact_id, rank). FK declarations are documentary (PRAGMA
  foreign_keys is OFF per 029 YAGNI).
- Three indexes (`schema.rs:294-302`): composite on (searched_at, id),
  reverse-FK (fact_id, audit_id), partial on memory_entries
  (valid_until, id) WHERE valid_until IS NOT NULL. The third
  enables the supersession join referenced in the 028 Topic 4 trigger.
- Strict + best-effort write helpers (`db.rs:289-353`):
  `record_search_audit` (TX-wrapped insert + N link rows, returns
  `audit_id`); `record_search_audit_best_effort` wraps it with
  warn-and-counter on failure. Audit failure NEVER mutates search
  response (`db.rs:328-353`).
- Single fire-point (`search.rs:401-452`): `memory_search_audited`
  orchestrator calls the best-effort wrapper exactly once
  post-filter. Both MCP and CLI surfaces converge here
  (`mcp_tools.rs:216-234`).

**This is sufficient substrate for several loop-closure metrics
without any new event stream.**

| Metric (from framing key question 1) | Computable from F-002 audit alone? |
|---|---|
| Search call frequency | Yes — `COUNT(*) FROM memory_search_audit GROUP BY date(searched_at)` |
| Returned-fact distribution | Yes — `COUNT(*) FROM audit_returned_facts GROUP BY fact_id` |
| Re-research time delta (same query at T0 vs T1) | Yes — `query` field is plaintext; cluster by similarity or exact match |
| Contradiction-detected count | Yes — joinable via `memory_entries.valid_until` (the index supports this) |
| Subsequent-decision-conflicts-prior trend | Yes — supersession join (the 028 Topic 4 query) |
| Search-result-cited rate | **NO** — would require ACK feedback from caller; 028 explicitly rejected ACK as v0.0.1 contract (`docs/discussions/028-v0.0.1-architecture-design/conclusion.md:22-32`) |
| Search-result-used vs ignored | **NO** — same as above |
| Re-search-of-same-topic interval shrinkage | Yes — query similarity over time |
| Empty-result-rate | Yes — F-002 already records returned_fact_ids; empty link list = empty result |

**028 explicitly rejected ACK as v0.0.1 contract.** This forces all
v0.0.1 metrics to be **server-side observable** (the 028 conclusion
phrase). That constraint plus the audit-table substrate means: no
separate event stream is needed; F-002 IS the measurement substrate.

**Operator-visibility constraint (framing key question 2 + topic 5
constraints).** The framing's "must produce a signal the operator
actually checks — a metric that lives only in `~/.mengdie/` and
never gets read is not a measurement" is the binding architectural
constraint. Three options for surfacing:

1. `mengdie stats` CLI subcommand — operator runs explicitly. Low
   coercion ("operator must remember"). Known to be ignored at
   solo-operator scale (industry pattern — most metrics dashboards
   never get visited).
2. MCP tool `loop_status` — caller-facing. AE plugin could call it
   pre-research as a sanity check. Higher coercion if AE skills
   actually call it; zero if they don't.
3. Ingestion-time surfacing — when `memory_ingest` runs, return
   "this project's loop status: N searches in last 7d, M syntheses,
   K contradictions" inline. Forces the operator to see the
   number every time they push an artifact. Highest coercion.
4. A daily report file written to `~/.mengdie/reports/<date>.md`
   that opens in the operator's editor on next mengdie invocation.
   Medium coercion; requires editor-launch surface.

**Architecturally, option 1 + option 3 (with option 2 deferred)
keep the surface minimal and avoid observability sprawl.** Option 1
(`mengdie stats`) is the existing CLI subcommand pattern; option 3
adds a small struct to `IngestOutput`.

**Concrete v0.0.1 minimum metric set.** Based on the 028
audit-table being the only durable substrate, the most defensible
minimum is two metrics (per the framing's "minimum signal set, one
or two items" question):

1. **Search-with-results-rate (7d rolling)**: `COUNT(audit rows
   with returned_fact_ids non-empty) / COUNT(all audit rows)` over
   the last 7 days. Falsification path: if this is < 30% across a
   7d window, the loop is NOT closing — the operator's queries
   are not finding prior facts. This is operator-actionable
   ("stop researching with mengdie unless something changes").
2. **Synthesis-influencing-search rate (lifetime, decay-weighted
   to last 30d)**: of the facts returned by `memory_search`,
   what fraction had `source_type = 'synthesis'`? If syntheses are
   never returned (or returned at < 5% rate), the synthesis pass
   is doing work that nobody benefits from. Joinable via
   `audit_returned_facts → memory_entries.source_type`.

Both are computable from existing F-002 schema with no new event
stream. Both are decay-weighted naturally (rolling-window queries).
Both have a falsification path: the operator can answer "is it
working?" with a number, and the number can drop low enough to
prove it isn't.

**What does NOT need to be in v0.0.1.** Hybrid metrics that combine
quantitative + qualitative (per the framing's "Hybrid combinations
permitted" line) introduce a UX surface for capturing operator
verdicts (thumbs up/down on retrievals, weekly retrospective
prompt). All require either extra MCP-tool calls or interactive
flows. Not needed for "minimum signal." Defer behind a BL with
trigger "if quantitative metrics produce a verdict the operator
disagrees with, file the qualitative companion."

### Preliminary directional verdict

**v0.0.1 minimum signal: two metrics derived from F-002 audit table,
surfaced via (a) `mengdie stats` (or a new `mengdie loop` subcommand)
and (b) inline in `IngestOutput` returned from
`memory_ingest`.** No separate event stream. No ACK contract.
Reversibility is high — additional metrics can be added without
schema migration; surfacing channels are additive.

**Architectural rationale:** F-002 audit table is the substrate
designed for the 028 Topic 4 supersession trigger. It is a
generalizable instrumentation primitive; loop-closure metrics ride
on the same substrate without growing observability sprawl. The
"forced visibility" requirement (framing constraints) is met by
inlining the headline metric in ingest responses — every artifact
push surfaces the loop status.

**The single risk to call out:** ingest-response inlining couples
the loop-closure surface to the ingest-tool API. If the metric set
grows, ingest responses bloat. This is bounded by emitting a single
`loop_status: { searches_7d: i64, hit_rate_7d: f64 }`-shape struct,
not a free-form metrics dump.

### Reversibility

| Direction | Cost | Trigger |
|---|---|---|
| Two-metric audit-derived → +qualitative ACK | High — adds caller-side contract; 028 rejected for v0.0.1 | Two-metric verdict diverges from operator gut feeling for 30+ days |
| Two-metric → +separate event stream | High — new table + new write path; observability sprawl | F-002 audit volume exceeds bucket size that aggregation queries can complete in <1s (i.e., ≥10⁶ rows) |
| Two-metric → +inline surfacing in `memory_search` response | Low — add `loop_status` to `SearchOutput` | Operator misses ingest-side surfacing because their AE skills call search more than ingest |

### Open Questions

- Should `mengdie stats` output be promoted to an MCP tool
  (`loop_status`) so AE skills can call it, e.g., before
  `/ae:retrospect` runs? This is the "is the loop working" surface
  the framing asks about; making it MCP-callable means the AE
  plugin can route operator attention to it. Architecturally
  trivial (one tool wrapper); v0.0.1 sprint scope question.
- Does the headline metric need a baseline period (framing key
  question "Is there a baseline period needed")? Architecturally:
  the 7d rolling window IS the baseline — nothing needs to be
  represented as "we're still in baseline." The first 7 days
  produce a value; whether the operator trusts that value is a
  process concern.
- Where does the "synthesis-influencing-search rate" trigger live —
  Topic 2 (reflection trigger) follow-up BL, or Topic 5 metric? It
  bridges both, which is the right outcome (a synthesis-influence
  metric IS the trigger condition for revisiting cron-only).
