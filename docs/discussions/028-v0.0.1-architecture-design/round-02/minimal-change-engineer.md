---
agent: minimal-change-engineer
round: 2
created: 2026-04-28
discussion: 028-v0.0.1-architecture-design
lens: YAGNI / refuse scope creep / 3 similar lines > premature abstraction
---

# Round 2 — minimal-change-engineer

## Findings (with file:line evidence)

### Topic 1 — confirm convergence on free-functions; engage gemini

I read all four peer Round 1 files. Convergence on the trait question
is 4-of-5: arch-reviewer (`architecture-reviewer.md:43-67`),
challenger (`challenger.md:42-58`), codex (`codex-proxy.md:36-78`),
and my Round 1 all reach NO trait + free functions. gemini is the
outlier (`gemini-proxy.md:42-52`).

**Engaging gemini's "Google pattern" reasoning directly:**

gemini's argument (`gemini-proxy.md:46-52`):

> "Introducing `Storage` trait now without splitting search out
> creates a leaky abstraction... If search is split to module-level
> API... then the Storage trait becomes narrow and well-designed:
> CRUD + temporal only, with provenance. Kuzu can implement this
> cleanly. This is the moment the trait becomes earned."

The flaw in gemini's reasoning: gemini conflates *"the API surface
becomes trait-shaped"* with *"a trait should be defined now."* These
are different claims. The search-split DOES make the eventual trait
introduction trivial — but "trivial to introduce later" is a
**reason to defer**, not a reason to do it now. Codex makes this
exact point at `codex-proxy.md:36-44` with the 5-condition test:

> "Introduce now only if **all** are true: Two *real* implementations
> exist (not test mocks); narrow contract; search implementable
> without downcasts; conformance test suite; supported as public API
> through early releases."

Mengdie has 1 real impl, no Kuzu commit, no shared conformance
suite. Fails 4-of-5 of codex's test.

gemini also cites Google's Go pattern at `gemini-proxy.md:52`:

> "In Go (heavily used at Google), interfaces are implicit and matched
> structurally. Google favors designing interfaces early but only for
> clearly bounded responsibilities."

This argument actually cuts the *other* way for Rust/mengdie:

1. **Go interfaces are structurally matched** — adding a Go interface
   doesn't force any change in the impl. **Rust traits are nominally
   matched** — defining a trait forces explicit `impl Storage for
   Db` blocks, every caller decides whether to take `&Db` or
   `&dyn Storage` or `<S: Storage>`, and refactoring the trait
   later is a breaking change. The "design early" pattern doesn't
   port to Rust costs-symmetrically.
2. **"Clearly bounded responsibilities"** is exactly what mengdie
   doesn't have at v0.0.1 for Storage. The bounded responsibility
   gets defined when Kuzu's actual needs are known, not before.

YAGNI rule survives gemini's challenge intact. The "Google pattern"
is real but its applicability to a 1-impl pre-spike Rust codebase
is weak.

**Convergence holds: search-split YES, free functions over `&Db`,
no `Storage` trait in v0.0.1.** Trigger for trait introduction:
named Kuzu sprint with concrete BL committed (per challenger's
flip condition at `challenger.md:217-219`).

### Topic 2 — REJECT permanently vs codex's DEFER with trigger

This is the engagement TL flagged. Reading codex's actual position
at `codex-proxy.md:129`:

> "Add a concrete trigger to the backlog: 'Add bi-temporal schema
> when the operator ingests the first artifact whose creation time
> and underlying decision time differ by > 60 seconds in production,
> or when post-hoc documentation becomes a regular workflow.'"

And arch-reviewer at `architecture-reviewer.md:106-115`:

> "Reject `event_time` column permanently for v0.0.1. File a note in
> the blueprint under §8 open questions: if bulk import of
> historically-dated artifacts becomes a workflow... accept an
> optional `valid_from` override on `memory_ingest` instead of a
> schema column."

