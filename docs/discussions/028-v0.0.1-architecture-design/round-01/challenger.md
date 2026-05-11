---
id: "028"
round: 1
author: challenger
lens: pure-opposition
created: 2026-04-28
---

# Round 1 — Challenger

## Findings (with evidence + push-back per topic)

### Topic 1: Storage trait + search-split scope

**Position: HOLD my conditional from analyze phase. Still unresolved.**

The framing restates the conditional correctly: if search-split is
in-sprint, define Storage trait in the same change; if not, defer.
What the framing does NOT answer is whether search-split is in scope.
That is still an operator call, not an architectural one.

YAGNI application with current evidence:

- Single SQLite impl exists. Tier 2 Kuzu has no commit date, no sprint
  assignment, no trigger that has fired. The §7 ladder is explicit:
  advance only when the trigger fires. The trigger is "cross-project
  graph traversal becomes a regular need OR contradiction chains exceed
  ~3 hops." Neither is observable at current corpus size (~214 facts).
- The search-split refactor IS independently valuable: the current
  `impl Db` grafting makes Retrieval invisible as a boundary. Splitting
  `search.rs` functions to module-level API is justified as cleanup
  even without Storage trait. These are separable decisions.

**Push-back on bundling them:**

The summary conflates two distinct changes — (a) moving search
functions to module-level (valid, cheap, improves call-site clarity)
and (b) introducing a Rust trait on top of that (adds abstraction with
no second impl). You can do (a) without (b). If the operator accepts
the search-split as part of the two-ingest-paths defect fix, that does
NOT automatically justify (b).

**Concrete demand:** if Storage trait is in scope for v0.0.1, name the
second impl that will exercise the trait boundary in the same sprint.
"Future Kuzu" is not an answer. If no second impl is named, the trait
is YAGNI and the answer is free functions over a Connection handle —
adequate until the Tier 2 trigger fires.

**Mechanism question (framing opened this, peers may not address it):**

The framing explicitly opened: trait / struct / free fns / none. The
analyze phase defaulted to "Rust trait" without justifying why not
free functions. Free functions over `&Connection` give you module-level
API (fixing the grafting problem) without introducing a trait that
nothing else implements. This is the lightest option and fully
consistent with blueprint §6 "do not introduce abstractions not earned
by current need." I expect this option to be under-argued in other
Round 1 writes.

### Topic 2: Bi-temporal event_time

**Position: MAINTAIN reject-permanently stance unless operator provides
the 60-second counter-example.**

My falsifiable demand from analyze phase: one AE artifact in
production where `event_time ≠ ingested_at` by > 60 seconds.

No evidence has been presented. The topic-02 summary acknowledges the
demand, names the exception scenario (post-hoc documentation), but
does not answer it. That is operator domain knowledge — no amount of
Round 1 research resolves it.

**What "post-hoc documentation" actually means:**

The exception is: operator writes `conclusion.md` today documenting a
decision made three months ago. Is this a first-class workflow? If yes,
the column is justified. If it's a rare edge case (single occurrence in
six months of production), the column is schema speculation.

**Counter-argument I'm anticipating and my response:**

"The column is cheap to add now; migration cost is higher later."

Response: migration cost is not zero. Every query that reads from this
table now needs a mental model of two timestamps. Every future code
path that ingests a fact needs to supply or default `event_time`.
Defaulting to `ingested_at` is always available, but the column's
presence creates a question ("why are these different?") that the v0.0.1
codebase cannot answer. Dead schema is a maintenance burden disguised as
future-proofing.

**My pre-registered flip condition:** operator names ≥1 concrete case
where post-hoc documentation is part of their AE workflow. Single
instance is sufficient — it proves the workflow exists.

### Topic 3: Reflection collapse + Reflector trait

**Two separate questions. I insist on treating them separately.**

**Sub-question A: Module collapse (clustering + synthesis → dreaming)**

The empirical evidence is strong: clustering.rs and synthesis.rs are
exclusively imported by dreaming.rs. There is no caller of either
module that bypasses dreaming. The module boundary is a file-system
boundary, not an API boundary. File-system boundaries are organizational
choices, not contractual ones.

The argument for deferring until the sqlite-vec spike resolves: if
sqlite-vec ANN replaces hand-rolled clustering, clustering.rs may be
deleted rather than merged. Collapse decision is moot. This is the
right reasoning. **I maintain: defer collapse until spike resolves.**

**Sub-question B: Reflector trait — is the sqlite-vec spike's outcome
actually a "second reflection strategy"?**

This is where I push back hardest. The framing says:

> "if sqlite-vec adoption introduces a 2nd reflection strategy
> in-sprint, satisfying the ≥2 impls condition"

This conflates storage backend with reflection algorithm identity. Let
me be precise:

The current reflection algorithm is: cluster related facts by cosine
similarity → LLM synthesizes a meta-fact from each cluster → store the
synthesis. If sqlite-vec ANN replaces hand-rolled cosine clustering, the
**clustering step uses a different data structure** but the reflection
algorithm is the same: cluster → synthesize → store. The algorithm's
identity has not changed. You have not introduced a second reflection
strategy. You have swapped the clustering implementation.

A second reflection strategy would be something like: temporal decay
pass (which facts have not been recalled and should be demoted) — this
is structurally different from the synthesis pass. Those two are
legitimate candidates for abstraction. Does dreaming.rs currently
implement both? Yes — promotion and decay are separate from clustering
and synthesis. But they share `dreaming.rs` and are called from the
same entry point.

**Revised YAGNI test for Reflector trait:**

