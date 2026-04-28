---
agent: architecture-reviewer
round: 2
created: 2026-04-28
---

# Round 2 — Architecture Reviewer

## Findings (with peer-citation file:line)

### Topic 1 — Free functions vs gemini's CONDITIONAL ACCEPT trait

gemini's position (`round-01/gemini-proxy.md:44-52`):

> "The trait is good; the implementation path matters. Don't introduce
> it without establishing the Retrieval boundary. [...] If search is
> split to module-level API, then the Storage trait becomes narrow and
> well-designed: CRUD + temporal only, with provenance."

gemini's reasoning is structurally sound but draws the wrong conclusion.
The argument is: search-split makes Storage narrow → narrow Storage trait
is earned → therefore introduce the trait when search-split lands. That
chain has a suppressed premise: that "earned" means "introduce now." My
counter: "earned" means "the condition is met"; it does not mean "the
timing is now."

**The gap in gemini's reasoning:**

gemini's Google-ecosystem principle (`round-01/gemini-proxy.md:51-52`) is
"design interfaces early but only for clearly bounded responsibilities."
The "clearly bounded responsibilities" condition applies to the **Storage**
interface. But gemini's own analysis (`round-01/gemini-proxy.md:25-40`)
shows that Mengdie's storage becomes too narrow for SQLite at 3–4 hop
graph traversals. That threshold is not crossed at v0.0.1's corpus size
(~214 facts, no cross-project graph queries). The interface is bounded, but
its value is bounded to the moment when a second impl is needed.

**Why free functions are strictly better at v0.0.1:**

codex-proxy (`round-01/codex-proxy.md:44-63`) cites sqlx's experience: the
`Executor` trait was not object-safe in all variants, and some impls were
deleted because they didn't fit newer crate architecture without rewriting
the trait. Premature trait introduction creates a public API contract to
maintain. At v0.0.1, with one impl (SQLite) and no second impl named for
the same sprint, a trait is a contract without a counterparty.

challenger (`round-01/challenger.md:43-47`) states the YAGNI demand
precisely: "name the second impl that will exercise the trait boundary in
the same sprint." No second impl has been named. Kuzu is on the §7
ladder but has no sprint assignment and no fired trigger.

minimal-change-engineer (`round-01/minimal-change-engineer.md:45-64`)
clarifies the distinction I should have made sharper in Round 1: the
search-split (move functions to module-level free fns) is the
right change; the trait on top of that is the premature step. The
search-split produces the same module-level API whether or not a trait
exists. A future Kuzu integration can lift from `fn search(db: &Db, ...)`
to `fn search<S: Storage>(store: &S, ...)` with a mechanical change that
touches one function signature per call site.

**Conclusion on Topic 1:**

gemini's conditional ("trait only if search-split is in scope") is
internally consistent but collapses into free functions under the YAGNI
pressure from challenger, minimal-change, and codex. The trait WOULD be
well-placed if Kuzu were in-sprint. It is not. Free functions are the
correct v0.0.1 mechanism. I update my Round 1 position to be sharper on
this: the search-split **is** in scope (either as part of the defect fix
or as a separate cleanup PR alongside it), the trait is **not** in scope,
and these two decisions are independent.

**Attempted falsification of the free-fn position:** Could there be a
caller today that requires runtime swappability of the storage backend? No.
The single caller surface (MCP tools, CLI) is concrete and imports `Db`
directly. No mock in tests replaces `Db` with a trait object — tests use
`Db::open_in_memory()`. Runtime swappability is not a current need.
Falsification fails. Free functions hold.

---

### Topic 2 — REJECT permanently vs codex's DEFER with trigger

codex-proxy's position (`round-01/codex-proxy.md:125-131`):

> "Add a concrete trigger to the backlog: 'Add bi-temporal schema when
> the operator ingests the first artifact whose creation time and
> underlying decision time differ by > 60 seconds in production, or
> when post-hoc documentation becomes a regular workflow.'"

The operational difference between REJECT permanently and DEFER with trigger:
- REJECT permanently: re-opening requires a new discussion, a conscious
  decision to revisit.
- DEFER with trigger: fires automatically when condition is met; no
  additional discussion gate.

**Why REJECT permanently is better for THIS case:**

