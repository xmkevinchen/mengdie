---
discussion: "027"
round: 1
agent: ai-engineer
lens: "ML / LLM-driven reflection, embedding clustering, synthesis pipelines"
created: 2026-05-05
focus: [topic-02, topic-05, topic-01, topic-03, topic-04]
---

# Round 1 — AI-Engineer findings

Lens: reflection trigger machinery, embedding-based synthesis, signal
quality. Read first:
`framing.md`, the five `summary.md` files, `analysis.md` (academic
literature + OSS frameworks), `docs/plans/010-dream-synthesis.md`,
`docs/plans/009-embedding-clustering.md`,
`src/core/dreaming.rs`, `src/core/synthesis.rs`,
`src/core/clustering.rs`, `src/core/metrics.rs`,
`src/core/schema.rs` v6 migration (F-002 audit tables),
`docs/discussions/029-f-002-audit-table-design/conclusion.md`.

Primary focus: Topic 2 (reflection trigger) and Topic 5 (loop signal).
Secondary: Topic 1 (whether ingest mechanism shape constrains
synthesis-quality choices later), Topics 3 + 4 (light touch — these
are mostly outside the ML lens).

---

## Topic 2 — Reflection trigger model

### Findings

**The five candidates re-stated in ML terms.**

| # | Candidate | Trigger fires when… | Required runtime metrics | New machinery |
|---|---|---|---|---|
| 1 | cron | wall clock matches a schedule | none | none — already shipped |
| 2 | on-demand | operator runs `mengdie dream --synthesize` | none | none — already shipped |
| 3 | salience-threshold (Generative Agents) | accumulated importance ≥ τ | per-memory importance score | LLM-mediated importance scoring per ingest, running sum, threshold compare |
| 4 | composite (SCM) | entropy > 0.9 OR conflict density > 0.3 OR elapsed > 1h | per-cluster Shannon entropy on embedding distribution; conflict count from contradiction events; write timestamps | entropy estimator over recent embeddings, conflict-density tracker, scheduler that wakes on event AND timer |
| 5 | debounced submit-dedupe (LangMem ReflectionExecutor) | each ingest enqueues; executor coalesces within window W and runs once | write-event timing | in-process executor with debounce window, coalesce keyed by something (project_id? cluster centroid?) |

**Empirical defensibility per analysis.md.**

- **Cron** has zero ML-defensibility — it is a backstop, not a quality
  argument. Generative Agents (Park 2023) explicitly contrasts
  threshold-triggered reflection with periodic reflection and shows
  importance-driven firing produces better higher-order memories.
  Cron's only real argument is operational simplicity, which the
  framing already concedes ("stable, predictable; already operational").