Before introducing Reflector, answer: do the two strategies need to be
independently configurable, or independently composable, by any caller
in v0.0.1? If the answer is "they're both called from the `dream` CLI
subcommand and from the launchd cron in sequence," then abstraction
buys nothing at v0.0.1. The caller doesn't select strategies; it runs
all of them.

**My demand:** name a call site in v0.0.1 that needs to select one
Reflector strategy over another at runtime. If no such call site
exists, Reflector trait is YAGNI regardless of whether sqlite-vec ships.

### Topic 4: A-MEM deferral trigger

**Position: Deferral is correct. But the proposed trigger conditions
are underspecified. Push for exactly one.**

Three candidate triggers were raised:
1. Corpus > 1k facts
2. Retrieval quality measurably degrading
3. Independent A-MEM replication published

**Push-back on (1) — corpus size:**

Corpus size predicts workload but not retrieval quality degradation.
At 1k facts you might have tight clusters with zero retrieval drift.
At 500 facts you might have contradictory facts that never surface
because the affected entities are never queried. Size is a proxy for
a problem we haven't measured.

**Push-back on (2) — retrieval quality:**

This is the right trigger in principle. The question is how you measure
it. v0.0.1 instruments search calls and what was returned. But "retrieval
quality measurably degrading" requires a baseline, a comparator, and a
threshold. None of those are defined. Without definition, this trigger
never fires because it's not measurable.

**Push-back on (3) — paper replication:**

This is external dependency. Waiting for an academic community to
replicate is not a trigger the operator controls. It is also the weakest
signal: replication confirms the technique works in some setting, not
mengdie's setting.

**My recommended trigger:**

Pick the cheapest measurable condition. The only measurement v0.0.1
will have is: (a) search call log, (b) what facts were returned, (c)
what facts were ACKed (used by the AI). If the ratio of returned-to-ACKed
facts drops below a threshold across N queries, retrieval quality is
degrading. This is measurable from v0.0.1 instrumentation without
additional infrastructure.

Concrete: **trigger = running retrieval precision (ACK/returned ratio)
drops below 0.5 over 20 consecutive queries, OR corpus exceeds 2k
facts AND precision has not been measured at all (forcing the
measurement)**. The OR clause handles the case where the loop is
running but no one is looking at precision yet.

If "ACK" is not part of the MCP contract (the AI tool does not report
back which facts it used), say so explicitly — and then the trigger
should be corpus size only, with the acknowledgment that we can't
measure quality. Don't claim a quality trigger if the instrumentation
won't support it.

---

## Pre-registered priors I'd flip if evidence shows otherwise

1. **bi-temporal column**: I flip from REJECT to ACCEPT if operator
   names even one concrete post-hoc documentation case from their
   actual AE workflow in the past 6 months. Single instance is
   sufficient. "I might do this" does not count.

2. **Storage trait**: I flip from FREE-FUNCTIONS to TRAIT if a second
   concrete impl is named for the same sprint. Kuzu being planned for
   Tier 2 "eventually" does not count. Named BL with concrete trigger
   condition counts.

3. **Reflector trait**: I flip from YAGNI to ACCEPT if a call site
   is named that selects strategies at runtime in v0.0.1. CLI `--strategy`
   flag with ≥2 concrete options would satisfy this.

4. **A-MEM trigger (corpus size only)**: I flip from skepticism to
   acceptance if the team confirms the MCP protocol does not support
   ACK feedback. If we genuinely cannot measure precision, corpus size
   is the best available proxy and I accept it as the sole trigger.

---

## Disagreements I expect to surface in Round 2

1. **Storage mechanism:** Other agents likely default to "Rust trait"
   without examining free functions. I expect to be the only voice
   arguing that free functions are the right mechanism absent a second
   impl.

2. **Reflector trait scope creep:** Other agents may accept that
   sqlite-vec introduces a "second strategy" without examining whether
   the algorithm's identity actually changed. I will hold this
   distinction in Round 2.

3. **A-MEM trigger precision:** Other agents may propose composite
   triggers (size AND quality AND replication) that are unmeasurable in
   practice. I want a single, measurable, independently-triggerable
   condition.

4. **bi-temporal framing:** The architecture-reviewer ACCEPT may have
   influenced framing. I expect other agents to lean toward "add it now,
   it's cheap." I maintain: dead schema is not cheap.

---

## Open Questions

1. **Does the MCP `memory_search` protocol support ACK feedback?** If
   the AI tool never reports which returned facts it used, retrieval
   precision is unmeasurable from v0.0.1 instrumentation alone. The
   A-MEM trigger must be size-only in that case.

2. **Is post-hoc documentation in the operator's actual workflow?**
   This is the single blocking question for bi-temporal. No research
   resolves it — only operator confirmation.

3. **What is the actual defect-fix scope for the two-ingest-paths
   bug?** Does consolidating `mcp_tools.rs:306-331` to call
   `ingest::ingest_document` require moving search functions to
   module-level? If it does, search-split is entangled with the
   mandatory defect fix and should be counted as in-sprint regardless
   of Storage trait decisions. If it doesn't, they're independent scope
   decisions. The topic-01 summary implies entanglement; the analyze
   phase summary is less clear.

4. **If sqlite-vec ANN replaces clustering, what happens to
   `synthesis.rs`?** Synthesis is called by dreaming using the output
   of clustering. If clustering is replaced by an ANN index, is the
   LLM synthesis pass still valid, or does the cluster-identity change
   (from cosine neighborhoods to ANN k-nearest) break the synthesis
   prompt's assumptions? This is a dependency the framing does not
   surface.