The trigger codex proposes ("first artifact where creation-time differs
from decision-time by > 60s") requires detecting, at ingest time, that
the caller-provided timestamp differs from the server-side `ingested_at`.
This detection requires the caller to supply an `event_time` parameter
that the system does not currently accept. The trigger cannot fire from
observing existing production data — it can only fire if someone adds a
parameter to pass the timestamp in. That is itself a schema and API
change decision, not a measurable condition from the current system.

In other words: codex's trigger is not "automatically fires when condition
is met in production" — it is "fires when a human decides to pass a
new parameter into the ingest API." That decision IS a new discussion. The
trigger is structurally equivalent to "re-open via a new discussion when
someone needs this." That is what REJECT permanently means.

**The alternative I proposed in Round 1** — optional `valid_from` override
on `memory_ingest` — covers the only legitimate use case (bulk import of
historically-dated artifacts) without a schema column. It is a single
optional parameter on the ingest API. This can be added under a minor
version bump without a schema migration, and it handles the post-hoc
documentation case without bi-temporal semantics (the import sets
`valid_from` = the historical date; there is no `event_time` separate from
`valid_from`). The v0.x schema already has `valid_from`; callers just
cannot set it today.

**Engaging codex's governance concern:**

codex's concern is: REJECT permanently makes it "harder to re-open if
evidence materializes." I accept that cost. The evidence that would justify
bi-temporal is operator-domain knowledge (does post-hoc documentation
happen?). That evidence should emerge through the operator's workflow,
not through an automatic threshold firing. When the operator says "I want
to ingest a document I wrote today about a decision I made three months
ago and have it show up in searches at the correct date," the right
response is to add an optional `valid_from` parameter — not to add a
second timestamp column. The full bi-temporal model (event_time as a
separate column with its own semantics in contradiction detection and
search ranking) requires a discussion to confirm that the simpler
alternative (caller-supplied `valid_from`) is insufficient.

**gemini's position on Topic 2** (`round-01/gemini-proxy.md:69-85`)
agrees with DEFER on the condition that "post-hoc documentation materializes
as a workflow." This is compatible with REJECT permanently + "re-open if
this happens." gemini does not require automatic trigger firing; it uses
the same condition codex does. The governance difference (auto-fire vs
re-open-via-discussion) is what separates my position from codex's, not the
condition itself.

**Conclusion on Topic 2:**

REJECT permanently, with the standard carve-out: if the operator identifies
a concrete post-hoc documentation workflow that the optional `valid_from`
override cannot serve, re-open via a new discussion. The trigger is a
conscious human decision, not an automatically-firing condition, because
the underlying evidence is operator domain knowledge rather than a
measurable system metric.

---

### Topic 3 — UAG candidate

**UAG position: YES, pass both sub-decisions.**

Sub-decision A: "Defer Reflection consolidation pending sqlite-vec spike outcome."

All 5 agents converge on defer:
- architecture-reviewer (`round-01/architecture-reviewer.md:Topic 3`): defer
- minimal-change-engineer (`round-01/minimal-change-engineer.md:120-141`): defer
- challenger (`round-01/challenger.md:98-115`): defer
- codex-proxy (`round-01/codex-proxy.md:141-175`): defer
- gemini-proxy (`round-01/gemini-proxy.md:120-122`): defer

No dissent. UAG PASS on consolidation deferral.

**Falsification attempt A:** Is there a reason to force the collapse NOW
despite the sqlite-vec spike? The only argument would be: "the current
3-file split is causing active confusion or integration errors." Evidence
check: there are no import cycles, no build errors from the split, no
test failures attributable to the boundary. The split is fictional as an
API boundary but harmless as a file organization. No falsification found.

Sub-decision B: "Do not introduce Reflector trait in v0.0.1 regardless
of sqlite-vec outcome."

Four explicit NO votes:
- architecture-reviewer (`round-01/architecture-reviewer.md:Topic 3`): NO
- minimal-change-engineer (`round-01/minimal-change-engineer.md:143-157`): NO
- challenger (`round-01/challenger.md:115-151`): NO (demands runtime call
  site selecting strategies)
- codex-proxy (`round-01/codex-proxy.md:164-167`): NO (only one strategy)
- gemini-proxy (`round-01/gemini-proxy.md`): silent on Reflector trait
  directly

gemini's silence is not a dissent — gemini's findings on Topic 3 address
collapse timing and async reflection triggers, not the trait itself.
gemini does not argue FOR the Reflector trait.

**Falsification attempt B:** challenger's observation
(`round-01/challenger.md:133-145`) that promotion/decay and clustering/
synthesis are two structurally different reflection operations that both
live in `dreaming.rs` is the sharpest challenge to "only one strategy."
If promotion+decay and clustering+synthesis are distinct algorithms, does
a Reflector trait abstracting both satisfy the "≥2 strategies" condition?

No. The test is not "≥2 algorithms" but "≥2 runtime-selectable strategies"
(challenger `round-01/challenger.md:147-151`). Promotion+decay and
clustering+synthesis are called in sequence from the same entry point; no
caller selects one without the other. There is no call site that runs
synthesis without running the decay pass, or vice versa. The abstraction
has no actual selection point. Even under this charitable reading, the
Reflector trait fails the runtime-selectability test.

UAG PASS on Reflector trait deferral, with gemini's silence treated as
non-dissent (gemini raised no counter-argument).

---

### Topic 4 — 3-AND composite vs other proposals, and the ACK protocol question

**My Round 1 trigger** (top-3 score < 0.35 over 30d rolling window of ≥ 20
queries, AND corpus > 500 facts, AND avg entity cluster size > 5).

**challenger's ACK argument** (`round-01/challenger.md:187-205`):

> "If the ratio of returned-to-ACKed facts drops below a threshold across
> N queries, retrieval quality is degrading. [...] If 'ACK' is not part
> of the MCP contract, say so explicitly — and then the trigger should be
> corpus size only."

This is the sharpest challenge to my position. challenger is correct that
the MCP `memory_search` contract currently has no ACK feedback mechanism.
The server returns results; the calling tool (AE plugin, or Claude) uses
what it uses and the server never learns which facts were used. The
synthesis.md verification artifact (`round-01/synthesis.md:38-42`)
confirms: "MCP protocol does not currently support ACK feedback — verified."

Does this invalidate my top-3 score metric? No, but it limits it.

**What IS measurable server-side without ACK:**
- The RRF-merged score of the top-3 results is computed server-side.
- The score is logged to the domain audit table at search time.
- A rolling average of these scores IS observable from the audit table.
- Score degradation DOES correlate with retrieval quality: when FTS5 and
  vector both return low-confidence matches (low RRF scores), the top-3
  scores drop.

**What is NOT measurable server-side:**
- Whether the caller actually used the results.
- Whether the results were relevant to the caller's task.
- Whether the facts returned were stale at the time of use.

So my score-based trigger is a **proxy for search-side signal quality**,
not end-to-end retrieval quality. It fires when mengdie's search finds
poor matches, not when mengdie returns good matches that the caller ignores.

**challenger's corpus-only fallback** (`round-01/challenger.md:194-199`):
"corpus size only, with the acknowledgment that we can't measure
precision." This is an honest position. But corpus size is the weakest
proxy: it fires from steady ingestion regardless of quality trends.

**minimal-change-engineer's stale-fact trigger**
(`round-01/minimal-change-engineer.md:162-178`): "≥ 5 instances in 30
days where a `memory_search` returned a fact that was later superseded
within 7 days of the search." This is elegant and measurable from the
audit table + `valid_until` column. No ACK needed: we know what was
returned (logged); we know when it was superseded (`valid_until`); the
join tells us when we served stale results. This is a better quality
signal than my RRF score proxy.

**Integrating minimal's stale-fact signal:**

I update my composite to incorporate minimal's insight. Revised trigger:

> Re-open A-MEM bidirectional update when ALL of:
> 1. Corpus ≥ 500 facts in a single project (sanity floor — below this,
>    cluster size is too small for bidirectional update to matter).
> 2. The domain audit table shows ≥ 5 instances in a rolling 30 days
>    where `memory_search` returned a fact that was invalidated
>    (`valid_until` set) within 7 days of that search call. This is
>    measurable from the audit log + invalidation timestamp without ACK.
> 3. Avg entity cluster size > 5 (structural precondition: if entity
>    clusters are too small, bidirectional update has no neighbors to
>    re-evaluate — the operation is vacuous).

This drops my opaque RRF-score threshold (< 0.35) in favor of
minimal's concrete stale-result count (which directly measures the
harm A-MEM is designed to prevent) and retains the corpus floor and
entity-cluster-size precondition.