- **Salience-threshold** is the canonical academic pattern. Generative
  Agents Round 0 reflection fires when sum-of-importance crosses ~150
  (paper §A.2). The paper makes salience a first-class memory-stream
  field, scored by an LLM call at ingest time ("On the scale of 1 to
  10, where 1 is purely mundane … rate the likely poignancy"). The
  ML cost is 1 LLM call per ingested memory. **Gap for mengdie:**
  AE-distilled facts have no inherent "poignancy" axis — a
  conclusion.md has the same propositional weight as a retrospect.md;
  the operator did not pre-rank. Salience would have to be derived
  (cluster-membership novelty, contradiction-trigger count, source
  type weighting) which is itself a research project.

- **Composite (SCM)** is the most mechanistically detailed analog
  (analysis.md cites this). But the SCM thresholds (entropy > 0.9,
  conflict density > 0.3) are calibrated for a specific embedding
  distribution and corpus size; transplanting them onto mengdie's
  214-memory corpus with no calibration is empirically indefensible.
  Even SCM authors note thresholds need per-deployment tuning.
  **Cost in mengdie's stack:** entropy over what? — If "entropy of
  embeddings of the last K memories," that's a Shannon estimator over
  384-dim continuous vectors, typically done via k-means binning or
  histogram on dimension-reduced data. Conflict density is more
  tractable (mengdie has `superseded_by` lineage and a contradiction
  module) but requires plumbing contradiction events into a rolling
  window — net new code, ~150–250 LoC plus tests.

- **Debounced submit-dedupe (LangMem)** is the most ergonomic at
  developer-tool scale. Each `memory_ingest` call enqueues; a
  background executor wakes after a quiet period and runs synthesis
  on whatever was enqueued. This MATCHES mengdie's actual usage
  pattern (AE pipeline writes happen in bursts: a `/ae:work` session
  produces 3–5 memories in 15 minutes, then nothing for hours).
  **Cost in mengdie's stack is the highest of any candidate**: the
  current process model is stdio-MCP-server-per-Claude-Code-session.
  There is no daemon. A `tokio::spawn` background task is ephemeral
  to the MCP session — when Claude Code exits, the executor dies
  with unflushed work. To make debounced robust, mengdie either
  (a) runs a separate long-lived daemon (a P0-shaped infrastructure
  change), or (b) accepts that debounce only works while a Claude
  Code session is open — which is most of the operator's productive
  hours, but is exactly the wrong window for synthesis (the operator
  is using the data, not waiting for it to be reflected). Option (b)
  is the LangMem pattern; it works for chat assistants because the
  chat *is* the foreground process. For a developer-tool MCP server
  it inverts the schedule.

- **On-demand** has a hidden virtue: it is the only candidate where
  the operator's engagement is the trigger. mem0's
  state-of-memory-2026 explicitly lists "reflection trigger that
  isn't cron or on-demand" as the unsolved problem; the framing
  inherits this dichotomy. But for a *solo-operator developer-tool*,
  on-demand is not a degenerate case — it is an operator-in-the-loop
  trigger that aligns reflection cost with operator attention.

### Are the metric-bearing triggers tractable for v0.0.1?

**Salience: not tractable as a primary trigger.** The salience score
itself requires either (a) an LLM call per ingest (doubles ingest
cost; conflicts with v0.0.1's "AE produces propositional facts —
mengdie does not re-process them" boundary from CLAUDE.md), or
(b) a heuristic proxy (entity-tag count, source-type weight) which
is itself a research artifact requiring validation. v0.0.1 cannot
ship a defensible salience trigger.

**Composite: partially tractable, but value is poor at the corpus
size.** The conflict-density component is a 1–2 day engineering
task — mengdie's `superseded_by` column and contradiction module
already provide the events; a rolling 30-day window counter on top
of F-002's audit table would be straightforward. But entropy on a
214-memory corpus is statistically meaningless — any reasonable
estimator has variance comparable to the threshold. **The composite
trigger is academically clean and operationally useless until corpus
size grows by 10×.**

**Debounced: tractable only if the v0.0.1 deployment grows a daemon.**
That is a P0 infrastructure decision, not a Topic 2 decision.
Arguing for debounced-as-default forces a shape change to the rest
of v0.0.1. The framing intentionally walls "event-driven
alternative (queue, message bus)" out of Topic 1 scope; debounced
trigger is the same shape change wearing a different hat.

### Minimum viable trigger framing

**The Topic 2 question is mis-shaped if read as "pick one
candidate."** A more defensible v0.0.1 framing:

> v0.0.1 ships with **cron-baseline + on-demand override** (both
> already exist). The trigger module is structured behind a
> `ReflectionTrigger` trait with a single `should_fire(&self,
> context: &ReflectionContext) -> bool` method. v0.0.1 implements
> two: `CronTrigger` (true when scheduler fires) and `OnDemand`
> (true when CLI invoked). Future triggers (salience, composite,
> debounced) implement the trait without re-architecting
> `run_synthesis_pass`.

This framing is empirically defensible because:

1. It does NOT claim cron is the right long-term answer — it claims
   cron is the right *baseline* until the corpus grows enough to
   make salience or composite meaningful. mem0's state-of-memory
   is honest evidence that the field has not solved this; mengdie
   does not have a duty to solve it at v0.0.1.

2. It preserves optionality. The single biggest cost of "pick one
   trigger now" is that the choice cements `run_synthesis_pass`'s
   call site. A trait-based seam (parallel to `LlmProvider`
   already in `src/core/llm.rs`) costs ~80 LoC and means salience
   / composite / debounced can land as v0.0.2 or v0.0.3 BLs
   without touching core synthesis logic.

3. It is honest about the empirical situation. "We picked cron
   because it works, and we built the trigger seam so the choice
   is reversible" is a stronger position than "we picked salience
   because Park et al. said so on a different corpus."

### Costs of this framing

- **Trait surface cost: ~80 LoC + 4 tests.** Same shape as
  `LlmProvider` (analysis.md confirms swiftide-style trait
  abstractions are the consensus Rust pattern).
- **Risk of premature abstraction**: yes. But the abstraction is
  one method (`should_fire`) over an existing implementation
  (cron). If v0.0.2 adds salience and the trait turns out wrong,
  the rewrite cost is the same as if we had cron-only today —
  the trait gives optionality without locking shape.
- **Risk of trait surfacing into the MCP API**: no. The trait is
  internal. MCP exposes `mengdie_dream` (CLI/operator surface).

### Agreements (placeholder)

To be filled by other Round 1 agents.

### Disagreements (placeholder)

To be filled by Sweep / synthesis if other agents push back on:
- Whether the trait-seam approach is justified at v0.0.1 (could be
  attacked as YAGNI by minimal-change-engineer or doodlestein-strategic).
- Whether on-demand should be the default rather than cron — strong
  case here: cron forces a deployment-surface (launchd plist) that
  is friction the operator must maintain; on-demand has zero
  ambient cost and matches "v0.0.1 = AE 的大脑, operator-driven."

### Open Questions

1. **Should the v0.0.1 default be cron OR on-demand?** I lean
   on-demand (lowest ambient surface, no plist to maintain, aligns
   with operator-attention-aligned reflection). Cron is the
   *fallback* — operator opts in via launchd plist if they want
   nightly runs. This inverts the v0.x default and reduces
   operational surface.
2. **What metric would tell us cron is the wrong default?** If we
   ship cron, the falsification signal is "cron fired but produced
   ≤1 synthesis on N consecutive runs" (over-firing on a quiet
   corpus). On-demand has no over-fire failure mode by
   construction.
3. **Where does the trigger trait live?** `src/core/trigger.rs`
   alongside `llm.rs`. Implementation: `impl ReflectionTrigger for
   CronTrigger`, `impl ReflectionTrigger for OnDemandTrigger`.
   `run_synthesis_pass` accepts `&dyn ReflectionTrigger` instead
   of bool flags. Cleanly testable; ~50 LoC seam.

---

## Topic 5 — Loop-closure signal

### Findings

**"Instrumented" vs "Measured" — the load-bearing distinction.**

The framing asks for "minimum signal that confirms the loop is
delivering value, not just being called." That phrasing is sharp —
it draws the line between **instrumentation** (events recorded) and
**measurement** (events interpreted). F-002's audit table is
instrumentation. It records `(query, scope, took_ms,
returned_fact_ids)` per search. It does not measure anything by
itself. The 029 conclusion is explicit: "v0.0.1 ships write-only.
The supersession SQL is a contract … but has no v0.0.1 in-binary
caller."

That gap is the entire Topic 5 question. F-002 instruments. Topic 5
must specify what gets *read* from the instrumentation, by whom,
when, and what the read produces.

### ML-flavored signals — a comparative ranking

For each candidate signal: data substrate, computability, and
falsification value (does it produce a number that could fail).

| Signal | Substrate | Cost | Falsifiable? |
|---|---|---|---|
| **Supersession rate** | F-002 audit + `superseded_by` column | One SQL query, O(audit rows × link rows). 029 R4 indexes are sized for it. | Yes — "facts retrieved 30 days ago that are now superseded" should trend ≠ 0 over months |
| **Synthesis-cited rate** | NEW event stream needed: when ae:analyze Round 0 injects a synthesis, did the agent's output reference the synthesis? | High. Requires AE-side instrumentation: agent must emit "I used memory X" or text-match heuristic on agent output | Hard. Heuristic citing detection is noisy; LLM self-report is unreliable |
| **Contradiction-detection trigger rate** | Existing contradiction module + counter | Already counted (`METRIC_CONFLICT_COUNT`). Just expose. | Yes — should trend > 0 if mengdie is detecting decision drift |
| **Cross-session re-research time delta** | NEW: timestamp of first ae:analyze on a topic vs subsequent ones | Very high. Topic identity is fuzzy (same topic, different wording); needs entity-overlap or embedding-similarity to define "same topic" | Hard. Confound with topic difficulty changes over time |
| **Round 0 injection acceptance rate** | NEW per-injection event: was the injected block surfaced/used by the agent in research? | Same problem as synthesis-cited rate. AE plugin is the right layer to measure this. | Hard. Needs AE-side hook |
| **Per-search nonempty rate** | F-002 audit table — `audit_returned_facts` JOIN on audit | Already computable. `nonempty/total` over rolling window | Yes — "operator runs searches but mengdie returns nothing" is exactly the negative signal |
| **Manual operator verdict (qualitative)** | New: weekly `mengdie reflect` prompt that asks operator "did mengdie short-circuit anything this week?" | Free — no instrumentation, just UX | Yes by definition |

### Is F-002 audit data sufficient for v0.0.1?

**Yes for two of seven signals (supersession rate, per-search
nonempty rate).** The 029 schema captures the right columns
(`query, scope, took_ms, searched_at` + `audit_id, fact_id, rank`)
to compute both without new tables. Specifically:

```sql
-- Supersession rate (29 framing.md, R7 schema)
SELECT
  COUNT(DISTINCT a.id)                                    AS audits_with_superseded,
  CAST(COUNT(DISTINCT a.id) AS REAL) /
    NULLIF((SELECT COUNT(*) FROM memory_search_audit WHERE searched_at > ?), 0) AS rate
FROM memory_search_audit a
JOIN audit_returned_facts l ON l.audit_id = a.id
JOIN memory_entries m       ON m.id = l.fact_id
WHERE m.valid_until IS NOT NULL
  AND a.searched_at > ?;

-- Nonempty search rate
SELECT
  CAST(SUM(CASE WHEN EXISTS (
    SELECT 1 FROM audit_returned_facts l WHERE l.audit_id = a.id
  ) THEN 1 ELSE 0 END) AS REAL) / COUNT(*) AS nonempty_rate
FROM memory_search_audit a
WHERE a.searched_at > ?;
```

Both are O(audit rows) under R4's `(searched_at, id)` index.

**Five of seven signals require new instrumentation** — most
critically, anything involving "did the agent USE the result." AE
plugin is the right layer for that hook (it sees the agent's
output); F-002 only sees mengdie's output. Citing-rate /
acceptance-rate / re-research-delta are AE-side instrumentation
problems, not mengdie-side.

### "Loop instrumented" vs "loop measured" — concrete distinction

- **Instrumented (v0.0.1 has this)**: every `memory_search` writes
  one row to `memory_search_audit` and N rows to
  `audit_returned_facts`. F-002 ships this.
- **Measured (v0.0.1 does NOT have this)**: there is a procedure
  that reads the instrumentation, computes a number, and surfaces
  it to the operator on a frequency the operator actually checks.

The 029 conclusion says read path is deferred. That means F-002
alone cannot answer Topic 5 — it provides the substrate, but the
measurement procedure is missing.

### Minimum viable measurement (Topic 5 proposal)

**Ship one quantitative signal + one qualitative prompt.**

1. **Quantitative: per-search nonempty rate.** Computed from F-002
   audit data, no new schema. Surfaced via `mengdie stats`
   (existing CLI) extended with a "loop signals" section. If this
   trends < 30% over 7 days, the operator is running queries that
   mengdie cannot answer — either the corpus is too small, or
   search is broken, or the queries are wrong-shape. Falsification
   path: < 30% over 7 days → operator is forced to investigate.
2. **Qualitative: weekly retrospective hook.** When the operator
   runs `ae:retrospect`, ae:retrospect's output template asks:
   "Did mengdie surface anything this week that you would
   otherwise have re-discovered? (yes/no/idk)". This is one line
   in retrospect output, zero new schema, and forces the operator
   to confront whether the loop is working.

These two together force the operator to confront both the
mechanical question ("is it returning data") and the value
question ("is the data short-circuiting work"). Neither requires
LLM-per-measurement (constraint 2 in the framing). Both surface in
places the operator already looks (constraint 3: "metric that
lives only in `~/.mengdie/` and never gets read is not a
measurement").

**Anti-recommendation: do not ship synthesis-cited rate or
re-research time delta in v0.0.1.** Both require AE-plugin-side
hooks that don't exist. Both are measurement infrastructure
projects that exceed v0.0.1 scope. File them as P1/P2 BLs with
explicit triggers ("revisit when ae:analyze emits structured 'I
used memory X' provenance").

### Falsification path

The blueprint's loop ("AI tools produce knowledge → mengdie
ingests → feeds context → better output → richer knowledge →
spiral") is not yet falsifiable. Topic 5 must produce a concrete
"this is how I would prove the loop is NOT closing for me right
now":

> If, over a 14-day window with ≥ 5 ae:analyze invocations,
> mengdie's nonempty rate stays < 20% AND the operator's weekly
> retrospect verdict is "no" or "idk" twice in a row, the loop
> is not delivering value. **Action**: stop running mengdie or
> dig into why search returns nothing.

This is a falsification rule, not a contract. The numbers are
informed by analysis.md ("Perplexity Memory: 77% → 95% recall by
storing half as many — high recall is achievable"). 20% is not a
calibrated threshold; it is a "this is clearly broken" floor.

### Industry reference — is this new ground?

analysis.md says: "no OSS framework instruments solo-operator-scale
loop closure." I verified by searching the four named OSS
frameworks for measurement primitives:

- **mem0**: `add_memory_metric` events for write counts; no
  measurement of whether retrieval was useful.
- **Letta**: `archival_memory_search` returns results; no
  cited-rate or value signal.
- **LangMem**: ReflectionExecutor produces metrics on coalesced
  events; nothing on retrieval value.
- **Graphiti**: query latency + DMR / LongMemEval benchmark
  numbers in publications; no per-deployment loop-closure metric.

**LongMemEval (ICLR 2025, arxiv:2410.10813)** is the closest
academic benchmark — it tests whether long-term memory frameworks
recover prior context. mengdie cannot run LongMemEval out of the
box (it's a benchmark, not a runtime metric), but the
methodology is informative: LongMemEval's recall metric is
essentially "retrieve-and-check," which IS measurable on
mengdie's audit table (recall = nonempty rate when the operator
re-asks a known-stored question).

So Topic 5 is not new ground in *what to measure* — recall and
citing-rate are well-known IR metrics — but it IS new ground in
*who measures it* (the solo operator's `mengdie stats`, not a
benchmark harness). That matters for design: the measurement
must be operator-runnable, not researcher-runnable.

### Agreements (placeholder)

To be filled by other Round 1 agents.

### Disagreements (placeholder)

Anticipated:
- doodlestein-adversarial may push back on the qualitative
  retrospect-hook signal as "vague" — defense: see falsification
  rule. Two "idk" verdicts in a row is unambiguous.
- minimal-change-engineer may push back on extending `mengdie
  stats` as scope creep — defense: extending an existing CLI
  output with one section is the smallest possible measurement
  surface.
- codex-proxy / gemini-proxy may argue for synthesis-cited rate
  as more directly tied to the loop thesis — defense: AE-side
  instrumentation is out of v0.0.1 scope; file as P1 BL.

### Open Questions

1. **Where does `mengdie stats` write the loop-signals section?**
   stdout (current), or a dated file in `~/.mengdie/reports/`?
   Current `mengdie stats` is interactive-CLI-only. For weekly
   review, a file is more durable — the operator can grep it.
2. **What window for the rolling metrics?** 7 days for nonempty
   rate (matches a work week). 30 days for supersession rate
   (matches A-MEM trigger window per 029). Both configurable, both
   defaulted.
3. **Does the qualitative retrospect-hook need agent-side
   support?** Probably yes — `ae:retrospect` template needs one
   line added. That's an AE-plugin change, but a one-line one.
   File as a precondition BL on the AE plugin side.

---

## Topic 1 — Ingest mechanism (lighter touch from ML lens)

### Findings

The mechanism choice does not strongly constrain synthesis quality.
Cluster + LLM-summarize works the same regardless of whether memories
arrived via push (`memory_ingest`) or pull (watcher). What the
mechanism DOES affect:

- **Latency between fact-write and synthesis-eligibility.** Push
  delivers a fact synchronously; the next synthesis pass sees it.
  Pull adds filesystem-detection latency (notify event → debounce →
  parse → embed → insert), typically sub-second on macOS but
  bursty under heavy IO. Either way, the cron / on-demand trigger
  cadence dominates the ingest-to-synthesis latency. Mechanism
  choice matters at the < 1 second scale; trigger choice matters
  at the > 1 hour scale.

- **Robustness of "what facts exist when synthesis runs."** Push
  has a clear contract: caller is responsible. Pull has a fuzzy
  contract: facts exist iff the daemon was running when the file
  was written. From the ML lens, push is preferable because it
  makes the fact set deterministic per session — synthesis runs
  on a known input set rather than "whatever the watcher caught."

- **Bulk-import / cold-start.** Push needs an explicit bulk
  command (`mengdie import-dir docs/`). Pull naturally replays.
  This matters for the v0.0.1 bootstrap (mengdie's own corpus is
  ~214 facts in `docs/`); a one-time bulk import is fine.

**Recommendation: push primary.** The watcher library exists in
v0.x but has zero production miles per the framing. Pulling that
into v0.0.1 carries unproven-code risk for a problem (file → fact)
that push solves cleanly. Push aligns with rmcp's request /
response semantics. The watcher library should be archived (or
explicitly marked "experimental, not v0.0.1") rather than wired
to a daemon.

### Synthesis-quality consequence

None at the mechanism level. The clustering algorithm (BL-006)
operates on `memory_entries` rows; it is mechanism-blind.

### Open Questions

1. **Should the ingest API pre-validate "AE-shaped propositional
   fact"?** The current `memory_ingest` MCP tool accepts any
   structured fact. If Topic 4 ratifies AE-only, should
   `memory_ingest` reject non-AE inputs at the MCP boundary, or
   trust the AE plugin to be well-behaved? Soft-fail (log
   non-AE input, accept anyway) is safest.

---

## Topic 3 — Cross-project default scope (lightest touch)

### Findings

From the ML lens, cross-project search has one specific failure mode
relevant to synthesis: **cluster contamination**. If
`cluster_memories` ever runs cross-project (it currently does NOT —
clustering is per-project per the BL-006 plan), the LLM summarizer
gets clusters that mix project-A's "use Arc<Mutex<T>>" with
project-B's "Avoid Arc<Mutex<T>>" — opposite decisions from
different stacks. The synthesis output would be incoherent.

So: **per-project default search is the right ML default**.
Cross-project clustering is the wrong shape — different projects
have different stacks, conventions, and conclusions; a cluster
that crosses projects is by definition a low-cohesion cluster.
Cross-project *retrieval* is fine when scoped by entity-tag and
weighted by source-project recency, but cross-project
*synthesis* is dangerous and should remain explicitly opt-out
(per-project default per BL-006 already enforces this).

**Recommendation: ratify §5 unchanged.** Add an explicit annotation
that synthesis (cluster + LLM consolidate) is per-project ONLY,
not just retrieval. This guard is already enforced by
`cluster_memories(db, project_id, ...)`; the discussion should
record it as an intentional ML-level invariant, not an accident.

---

## Topic 4 — Ingest source boundary (lightest touch)

### Findings

From the ML lens, AE-only is the right v0.0.1 boundary because
**signal quality dominates corpus size at this scale**. analysis.md
cites Perplexity's 77% → 95% recall improvement *by storing half as
many memories*. The lesson generalizes: at sub-1000-fact corpus
sizes, admission filtering beats volume.

AE pipeline outputs are already filtered (LLM-mediated extraction
upstream, structured frontmatter, propositional content). Broader
sources (commit messages, chat summaries) carry lower signal /
noise. Mixing them at v0.0.1 dilutes the synthesis input
distribution.

**Recommendation: ratify AE-only.** Record that the cap is a
signal-quality argument, not an arbitrary v0.0.1-pragmatic choice.
Specifically: AE-only because AE outputs are LLM-pre-distilled;
adding a source means subjecting it to equivalent extraction
discipline upstream, not loosening the boundary downstream.

---

## Cross-topic note: dependency between Topic 2 (trigger) and Topic 5 (signal)

These two are coupled in a way the framing does not yet make
explicit:

- If Topic 2 ships **cron-only**, Topic 5's measurement runs at
  cron cadence too — the operator sees a daily / weekly summary.
- If Topic 2 ships **on-demand**, Topic 5 must surface measurements
  on each invocation OR via an explicit `mengdie stats` run.
- If Topic 2 ships the **trigger trait** I proposed, Topic 5 can
  attach measurements to specific triggers (e.g., on each cron
  fire, also log the resulting nonempty rate to a dated file).

**The trigger seam and the measurement surface should ship
together**, because the measurement question ("is the loop
working?") is downstream of the trigger question ("when does the
loop run?"). Filing them as separate BLs is fine; treating them
as architecturally independent is not.

---

## Summary of recommendations (by topic)

- **Topic 2**: ship **on-demand as v0.0.1 default + cron as
  opt-in**, behind a `ReflectionTrigger` trait. Salience /
  composite / debounced filed as P1+ BLs with explicit triggers
  (corpus size > 1000, daemon infrastructure decision, AE-side
  importance hook). Honest about mem0's "unsolved" framing —
  v0.0.1 does not pretend to solve it.
- **Topic 5**: ship **per-search nonempty rate** (computed from
  F-002 audit) **+ qualitative retrospect-hook**. Both surface
  via `mengdie stats` and `ae:retrospect`. Falsification rule:
  nonempty < 20% over 14d AND two "idk" retrospect verdicts → loop
  is not delivering. Synthesis-cited rate / re-research-delta
  filed as P1+ BLs requiring AE-side instrumentation.
- **Topic 1**: push primary, watcher archived (lightest ML
  consequence; mechanism is largely orthogonal to synthesis
  quality).
- **Topic 3**: ratify §5; add explicit ML-invariant note that
  synthesis is per-project ONLY.
- **Topic 4**: ratify AE-only on signal-quality grounds (Perplexity
  empirical evidence).