The two positions land in similar operational territory. TL is right
that they're "operationally indistinguishable if no AE artifact
ever has a >60s gap." Defending REJECT permanently as cleaner
governance:

**Why REJECT permanently is cleaner than DEFER with trigger:**

1. **The trigger is unobservable to mengdie**. codex's trigger fires
   when "the operator ingests an artifact where creation-time
   differs from decision-time by > 60 seconds." But the AE pipeline
   stamps `created_at` at file-write time and the ingest path reads
   that — see `src/core/ingest.rs:51-62` (the `NewMemory` struct has
   no `event_time` field; the schema doesn't carry the
   distinction). To detect the trigger, mengdie would need to
   *already* be capturing decision-time separately from
   ingestion-time. The trigger requires the column to exist to be
   measurable.

   Compare to A-MEM (Topic 4): "≥5 stale-retrieval instances in 30
   days" is measurable from the audit log mengdie *will already
   have* in v0.0.1. That's a real trigger. Topic 2's trigger is
   chicken-and-egg.

2. **DEFER with trigger implies "we expect this to fire."** The
   governance signal of a deferred BL is "this is on the radar; we
   will revisit." Mengdie does not expect this to fire. Blueprint
   §3.1 lists the temporal model explicitly:
   `valid_from`/`valid_until`/`superseded_by`. There is no
   `event_time` field anywhere in the blueprint. The data model
   the operator committed to is **uni-temporal-with-supersession**.
   `event_time` is not a deferred feature — it's a feature the
   blueprint never agreed to.

3. **The "alternative path" arch-reviewer named is the actual right
   answer.** If post-hoc bulk import ever becomes a workflow, the
   answer is an optional `valid_from` override on the ingest tool
   — see `src/core/db.rs` `NewMemory` struct, which already carries
   `valid_from` semantics via `valid_from` column. That's a one-
   parameter API extension, not a schema column. Permanent
   rejection of `event_time` doesn't preclude this path; it just
   names the right path.

**Concession to codex's framing:** I'll accept "DEFER with trigger"
as operationally adequate IF the trigger is reformulated to be
mengdie-measurable. arch-reviewer's "first artifact where caller
explicitly supplies a `valid_from` differing from now()" would be
measurable — but that's the alternative-path trigger, not the
event-time-column trigger. Different feature.

**Final position: REJECT permanently.** The "alternative path"
(optional `valid_from` parameter) is filed in the blueprint §8 open
questions per arch-reviewer's proposal. That replaces "defer the
column" with "we know the right answer if the workflow emerges."

### Topic 3 — UAG candidate evaluation

TL asked: pass UAG on "defer Reflection consolidation pending
sqlite-vec" + "Reflector trait NO in v0.0.1 regardless of sqlite-vec
outcome"?

**Yes, pass UAG.**

5-of-5 agree on the consolidation defer (synthesis.md:108-114):
arch-reviewer (`architecture-reviewer.md:139-156`), me (Round 1
already), challenger (`challenger.md:99-112`), codex
(`codex-proxy.md:172-176`), gemini (`gemini-proxy.md:120`). No
disagreement.

4-of-5 explicit on Reflector NO; gemini silent
(`synthesis.md:114`). gemini's `gemini-proxy.md:113-115` discusses
salience-weighted reflection patterns but does not propose a
Reflector trait. gemini's silence is consistent with NO, not against
it. UAG threshold of "no opposition" is met.

**Convergent reasoning, with my own summary:**

The Reflector trait fails YAGNI because the "≥2 strategies"
condition is not satisfied by sqlite-vec adoption. arch-reviewer at
`architecture-reviewer.md:161-169` and codex at
`codex-proxy.md:166-167` reach the same architectural distinction:
ANN swaps the *primitive* used by step 1 (clustering); it does not
introduce a different *strategy*. challenger sharpens this further
at `challenger.md:140-148`: "answer: do the two strategies need to
be independently configurable, or independently composable, by any
caller in v0.0.1?" The answer is no — both promotion and clustering-
synthesis are called from the same `dream` CLI subcommand in
sequence.