**Why not codex's 10k-fact threshold**
(`round-01/codex-proxy.md:253-266`)?

codex (`round-01/codex-proxy.md:246`) argues "< 1k facts: definitely
premature; 1k–10k: tune retrieval first; 10k+: start A-MEM experiments."
These thresholds are calibrated to Graphiti/Zep production workloads
(hundreds of users, high ingest rate, millions of tokens). Mengdie is a
solo operator with ~214 facts growing at a slow AE-workflow pace. 10k
facts is years away at current trajectory. Setting the trigger at 10k
effectively defers A-MEM to "never" for this operator's scale. I prefer a
quality-based trigger that fires when harm is observed rather than a
corpus-size trigger that fires on a scale the operator may never reach.

**Why not gemini's 5k-fact threshold**
(`round-01/gemini-proxy.md:167-170`)?

Same argument — 5k is still 20× the current corpus. Quality-first is
better than size-first.

**Why not challenger's single-condition ACK-precision**:

ACK is not in the MCP contract. challenger acknowledges this as their flip
condition (`round-01/challenger.md:222-226`): "I flip from skepticism to
acceptance [of corpus-size-only] if the team confirms the MCP protocol
does not support ACK feedback." The synthesis verification confirms no ACK.
challenger's own flip condition fires: accept corpus size as floor, with
the stale-result count from the audit log as the quality signal.

