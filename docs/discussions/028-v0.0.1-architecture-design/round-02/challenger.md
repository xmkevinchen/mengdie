---
id: "028"
round: 2
author: challenger
lens: pure-opposition
created: 2026-04-28
---

# Round 2 — Challenger

## Meta-decision: Should v0.0.1 `memory_search` include an ACK feedback channel?

**Position: NO. The cost is real; the benefit is premature.**

This is load-bearing for Topic 4 and I argued it in Round 1 without
taking a position on whether ACK _should_ be in the contract. Here is
my explicit position.

### What ACK would require

An ACK channel means the caller (AE plugin calling `memory_search`)
must report back which result IDs it used. Concretely, this requires:

1. A second MCP tool call (`memory_ack` or `memory_search_ack`) after
   the AI finishes processing — or a request/response extension on
   the existing `memory_search` call shape.
2. The AE plugin author (the operator's own AE plugin code) to
   implement the ACK call in every skill that calls `memory_search`
   (ae:analyze, ae:discuss, ae:plan, ae:work at minimum).
3. The operator to define "used" — did the AI mention the fact? Did
   the AI build an argument that depended on it? Did the AI discard
   it after reading? There is no mechanical definition of "used" that
   doesn't require either another LLM call or guessing.

Point 3 is the fatal objection. "ACK" sounds simple but the signal is
ambiguous: an AI that reads 5 returned facts and ignores 4 of them has
"used" none — but it may have refined its reasoning by exclusion. The
ACK signal is contaminated from day 1.

### What ACK costs vs what it buys

**Cost:**
- MCP contract expands: callers must implement a new call pattern.
  This is a contractual obligation imposed on every future integrator.
- AE plugin surface grows: each skill needs ACK instrumentation.
  This is plugin-side work, not mengdie-side work — it costs the
  operator's own plugin development time.
- The signal requires a definition of "used" — which is not
  definitionally settled.

**Benefit if ACK were perfect:**
- Retrieval precision is measurable from server-side logs alone.
- Topic 4 trigger becomes: precision < 0.5 over 20 queries.
- No secondary proxy (corpus size, score distribution) needed.

**Benefit with the actual ACK signal (ambiguous "used"):**
- Retrieval precision estimate is noisy — same problem, worse baseline
  than the score-distribution proxy the arch-reviewer proposed
  (`round-01/architecture-reviewer.md:199-221`).

**My verdict: NO ACK in v0.0.1 contract.** The MCP contract should
be stable and minimal at v0.0.1. ACK is a v0.x extension if and when
the operator decides "used" is operationally definable in their
workflow. Until then, it is speculative instrumentation paid by every
future integrator.

**Consequence for Topic 4:** with no ACK, Topic 4 trigger must be
server-side observable only. The options are: (a) corpus size, (b)
score distribution from the domain audit log, (c) supersession-
within-N-days proxy (minimal-change-engineer's proposal,
`round-01/minimal-change-engineer.md:166-196`). I update my
single-condition proposal accordingly — see Topic 4 section below.

---

## Topic 1: Storage mechanism — engaging gemini's CONDITIONAL ACCEPT

**Gemini's position** (`round-01/gemini-proxy.md:43-54`): the Storage
trait is "conditionally well-timed if and only if the search-split
refactor is in v0.0.1 scope." Reasoning: "CRUD-only is earned by the
act of splitting search out."

**Push-back: the YAGNI rule does not have a conditional form.**

The YAGNI rule is: introduce an abstraction when ≥2 concrete impls
exist or are committed in the same sprint. The search-split refactor
satisfies neither condition. It changes the call-site shape (from
`self.db.memory_search()` to `search::memory_search(&self.db, ...)`)
but does not introduce a second impl of the storage layer. There is
still exactly one impl: `Db` over SQLite.

Gemini's argument is: "if we're cleaning up the boundary anyway, we
should define the trait at the same time." This is the classic scope
creep pattern — work begets adjacent work because it's "natural" to
do it at the same time. Blueprint §6 says the opposite: "do not
introduce abstractions not earned by current need." A Storage trait
earned by a search-split is earned by a cleanup, not by a second impl.

**The Google pattern argument** (`round-01/gemini-proxy.md:50-54`):
"Google favors designing interfaces early but only for clearly bounded
responsibilities." This is industry pattern evidence, not evidence
specific to mengdie's situation. The pattern is: if a boundary is
"clearly bounded," define the interface early. The question is whether
mengdie's storage boundary is clearly bounded when the only impl is
SQLite. It is not — the boundary is not tested by a second impl, so
"clearly bounded" is speculative.

**arch-reviewer's position is stronger** (`round-01/architecture-reviewer.md:42-66`):
"A trait at this point adds a generic parameter or dyn dispatch to
every call site for zero benefit (one impl, no mock needed — tests use
`Db::open_in_memory()`). The free-function refactor is the correct
intermediate step."

**My position unchanged: free functions over `&Db`. No Storage trait
until a second impl is named in the same sprint.** If Gemini can name
a second concrete impl committed to v0.0.1, I update. "Future Kuzu"
without a commit date does not count.

---

## Topic 2: REJECT vs DEFER — pushing both camps on what evidence re-opens

**My Round 1 position:** hold reject until operator names one concrete
post-hoc documentation case.

**Arch-reviewer's REJECT + alternative** (`round-01/architecture-reviewer.md:87-115`):
reject permanently; offer optional `valid_from` override on
`memory_ingest` for bulk import. This is a cleaner resolution than my
"reject until evidence." It names the legitimate use case (bulk
historical import) and offers a cheaper mechanism (one optional
parameter vs a schema column). I concede this framing is better than
mine.

**Minimal-change-engineer's REJECT** (`round-01/minimal-change-engineer.md:73-111`):
reject permanently; blueprint §3.1 lists `valid_from` / `valid_until` /
`superseded_by` but not `event_time`. The current model is
uni-temporal-with-supersession, sufficient for the core promise. Also:
"defer with trigger implies we expect this to fire. I see no evidence
we expect this to fire."

This last point is sharp. Codex's position (`round-01/codex-proxy.md:128-129`)
is "defer with trigger" — trigger: "first artifact where creation time
differs from decision time by >60s in production." But who is watching
for that trigger? It requires monitoring a case that has never
occurred in 214 production facts. A trigger that has never fired and
requires new observability to detect is operationally indistinguishable
from "never."

**Are codex's trigger and my falsifiable demand the same?**

Codex says ">60s creation/decision gap in production." My demand was
"one concrete case from operator's actual workflow." Operationally:
yes, these are the same test. The difference is governance:
- REJECT permanently: re-open requires filing a new discussion. Slower
  but cleaner — forces the operator to deliberately choose to re-open.
- DEFER with trigger: fires automatically when condition met. Faster
  but requires someone to watch the trigger.

Since the trigger has never fired in 6 months of production and requires
new observability to detect, REJECT permanently with the arch-reviewer's
optional `valid_from` alternative is the operationally sound choice.

**My update: REJECT permanently, accept arch-reviewer's optional
`valid_from` parameter as the alternative.** The governance distinction
matters: triggers that require observability to detect are not
self-firing. Reject now; re-open explicitly if post-hoc documentation
becomes a real workflow.

---

## Topic 3: UAG candidate — affirm or decline

**Question:** Does absence of a runtime call site selecting strategies
definitively close the Reflector trait door for v0.0.1?

**My position: AFFIRM the UAG (Reflector trait REJECT for v0.0.1).**

Falsification attempt: could sqlite-vec ANN success constitute a 2nd
reflection strategy?

As I argued in Round 1, and as arch-reviewer (`round-01/architecture-reviewer.md:158-178`)
and minimal-change-engineer (`round-01/minimal-change-engineer.md:142-157`)
independently confirm: ANN-based neighbor finding is a replacement
for the similarity primitive inside the cluster step, not a distinct
reflection algorithm. The algorithm identity is: cluster → synthesize →
store. ANN changes how clusters are found; it does not change what
reflection means.

Minimal-change-engineer's framing (`round-01/minimal-change-engineer.md:153-154`)
is the sharpest: "A function pointer or `enum SimilarityBackend` is the
YAGNI shape, not a `Reflector` trait wrapping the whole pass." If
sqlite-vec adoption makes the similarity step swappable, the correct
abstraction is a local swap at `cluster_memories()`, not a trait
wrapping the entire reflection pass.

**Does the absence of a runtime call site definitively close the door?**

Yes, for v0.0.1. The Reflector trait is YAGNI until:
1. Two strategies exist that are used by the same call site, AND
2. That call site needs to select between them at runtime (not just
   use one sequentially after the other).

No call site exists or is planned in v0.0.1. Closing the door for
this sprint is definitively correct.

**Caveat I pre-register:** if the operator's v0.x.y roadmap includes
a Reflexion-style critique loop or entropy-triggered summary as a
distinct reflection pass, Reflector trait re-opens. But that's a
future sprint; not v0.0.1.

---

## Topic 4: Trigger structure — engaging arch-reviewer's score-distribution composite

**Arch-reviewer's composite** (`round-01/architecture-reviewer.md:199-221`):
"avg top-3 search score < 0.35 over 30-day rolling window of ≥20 queries
AND corpus > 500 facts AND avg entity cluster size > 5."

**Is "avg top-3 score < 0.35" a quality signal without ACK?**

This is the crux. Score here is the 0–1 normalized RRF output from
the hybrid retrieval system. The score measures how well a returned
fact matched the query — it does not measure whether the fact was
useful to the caller.

**The problem:** a fact scoring 0.28 (below the 0.35 floor) might
still be the most relevant fact in the corpus for that query. Low RRF
score means the hybrid ranker was less confident, not that the fact
was inadequate for the operator. Conversely, a fact scoring 0.7 might
be highly similar to the query but completely stale (superseded,
misapplied project). Score measures retrieval relevance; it does not
measure retrieval utility.

**"Score-floor" is the right framing, not "quality signal."** A
rolling average of top-3 scores below 0.35 tells you the retrieval
system is less confident — it found things, but they're weakly matched.
Whether the weakly-matched results were still useful is invisible
without ACK.

**So: is score-distribution diagnostic of the problem A-MEM solves?**

A-MEM's value is: when a new fact arrives, re-evaluate related facts
so stale context is updated. The problem it solves is not "low-
confidence retrieval" — it is "retrieved facts are confident but
outdated because their cluster context was never re-evaluated." A
high-confidence retrieval can still be outdated. Low-confidence
retrieval doesn't necessarily need bidirectional update — it might
need better ingestion or better FTS5 tokenization.

**My verdict: "avg top-3 score < 0.35" does NOT diagnose the A-MEM-
specific problem.** It is a general retrieval quality floor, not an
A-MEM-specific signal.

**Updated trigger proposal (no ACK, server-side observable only):**

Given: no ACK in v0.0.1 contract, score-distribution is a floor not
a quality signal, corpus size is a necessary-but-not-sufficient
condition.

I accept minimal-change-engineer's "superseded-within-7-days" metric
(`round-01/minimal-change-engineer.md:166-196`) as the closest
proxy to "bidirectional update would have helped." The reasoning: if
a returned fact was superseded within 7 days of retrieval, the
retrieval system served stale context — this is exactly the failure
mode A-MEM is designed to prevent. This metric IS measurable from the
domain audit log with a SQL join on `memory_search` log + `valid_until`
column.

**Proposed single trigger (revised):**

> Re-open A-MEM bidirectional update when BOTH:
> 1. Domain audit log shows ≥5 instances in any 30-day window where
>    a retrieved fact was superseded within 14 days of retrieval (fact
>    served that became invalid shortly after serving — stale retrieval
>    evidence).
> 2. Corpus ≥ 500 facts in a single project (sanity floor; below this,
>    the above can't accumulate meaningfully).

Why BOTH not single-condition: condition 1 alone can fire on a single
burst of ingestion. Condition 2 ensures we're at a scale where the
pattern is structural, not incidental.

Why 14 days (not minimal-change-engineer's 7): 7 days may be too
tight for AE workflows where ingestion rate is low (the operator might
not run a new AE discussion on the same topic within 7 days, so the
supersession wouldn't be observable in the window). 14 days aligns
better with the operator's expected AE cadence.

**On codex's composite** (`round-01/codex-proxy.md:252-264`): the
threshold of ≥10k facts or ≥1M source tokens before even
experimenting is too conservative for a solo operator with ~214 facts
at v0.0.1 ship. If codex is right, A-MEM never fires for this
operator. That may be correct — but it means the trigger should be
explicit about "this feature may never be needed" rather than giving
a threshold that sounds reachable but isn't.

**Codex's 10k threshold conflicts with gemini's 5k threshold conflicts
with arch-reviewer's 500 threshold.** The 20× range (500-10k) is
unresolved. I am not resolving the specific number — that is operator
domain knowledge about their expected corpus growth rate. What I insist
on: the trigger must include a behavioral signal (stale-retrieval count)
not just a size threshold. Size-only triggers fire automatically without
evidence the feature is needed.

---

## Responses to peer positions

**gemini on "Storage trait earned by search-split":**
See Topic 1 above. I reject this. The search-split earns the boundary;
it does not earn the trait. Those are separable claims.

**minimal-change-engineer on "reject permanently not defer":**
For Topic 2: I concede. See Topic 2 above — I updated to REJECT
permanently with optional `valid_from` alternative.

**arch-reviewer on "score-distribution as quality proxy":**
For Topic 4: I disagree. See Topic 4 above. Score measures retrieval
confidence, not whether bidirectional update is needed. The proxy is
mis-specified for the A-MEM-specific failure mode.

**codex on "10k facts before A-MEM experiment":**
The threshold may be empirically correct but makes the trigger
effectively unreachable for this operator. Codex should either argue
the feature is never needed (in which case: reject permanently, not
defer) or set a threshold calibrated to the operator's actual corpus
growth trajectory.

---

## Pre-registered priors I'm updating this round

1. **bi-temporal: updated from "reject until evidence" to "reject
   permanently" with optional `valid_from` alternative** — arch-
   reviewer's framing is cleaner. The trigger-that-never-fires is
   operationally equivalent to never.

2. **ACK in v0.0.1 contract: NO.** Explicit position stated. Ambiguous
   signal + contractual burden on every integrator does not justify.

3. **Topic 4 trigger: updated.** No longer single-condition corpus-only
   fallback. Now: stale-retrieval count (≥5 / 30 days) AND corpus
   floor (≥500). The stale-retrieval metric is measurable from the
   domain audit log without ACK.

---

## Remaining disagreements for Round 3 / Sweep

1. **Topic 1: gemini vs others on Storage trait.** If gemini can
   name a second impl committed in v0.0.1, I update. Otherwise,
   YAGNI holds and trait is deferred.

2. **Topic 4: trigger number.** 500 vs 1k vs 5k vs 10k corpus floor.
   This is operator domain knowledge; council should flag it as
   requiring operator input, not a council decision.

3. **Topic 4: stale-retrieval window.** 7 days (minimal-change) vs 14
   days (my proposal) vs the arch-reviewer's score-distribution. The
   right window depends on the operator's AE ingestion cadence. Flag
   for operator, not council.

---

## Open Questions

1. **Is the optional `valid_from` parameter on `memory_ingest` within
   v0.0.1 scope?** Arch-reviewer proposed it as the alternative to the
   bi-temporal column. It is one optional parameter on an existing MCP
   tool — minimal cost. But it must be explicitly included in the
   defect-fix BL scope or it will be silently dropped. Does it belong
   in the same BL as the two-ingest-paths fix?

2. **What is the domain audit table schema?** The stale-retrieval
   metric requires a join between search-call log (which facts were
   returned for which query) and fact validity timestamps (when each
   returned fact was superseded). Does the planned domain audit table
   include both: the returned-fact IDs per query AND the query
   timestamp? If not, the stale-retrieval trigger metric cannot be
   computed even with perfect future instrumentation.

3. **Corpus size floor for Topic 4: is 500 / 1k / 5k / 10k a
   council decision or an operator decision?** The 20× range in
   Round 1 proposals reflects genuine uncertainty about the operator's
   expected corpus growth rate. If the council picks 500 and the
   operator expects to stay at ~500 facts for 2 years, the trigger
   fires too early. If the council picks 10k and the corpus reaches
   1k, the trigger never fires for years. This should be explicitly
   flagged as "operator calibrates this threshold after 3 months of
   production."