**One-line falsification attempt:**

The proposition would fail if **someone names a v0.0.1 call site
that selects between ≥2 reflection strategies at runtime, with both
strategies committed in-sprint (not "future Reflexion-style critique
loop", not "future entropy-triggered summary")**. No agent named
such a call site. The challenger's flip condition at
`challenger.md:222-224` matches this falsification attempt exactly
and was unsatisfied by all of Round 1.

**UAG: pass.** Topic 3 can be marked converged/closed before final
sweep.

### Topic 4 — defend external-paper clause OR concede

TL asked me to defend the "1 independent A-MEM replication paper"
external clause OR concede if internal-only is cleaner. Reading
the four peer positions:

- arch-reviewer (`architecture-reviewer.md:199-221`): fully internal,
  measurable from v0.0.1 audit table — "avg top-3 score < 0.35
  over 30-day rolling window of ≥20 queries, AND corpus > 500, AND
  avg entity cluster size > 5"
- challenger (`challenger.md:185-205`): single-condition; precision
  if MCP-ACK, corpus-only if not — depends on whether ACK is in
  v0.0.1 contract
- codex (`codex-proxy.md:251-261`): 4-AND composite with offline
  eval set; requires `insufficient_context_rate >= 15%` on a
  curated 50+ question eval, plus offline ablation showing ≥8pp
  gain
- gemini (`gemini-proxy.md:166-170`): composite — corpus > 5k AND
  retrieval quality degrading AND conflict density > 5%

**Concede on the paper-replication clause.**

Three reasons:

1. **It's not programmatically observable**, as TL flagged. There
   is no signal mengdie can wire up that says "an A-MEM
   replication paper has appeared." It would require a manual
   literature-watch, which is not a trigger — it's a vibe.

2. **arch-reviewer at `architecture-reviewer.md:194-198` is
   correct**: "External replication tells us the technique works in
   principle, not that mengdie's specific workflow needs it." The
   operator's loop is AE-artifact-driven; A-MEM was tested on
   conversation-derived facts. Even a successful replication doesn't
   tell us A-MEM helps mengdie's actual workload.

3. **My own clause-3 was already labeled "optional safety check;
   can be waived if clauses 1+2 hit hard"** in my Round 1
   (round-01/minimal-change-engineer.md `### Topic 4`). It was
   already non-load-bearing. Removing it removes only a comfort
   blanket.

**Updated trigger (conceding clause 3):**

> Re-open A-MEM bidirectional update when BOTH:
> 1. **Corpus ≥ 1k facts in a single project** (sanity floor;
>    below this, cluster-reevaluation cost is negligible).
> 2. **The persisted domain audit shows ≥5 instances in 30 days
>    where a `memory_search` returned a fact that was later
>    superseded within 7 days of the search** — directly measurable
>    from the v0.0.1 audit table without ACK protocol changes.

**Engagement with peer positions on the remaining 2-clause trigger:**

- vs **arch-reviewer**'s "avg top-3 score < 0.35": I prefer the
  supersession-within-7-days metric over score-distribution because
  score thresholds are sensitive to the RRF normalization (see
  `src/core/search.rs:39-64` where boost+decay can move scores by
  20%+) — a score-based threshold could fire from ranking-tuning
  changes alone, not from genuine retrieval drift. Supersession-
  within-window is mengdie's existing semantics applied to its own
  audit log; it doesn't depend on score normalization stability.
- vs **challenger**'s ACK-or-corpus-only: my supersession metric
  doesn't require MCP ACK. The supersession event already happens
  inside mengdie (via `memory_invalidate` MCP tool, see
  `src/core/mcp_tools.rs` invalidate handler). We can join the
  audit log against the supersession log entirely server-side. This
  makes challenger's ACK-vs-no-ACK fork irrelevant for this metric.
- vs **codex**'s 4-AND with curated eval: codex's trigger is
  rigorous but **requires building a curated 50+ question eval
  set**, which is itself v0.0.1 scope creep. The supersession-from-
  audit-log metric needs zero new infrastructure beyond the §5 P0
  audit table. YAGNI says: prefer the trigger that uses what we'll
  already have.
- vs **gemini**'s 5k + quality + conflict-density: 5k is too high
  as a sanity floor (mengdie may never hit 5k in practice; trigger
  effectively never fires). Conflict density is a 3rd clause that
  reintroduces complexity without proportional benefit at solo-
  operator scale.

**Note: the 1k floor and 5-instance threshold remain heuristics**
(per `synthesis.md:121` verification table). I accept them as
heuristics; first measurements in production calibrate them.

## Agreements

- **Topic 1**: 4-of-5 NO trait + free functions. Search-split YES
  alongside the two-ingest-paths defect fix
  (`architecture-reviewer.md:64-67`, `challenger.md:50-58`,
  `codex-proxy.md:71-77`, my Round 1).
- **Topic 2**: 3-of-5 reject permanently (arch-reviewer + me +
  challenger). codex's "DEFER with trigger" lands in similar
  operational territory but the trigger is unmeasurable from
  v0.0.1 instrumentation.
- **Topic 3**: 5-of-5 defer consolidation; 4-of-5 explicit
  Reflector NO + gemini silent-consistent-with-NO. **UAG: pass.**
- **Topic 4**: composite trigger over single-clause; corpus floor
  + retrieval-quality measurement is the convergent shape.

## Disagreements

- **Topic 1 vs gemini**: gemini's "Google pattern" reasoning does
  not survive YAGNI rule under Rust's nominal-typing cost
  asymmetry (see Topic 1 above).
- **Topic 2 vs codex**: codex's "DEFER with trigger" is
  operationally similar to my REJECT permanently, but its trigger
  is unobservable until the column exists. REJECT permanently +
  arch-reviewer's `valid_from` override path is the cleaner
  governance.
- **Topic 4 vs arch-reviewer**: prefer supersession-within-7-days
  over score-distribution metric. Score thresholds are
  normalization-fragile; supersession is semantically grounded.
- **Topic 4 vs codex**: prefer audit-log-derivable metric over
  curated 50+ question eval set. Eval set is v0.0.1 scope creep
  for an A-MEM trigger; YAGNI says use what we'll already have.
- **Topic 4 vs gemini**: 5k floor too high; 3-clause too complex
  for solo-operator scale.

## Open Questions

1. **Does mengdie's audit table support the join I'm proposing for
   Topic 4 clause 2?** I'm assuming v0.0.1's persisted domain audit
   logs `(query, returned_fact_ids, timestamp)` and that
   supersession events are already logged elsewhere with timestamps.
   If the audit table doesn't capture `returned_fact_ids` (just
   query + count), the supersession-within-7-days metric is not
   computable and I fall back to corpus-floor-only — which both
   challenger and codex flag as too permissive. Audit table schema
   is an upstream blueprint §5 P0 design decision; this trigger
   depends on it.

2. **Does the operator accept the search-split as in-sprint with
   the two-ingest-paths defect fix?** arch-reviewer at
   `architecture-reviewer.md:39` explicitly notes: "The search-split
   refactor does NOT fall out automatically from the defect fix" —
   only the ingest side is touched. So search-split is a separate,
   independent in-scope decision. This is a genuine operator call,
   not yet decided in this discussion.

3. **What is the falsification path for the Topic 3 UAG?** If a
   reviewer in Round 3 surfaces a v0.0.1 call site that legitimately
   selects between reflection strategies, the UAG fails. None
   surfaced in Round 1; Round 2 is the last opportunity before
   Sweep.