**Conclusion on Topic 4:**

Updated 3-condition composite:
1. Corpus ≥ 500 facts (sanity floor, not primary trigger)
2. ≥ 5 stale-result deliveries in 30 days (measurable from audit + valid_until, no ACK needed)
3. Avg entity cluster size > 5 (structural precondition)

All 3 must hold. This is measurable from v0.0.1 instrumentation without
ACK protocol changes.

---

## Agreements

| Peer | Topic | Line | Agreement |
|---|---|---|---|
| minimal-change-engineer | T1 | `round-01/minimal-change-engineer.md:45-69` | Search-split and Storage trait are separable; free fns only |
| challenger | T1 | `round-01/challenger.md:43-47` | YAGNI demand: name the second impl; none named; free fns |
| codex-proxy | T1 | `round-01/codex-proxy.md:69-76` | No public Storage trait; search-split independent justification |
| minimal-change-engineer | T2 | `round-01/minimal-change-engineer.md:73-111` | REJECT permanently; borrowing Graphiti pattern without AE evidence is wrong |
| challenger | T2 | `round-01/challenger.md:61-93` | Falsifiable demand holds; dead schema is maintenance burden |
| minimal-change-engineer | T3 | `round-01/minimal-change-engineer.md:120-157` | Defer consolidation; Reflector YAGNI even under sqlite-vec |
| challenger | T3 | `round-01/challenger.md:147-151` | Runtime call site test for Reflector; none exists in v0.0.1 |
| codex-proxy | T3 | `round-01/codex-proxy.md:164-167` | One strategy; Reflector not legitimate until distinct 2nd strategy |
| minimal-change-engineer | T4 | `round-01/minimal-change-engineer.md:162-178` | Stale-result count from audit+valid_until is better quality signal than score proxy (incorporated into my updated trigger) |

---

## Disagreements

| Peer | Topic | Line | My counter |
|---|---|---|---|
| gemini-proxy | T1 | `round-01/gemini-proxy.md:44-52` | Trait conditionality is correct but draws wrong conclusion: "earned" ≠ "introduce now." The boundary is made real by free fns; trait waits for second impl (Kuzu). gemini's "design interfaces early" principle applies at service boundaries with stable contracts, not inside a single-binary codebase with no second impl named |
| codex-proxy | T2 | `round-01/codex-proxy.md:125-131` | DEFER-with-trigger collapses into REJECT-then-new-discussion when the trigger requires human action (passing a new API parameter) to fire. The governance difference is real: auto-fire requires an observable metric, not a design decision. This trigger is a design decision masquerading as an observable metric |
| codex-proxy | T4 | `round-01/codex-proxy.md:253-266` | 10k-fact threshold is calibrated to Graphiti/Zep multi-tenant scale. Mengdie is solo operator at 214 facts; 10k is ~10 years of growth at current rate. Quality-signal trigger (stale results) fires when harm is observable regardless of corpus size |
| gemini-proxy | T4 | `round-01/gemini-proxy.md:167-170` | 5k-fact threshold has same scale mismatch as codex's 10k. At solo-operator pace, size-based triggers effectively defer A-MEM permanently |

---

## Open Questions

1. **Minimal's stale-result trigger (7-day window):** Does the v0.0.1
   domain audit table schema log the fact IDs returned per search call?
   If the audit table only logs the query and count, the stale-result join
   (`audit.returned_ids ∩ invalidated_facts`) is not possible without
   schema changes. This needs verification before committing the 7-day
   trigger as measurable from v0.0.1 instrumentation.
   (`round-01/minimal-change-engineer.md:247-251` raises this same concern.)

2. **Search-split coupling to defect fix:** challenger
   (`round-01/challenger.md:266-273`) asks whether the defect fix
   requires moving search functions to module-level. My Round 2 position:
   they are independent. The defect fix touches `mcp_tools.rs` ingest side
   (lines 282–395); search side (lines 175–276) still calls
   `self.db.memory_search()` after the fix. Search-split is a follow-on
   cleanup that can be its own PR. TL should clarify whether to bundle or
   sequence.

3. **gemini on Reflector trait:** gemini was silent on the trait
   specifically. If gemini holds a latent position compatible with the
   "≥2 distinct strategies" framing from Google's generative agents research
   (Park et al., which gemini cites at `round-01/gemini-proxy.md:93-95`),
   that could surface as a dissent in Round 3. The UAG on Reflector NO
   should be confirmed after gemini explicitly responds.
