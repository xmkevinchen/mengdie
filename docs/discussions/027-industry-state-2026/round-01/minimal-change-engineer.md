---
agent: minimal-change-engineer
round: 1
topics: [01, 02, 03, 04, 05]
lens: scope-discipline / refuse-over-engineering
timestamp: 2026-05-05
---

# minimal-change-engineer — Round 1

Lens carry-over from Round 0 framing review: every line of v0.0.1 code
has to earn its keep. For each of the five design points I propose the
**minimum defensible answer**, name what additional work `more` would
require, and judge whether `more` is justified for v0.0.1.

The bar is not "what would be nicest to have"; the bar is "what
breaks the core promise (blueprint §2) if we do not ship it." When
the answer to that question is `nothing`, the item is scope creep
for v0.0.1, regardless of how attractive it sounds in isolation.

---

## Topic 1 — Ingest mechanism

### Findings

Two pieces of v0.x infrastructure exist already:

- Push: `src/core/mcp_tools.rs::ingest` (the `memory_ingest` tool —
  exercised by the CLI import path and reachable from any AE skill
  that already calls MCP tools). Hard-error embed-fail semantics
  shipped in F-003.
- Pull: `src/core/watcher.rs` (notify-debouncer over directories) —
  library-level only, never wired to a daemon, no production miles.

Topic 4 (this discussion) ratifies AE-only ingest. AE skills already
do LLM-mediated extraction *upstream* of mengdie — what mengdie
receives is a propositional fact, not a raw markdown blob. That
upstream contract anchors which mechanism is the right minimum:

- **Push**: AE skill calls `memory_ingest` after the LLM-mediated
  extraction step. The bytes that cross the wire are already the
  structured fact (`title`, `content`, `entities`, `source_file`,
  `source_type`, `knowledge_type`). One MCP call. Errors surface
  to the caller skill (visible in the same session the operator is
  watching). Cold-start replay = `mengdie import <dir>` walks the
  tree and emits N pushes; bulk-import is a CLI subcommand, not a
  daemon feature.
- **Pull**: a watcher daemon over `docs/` parses markdown
  *post-hoc*. The semantics are wrong: the daemon sees raw
  markdown and would have to re-derive the structured fact that
  AE *already produced*. Either mengdie re-implements the
  LLM-mediated extraction (duplicates AE's job — explicitly out
  per CLAUDE.md 2026-04-27 reframe), or the daemon naively
  chunks/embeds the file (reverts to v0.x naive-ingest, exactly
  what the rebuild is undoing).
- **Hybrid (push primary + pull fallback)**: doubles the surface
  area to monitor — operator now needs a live "is the watcher
  process alive" signal *plus* the push call-site error visibility.
  Two ways to fail silently. Two paths for cold-start replay
  semantics. Two ingest-time error budgets.
- **Event-driven (queue/bus)**: third process surface. Out of
  v0.0.1 scope by inspection.

The literature mostly does not map cleanly. mem0's "async write
path" is in-process within one Python service, not cross-process
push-vs-pull. LangMem's ReflectionExecutor is for *reflection*
triggers, not ingest. Graphiti MCP is push (the calling agent
invokes the MCP `add_episode` tool). The closest pattern in the
2026 industry survey for *ingest delivery* is push.

### Recommendation

**Minimum: push-only as the v0.0.1 contract.**

- Keep `core/watcher.rs` as a library with its existing tests, but
  do NOT wire it to a daemon in v0.0.1. Mark it as opt-in
  experimental in a comment if it stays in-tree.
- Cold-start replay: implement `mengdie import <dir>` as a CLI
  subcommand that walks ingestable files and emits push calls
  (reuses the same `ingest::ingest_text` / `ingest_text_with_resolves`
  primitives that MCP push uses today; F-003 collapsed both into
  shared helpers — this is essentially free).
- AE plugin side: each AE skill that produces an ingestable
  artifact (`/ae:plan`, `/ae:discuss`, `/ae:review`, `/ae:retrospect`,
  `/ae:analyze`) terminates with one `memory_ingest` MCP call.
  This is the only AE-plugin-side change — file a single BL.

**Scope creep risk if we instead pick hybrid or pull**:

- Pull-as-primary: a daemon that has to be supervised (launchd
  plist #2 alongside the dream plist), restart-on-crash logic,
  log rotation strategy, "is the watcher alive" doctor signal —
  all to re-derive what AE already produced. ~5x the code for
  semantically inferior input.
- Hybrid: every observability story (Topic 5) has to cover two
  surfaces. Every cold-start question has to specify which path
  is authoritative. Net: doubled maintenance for redundant
  delivery — exactly the v0.x-rebuild-against pattern.

### Karpathy load-bearing test

Push: zero new infra; one MCP call per skill termination; reuses
F-003 ingest primitives. **Every line traces to the user's stated
goal** ("AE produces fact → mengdie ingests").
Pull: daemon process + supervision + log routing + doctor signal
+ post-hoc markdown re-extraction. **No line of this is forced by
the core promise**; all of it is recreating AE-side work.

### Agreements (placeholder for Round 2)

_To be filled after reading other agents' Round 1 findings._

### Disagreements (placeholder for Round 2)

_To be filled after reading other agents' Round 1 findings._

### Open Questions

1. AE-plugin-side wiring: should the `memory_ingest` call live in
   the skill itself, or in a thin shared helper inside the AE plugin
   so all skills route through one place? Cheap call — minimum is
   per-skill direct call; helper is a nice-to-have, not v0.0.1.
2. Cold-start `mengdie import <dir>`: should it deduplicate against
   existing rows by content hash, or accept duplicates and rely on
   F-002 contradiction handling? Minimum is no-dedupe + warn on
   re-ingest; full dedupe is a separate BL if duplicate noise shows
   up in audit stats.

---

## Topic 2 — Reflection trigger model

### Findings

v0.x already shipped cron via `resources/com.mengdie.dream.plist`
(launchd, daily 03:00, calls `mengdie dream`). The first real run
produced 13 syntheses against the production DB. **Cron is not a
proposal; it is the running default.** On-demand is also already
shipped — `mengdie dream` is callable from the operator's shell at
any time (`src/bin/cli.rs::cmd_dream`).

So the candidates split into two camps:

- **Already-shipped, zero-new-code**: cron, on-demand. Both work.
  Both are operationally observable (`/tmp/mengdie-dream.log` from
  the launchd plist; stdout of the shell invocation).
- **Each requires new runtime metrics mengdie does not compute**:
  salience-threshold, composite (SCM), debounced submit-dedupe.

### Karpathy load-bearing test, applied per candidate

| Candidate | New code required for v0.0.1 | Load-bearing for core promise? |
|---|---|---|
| **cron** | None. Already operational; produced 13 syntheses. | YES — closes the loop today. |
| **on-demand** | None. `mengdie dream` already exists. | YES — operator-driven manual trigger. |
| **salience-threshold** | per-memory `importance` score (new column + scorer); AE outputs carry no like/star/highlight signal, so importance has to be either heuristic (length, entity count) or LLM-judged (one extra LLM call per ingest). New configuration knob. | NO — there is no observed pain that "synthesis fires too late on important facts." |
| **composite (SCM)** | Entropy computation per cluster (new — not in `clustering.rs`); conflict-density rolling window (no current tracker; F-002 audit table is per-search not per-conflict); composite trigger service. | NO — same observation; no v0.0.1 evidence cron-only is misfiring. |
| **debounced submit-dedupe (LangMem)** | In-process executor in mengdie's stdio MCP server (must not block tool dispatch); persistent dedupe queue (process restart must not lose pending reflections); concurrency model for write-events crossing the executor. | NO — v0.0.1 ingest volume is solo-operator pace; debouncing solves a problem that has not appeared. |

mem0's own state-of-memory-2026 lists "reflection trigger that is not
cron or on-demand" as **unsolved**. Picking salience / composite /
debounced for v0.0.1 means committing to either inventing a solution
to an open industry problem or transcribing an academic-paper trigger
without empirical validation.

### Recommendation

**Minimum: cron + on-demand. Both already shipped; zero new code.**

- Document that cron is the v0.0.1 default (`resources/com.mengdie.dream.plist`
  + `docs/operations/dreaming-decay.md`).
- Document that `mengdie dream` is the on-demand escape hatch.
- File the other three candidates (salience, composite, debounced)
  as backlog items with concrete trigger conditions:
  - **Salience-threshold**: revisit when audit stats show ≥3
    operator-flagged "synthesis missed an obviously-important fact"
    incidents per month.
  - **Composite (SCM)**: revisit when ingest volume exceeds N/day
    (requires audit-stats baseline first — pin N empirically) AND
    cron-once-daily is shown to leave high-entropy clusters
    unconsolidated for >24h.
  - **Debounced submit-dedupe**: revisit when ingest write-event
    rate exceeds debounce-window-equivalent of cron+on-demand
    (concretely: >1 ingest per 10 minutes sustained).

**Scope creep risk if we instead pick salience / composite / debounced**:

- Salience: importance scoring is a one-way door. Adding it later
  is a column + backfill; building it speculatively now is a
  non-trivial chunk of LLM-budget code with no measured win.
- Composite: entropy + conflict-density are *separate metrics from
  F-002 audit data*. They each need their own collection scaffolding.
  Net: at least two new tables (or two new in-memory aggregators
  with persistence) plus a new trigger service. ~10x the code of
  cron with no observed problem to solve.
- Debounced: in-process executor inside an stdio MCP server is a
  concurrency-correctness problem (must not hold stdin's read
  loop, must persist across process restart). High-bug-rate code
  for low-empirical-value trigger.

### Karpathy load-bearing summary

**Cron + on-demand is provably load-bearing**: 13 syntheses produced,
operator workflow uninterrupted, zero new code for v0.0.1 to ship
the trigger story.

The other three triggers are answers to questions v0.0.1 has not
asked. File them; do not build them.

### Agreements (placeholder for Round 2)

_To be filled._

### Disagreements (placeholder for Round 2)

_To be filled._

### Open Questions

1. Should `mengdie dream` produce a stderr/stdout summary the
   operator actually reads, or is the launchd `/tmp/mengdie-dream.log`
   sufficient? Minimum: the existing log file is enough; if Topic 5
   (loop signal) demands richer output, route it through the
   `audit-stats / doctor` subcommand (BL-014) rather than dream's
   own output.
2. Is there any v0.0.1 case where cron's daily cadence is provably
   too slow? If yes, the minimum is "cron-twice-daily" (one plist
   line edit), not switching to a metric-bearing trigger.

---

## Topic 3 — Cross-project default retrieval scope

### Findings

This is a **ratify** topic. CLAUDE.md Key Design Decisions §5 is the
prior commitment: "Global storage, per-project default search —
avoid migration cost when adding cross-project later." The bar for
revising is "evidence to overturn", not preference.

Implementation state:

- Storage is already global at `~/.mengdie/db.sqlite`.
- `project_id` inferred from git context (`src/core/project.rs`,
  human-readable id since plan 005).
- `memory_search` already exposes `scope: 'global'` opt-in
  (`mcp_tools.rs:192`).
- Per-call cross-project search is therefore **already a one-arg
  change at the call site** — no infrastructure migration needed
  to flip the default later if evidence emerges.

What evidence exists for revision *today*?

- F-002 audit table just shipped. **It has not yet collected
  enough data** to show what fraction of operator queries genuinely
  benefit from cross-project sources. Any Round 1 verdict that the
  default is wrong is necessarily based on speculation, not data.
- Framing.md lists three "fights the operator" scenarios (new
  project rediscover / cross-cutting Rust idiom / multi-project
  same-problem). All three are **hypothetical loss patterns**, not
  observed. The operator may have these problems, but the system
  has not yet measured them.

### Recommendation

**Minimum: ratify §5 unchanged + record explicit reopening trigger.**

- **Decision**: per-project default; cross-project via explicit
  `scope: 'global'` opt-in. Unchanged.
- **Reopening trigger**: when F-002 audit data accumulates to
  ≥30 days post-deployment AND shows EITHER (a) ≥10% of operator
  searches use `scope: 'global'` *and* return non-empty results
  *and* the same operator manually reissued a project-scoped query
  for the same intent within the same session (signal: per-project
  default is making the operator redo work), OR (b) operator
  explicitly reports ≥3 cross-project rediscovery incidents in a
  retrospect cycle.
- **Reversibility**: HIGH. Storage is already global; flipping the
  default is one constant change in `mcp_tools.rs:192`. No
  migration. The audit-stats subcommand (BL-014) is the right
  place to surface the relevant signal when it ships.

**Scope creep risk if we instead pick "per-call config / per-skill
policy / per-memory-type rules"**:

- "Per-call config decided by calling skill" → each AE skill needs
  to be taught when to opt cross-project. This is N skill changes
  on the AE side, plus a docs surface ("when does ae:analyze go
  cross-project?"), plus operator-confusion when skills disagree.
- "Per-memory-type policy" (e.g., Rust-idiom type cross-project,
  decision-type per-project) → introduces a memory-type taxonomy
  that does not currently exist; the `knowledge_type` field is
  free-form today. Building taxonomy upfront, before any data tells
  us the categories are real, is the v0.x reinventing-infra
  pattern this rebuild explicitly rejects.

### Don't re-deliberate solved problems

The framing classifies this as ratify-or-defer for a reason. Per-
project default ships and works. The cross-project opt-in escape
hatch already exists. There is no v0.0.1-blocking question here.

The ONE thing v0.0.1 should commit to is the *trigger condition*
that would reopen this — so future operators reading audit data
have a falsifiable rule for when to revisit. The trigger is the
artifact; the decision itself is unchanged.

### Agreements (placeholder for Round 2)

_To be filled._

### Disagreements (placeholder for Round 2)

_To be filled._

### Open Questions

1. Should the audit-stats subcommand (BL-014) explicitly compute
   the "per-project default vs cross-project usage" ratio as one
   of its dashboards, so the trigger above is mechanically
   observable? Minimum: yes, one extra SQL query in BL-014.

---

## Topic 4 — Ingest source boundary

### Findings

This is a **ratify** topic. CLAUDE.md Project Status (2026-04-27
strategic reframe) commits: "mengdie = AE 的大脑 ... AE plugin
handles in-session LLM-driven processing ... mengdie receives
AE-distilled propositional facts as ingest input." Blueprint §1
elaborates: "Not a generic ingestion endpoint (no arbitrary markdown
/ PDF / chat transcripts — only AE pipeline artifacts and similarly
structured outputs)."

What v0.0.1 use case would force broadening?

- **Commit messages with conventional prefixes** (fix:, feat:): the
  proposition density is low (commit messages encode "what changed"
  not "why we decided X over Y"); the AE pipeline already captures
  the why in `conclusion.md` / `retrospect.md`. Including them
  duplicates without adding signal.
- **Chat summaries**: by definition pre-LLM-extraction. To ingest
  them mengdie would need to do extraction itself — exactly the
  CLAUDE.md-2026-04-27-rejected layering.
- **Issue / PR content**: structured but not propositional; lots of
  tracking-state noise.

In none of these cases does the operator's *current* workflow
demand v0.0.1 ingest those sources. AE pipeline artifacts are the
high-signal-to-noise input; the analysis.md "Industry Practice
Comparison" point 1 calls this out as mengdie's specific niche.

### Recommendation

**Minimum: ratify AE-only.** No forward-compat hooks.

- **Decision**: v0.0.1 ingest sources = AE pipeline artifacts only
  (`conclusion.md`, `plan.md`, `review.md`, `retrospect.md`,
  `analysis.md`, `discussion.md`).
- **Reopening trigger**: when the operator identifies a *specific*
  fact class (a) consistently produced outside AE pipeline AND
  (b) high-value for the AI feedback loop AND (c) cannot be
  retrofitted into an AE skill. All three are required; missing
  any one means the answer is "fix it upstream in AE", not
  "broaden mengdie's ingest."
- **Reversibility**: MEDIUM. Storage and search are
  source-agnostic by design (already), so adding a source later
  means adding an ingest pathway and an extraction discipline for
  that source. No schema change. No migration. The `source_type`
  column is already free-form text and accepts new values.

**Scope creep risk if we instead pick "build forward-compat for
sources we may add"**:

- Typed source markers / per-source filters / generic ingest
  schema → API design work *now* for sources we are not adding *now*.
  YAGNI. v1 API breakage cost (the steel-man for forward-compat) is
  hypothetical; v0.0.1 has zero external consumers (no public
  release; the v0.0.1 thesis is "personal-use rebuild").
- Worse: any forward-compat scaffolding bakes in assumptions about
  what *future* sources look like. Those assumptions will be wrong
  more often than not — that is the entire reason CLAUDE.md
  鼓励 simpler-first.

### YAGNI assertion

Building flexibility for hypothetical future ingest sources is the
exact failure mode the v0.0.1 rebuild is correcting against v0.x.
The right move is: ratify, document the trigger, ship.

### Agreements (placeholder for Round 2)

_To be filled._

### Disagreements (placeholder for Round 2)

_To be filled._

### Open Questions

1. Should the trigger condition above be expressed in the F-001
   ingest plan as a `source_type` whitelist (technical guard) or
   as documentation only? Minimum: documentation only. A whitelist
   is artificial constraint that adds future migration cost when
   the trigger fires. The conceptual boundary is enforced by AE
   plugin discipline — only AE skills call `memory_ingest` —
   not by mengdie-side filtering.

---

## Topic 5 — Loop-closure signal

### Findings

F-002 just shipped the audit table:

```
memory_search_audit (id, query, scope, took_ms, searched_at)
audit_returned_facts (audit_id, fact_id, rank)
```

Per the F-002 conclusion, the audit hook fires on every
`memory_search` call (MCP + CLI), best-effort write, with a
`METRIC_AUDIT_WRITE_FAILURES` counter for failure-mode visibility.

028 conclusion locks: "MCP `memory_search` ACK feedback: NOT in
v0.0.1 contract. Triggers must be server-side observable." This
is a *hard constraint*. Any signal that requires the calling
agent to send back "yes I used this fact" is out of v0.0.1 scope.

What signals does F-002 enable *with zero new schema*?

| Signal | Source | Cost |
|---|---|---|
| Search-call rate per day | `memory_search_audit` row count over time | one SQL query |
| Empty-result rate | `memory_search_audit` rows with no matching `audit_returned_facts` | one SQL query |
| Repeat-query density | `memory_search_audit.query` group-by | one SQL query |
| Returned-fact id histogram | `audit_returned_facts.fact_id` group-by | one SQL query |
| Cross-project usage ratio | `memory_search_audit.scope = 'global'` count | one SQL query |
| Top recalled facts | `audit_returned_facts.fact_id` join `memory_entries.recall_count` | one SQL query |

What signals would require **new** infrastructure?

| Signal | Why it needs new infra |
|---|---|
| Search-result-cited rate | Requires the calling agent to ACK that a returned fact was actually used in its output → ACK feedback explicitly out of v0.0.1 per 028. |
| Re-research time delta | Requires defining "what is a re-research" + correlating searches across sessions → heavy. |
| Contradiction-detected trend | Existing `METRIC_CONFLICT_COUNT` counter is in-memory only; trend requires persistence + time-series storage. |
| Operator like/dislike marks | New MCP tool surface; new schema; new AE plugin wiring. |

### Recommendation

**Minimum: the audit-stats / doctor CLI subcommand (BL-014 already
filed). Zero new schema. One new CLI subcommand. SQL queries only.**

The minimum loop-closure signal set is two items:

1. **Quantitative**: `mengdie audit-stats` (BL-014) emits a daily
   summary derived purely from F-002:
   - Search calls: total, project-scoped, global-scoped
   - Empty-result rate (loop is being called but mengdie is mute)
   - Top 10 returned facts (which memories are doing the work)
   - Repeat queries (operator searching for the same thing
     repeatedly = potential rediscovery loss)
2. **Qualitative**: the operator's own `/ae:retrospect` cycles. Each
   retrospect already touches mengdie content. The operator answers
   "did mengdie save me work this period? where did it fail?" — this
   answer is the qualitative ground truth and requires zero mengdie-
   side instrumentation.

The forced-confrontation requirement ("operator actually checks the
signal") is satisfied by:

- Quantitative: pinning `mengdie audit-stats` to the existing
  launchd plist (one extra `<string>audit-stats</string>` line, or
  a second plist) so the daily summary appears in the dream log
  (or its own log). Zero engineering.
- Qualitative: it is already enforced by AE retrospect's existence.

**Scope creep risk if we instead pick "comprehensive observability"**:

- Search-result-cited rate: violates the 028 ACK-feedback constraint.
  Either v0.0.1 keeps that constraint and skips the metric, or
  v0.0.1 reopens 028 — high cost.
- Re-research time delta: requires defining + correlating; this is
  a semi-research project, not a v0.0.1 metric.
- Contradiction-trend persistence: turns the in-memory metrics
  counter into a time-series table — separate schema work,
  separate read path, separate retention policy. None of this is
  load-bearing for the loop signal: BL-014 already covers
  contradiction visibility via existing audit data joins.

### What `value` minimally means

Per blueprint §2, the loop's value is "the operator's prior thinking
informs the operator's next thinking." The minimum signal that
falsifies "the loop is closing" is:

- **Search calls happen** but **always return empty** (blueprint
  is wired but corpus is dead) → empty-result rate.
- **Search calls happen, return non-empty**, but the same
  operator-issued query reappears within hours/days → repeat-query
  density (rediscovery the loop should have prevented).
- **Search calls do NOT happen at all** during AE invocations →
  zero-row days (loop is not being called).

All three are derivable from F-002 audit data. None require new
schema. All three force the operator to look at the data; if the
operator never reads the daily summary, the daily summary will
show the corpus is fine even when the loop is dying — but that
is a property of any signal at solo-operator scale and is not
fixable by adding more metrics.

### Agreements (placeholder for Round 2)

_To be filled._

### Disagreements (placeholder for Round 2)

_To be filled._

### Open Questions

1. Where does `mengdie audit-stats` output go? Stdout (operator
   pipes it); a log file (launchd); a markdown report in
   `~/.mengdie/reports/`? Minimum: stdout + launchd capture, like
   dream. Anything more is structure for structure's sake.
2. Should the empty-result rate threshold be configurable? Minimum:
   no — print the raw number; the operator decides what is "too
   empty" without a config knob.
3. Should retrospect explicitly include a "mengdie loop" prompt
   from AE plugin side? Out of mengdie scope; this is an AE
   plugin-side BL if anything.

---

## Cross-topic discipline summary

| Topic | Decision | New v0.0.1 code | If we did `more` |
|---|---|---|---|
| 1 ingest mechanism | push-only + `mengdie import <dir>` cold-start | one CLI subcommand | hybrid/pull = daemon + supervision + post-hoc extraction (5x more code, semantically inferior) |
| 2 reflection trigger | cron + on-demand (already shipped) | zero | salience/composite/debounced = 10x more code, no observed problem |
| 3 cross-project scope | ratify §5 + record reopening trigger | zero (one trigger note) | per-call/per-skill/per-type policy = AE-side N changes + new taxonomy |
| 4 ingest source boundary | ratify AE-only + record reopening trigger | zero (one trigger note) | forward-compat = API design for hypothetical sources |
| 5 loop signal | F-002 audit + `mengdie audit-stats` (BL-014) | one CLI subcommand (already filed as BL-014) | comprehensive obs = ACK feedback (violates 028) + new event streams |

**Total new v0.0.1 code these decisions imply**: two CLI
subcommands (`mengdie import <dir>`, `mengdie audit-stats`) + one
AE-plugin-side BL (each terminal AE skill calls `memory_ingest`).
Three small, self-contained units of work.

**Total `more` we are explicitly NOT doing in v0.0.1**: watcher
daemon, salience scoring, composite trigger metrics, debounced
in-process executor, per-call/per-skill/per-type cross-project
policy, source-type forward-compat scaffolding, ACK feedback
contract, separate event streams for loop measurement.

Each item we are NOT doing has either an explicit trigger
condition for reopening (Topics 2/3/4/5) or is gated by a 028
constraint (Topic 5 ACK).

This is the minimum that delivers the core promise (blueprint §2)
end-to-end: AE artifact lands → mengdie ingests (Topic 1, push) →
on the next AE invocation in the same area, the AI sees relevant
prior facts (Topic 4 ratifies the ingest source; Topic 3 ratifies
the search scope) → operator can verify the loop is closing
(Topic 5 audit-stats) → reflection consolidates over time
(Topic 2 cron + on-demand).

Anything else is for v0.0.2+.
