---
role: challenger
round: 1
date: 2026-04-20
---

# Round 1 — Challenger

## Findings (with file:line evidence)

### F1 — Prior-art §3 is factually wrong: `is_longterm` IS read by search

Prior-art.md §3 states `is_longterm` is "never read by `search.rs` or `mcp_tools.rs`."
This is false. `src/core/search.rs:9` defines `LONGTERM_BOOST = 1.2` and
`src/core/search.rs:142–146` applies it:

```rust
let boosted = if entry.is_longterm {
    (normalized * LONGTERM_BOOST).min(1.0)
} else {
    normalized
};
```

This was wired on 2026-04-05 (commit `b59fbe0`, "Fix 3 issues from analyze sessions:
… Wire is_longterm into search as 1.2x score boost"). The prior-art memory claiming
the flag is unwired was dated 2026-04-06 — one day AFTER the fix landed. The memory
was written with stale data at origin. The KB has been wrong about this since it was
ingested.

**Impact on BL-008:** The entire "demotion is a no-op from the user's perspective"
attack surface (attack surface #5) is unfounded. Demotion from `is_longterm = 1`
to `is_longterm = 0` DOES affect user-facing search ranking — it removes the 1.2x
boost. This makes demotion higher-stakes than BL-008's framing implies: every
wrongly-demoted memory loses its search boost, silently degrading retrieval quality.
The risk is now asymmetric — over-aggressive decay actively harms recall, not just
wastes a bit.

---

### F2 — Distribution assumption makes `effective < 0.01` a near-certain mass-demotion trigger

BL-008 sketches demotion at `effective < 0.01`. Prior-art §2 establishes that
`avg_relevance` clusters near 0.5 (not uniform on [0,1]). The topic-01 summary
acknowledges this constraint but doesn't compute the consequence.

Math: `effective = 0.5 × 0.95^days`. Solving for `effective < 0.01`:
`0.95^days < 0.02` → `days > ln(0.02)/ln(0.95) ≈ 76` days.

Any memory not recalled in ~76 days would be demoted if `last_recalled` drives
decay, or demoted at ~76 days post-creation if `created_at` drives it. The
production corpus of 238 memories includes memories from project start (early
March 2026, ~50 days ago as of 2026-04-20). On a `created_at`-based formula,
~50-day-old memories would already be at `0.5 × 0.95^50 ≈ 0.022` — close to
the floor but not yet demoted.

But the framing says "ship before it's needed." If the corpus grows for 2–3
months without recalibration, the `0.01` floor could trigger bulk demotion on
the first pass — removing the 1.2x search boost from memories that may still
be relevant. The formula assumes values start near 1.0; they start near 0.5.
The threshold needs to be calibrated for the actual distribution, not the
theoretical maximum.

---

### F3 — `recall_count` inflation (prior-art §1) is a precondition BL-008 ignores, and it makes decay toothless for burst-recalled memories

Prior-art §1 documents that `record_recall` increments `recall_count` on every
search hit with no session deduplication — a single ae:analyze session can add
10+ counts to a memory. This is unresolved (see `docs/discussions/009-dreaming-promotion/analysis.md`).

BL-008's stated goal (framing §problem bullet 2): "A memory recalled 10 times
in week 1 and never again scores identically to a memory recalled 10 times
over the past two weeks." The proposed fix is time-decay. But if those 10
"recalls" in week 1 were actually 1 session × 10 search hits, the memory has
an inflated `recall_count` of 10 (or higher across sessions) AND an
`avg_relevance` anchored near 0.5. The decay formula operates on `avg_relevance`,
not `recall_count`. A time-decay multiplier on `avg_relevance` does not fix the
original stated problem — it adds a time penalty on top of an inflated signal.

The toothlessness depends on the `age_input` choice (topic-01 key question #2):
- If age is driven by `last_recalled`: one incidental recall resets the clock.
  Burst-recalled-then-abandoned memory gets its clock reset whenever it happens
  to appear in a search, even once. Decay is toothless for anything occasionally
  touched.
- If age is driven by `created_at`: burst memories from the project start are
  aged out at the same pace as never-recalled memories. That correctly ages out
  the burst-early-then-abandoned case, but also ages out a memory that was
  genuinely relevant at creation and never needed subsequent recall (stable facts).

Neither option cleanly fixes the stated problem because `recall_count` inflation
is the real pathology. BL-008 should name this dependency explicitly or narrow
its claimed fix.

---

### F4 — Exponential is not a power-law; naming is incorrect and matters for correctness claims

Topic-01 summary calls this "Power-Law Decay" in the discussion ID, title, and
framing. `0.95^days` is an exponential decay function. A power-law would be
`c / days^k` or `c × (1 + days/τ)^(-k)`. These have fundamentally different
tail behavior:

- Exponential: falls below any threshold in finite time for ANY non-recalled memory.
- Power-law: has a fat tail — decay slows as time increases; very old memories
  asymptotically approach zero but never reach it as fast.

For a forgetting mechanism meant to demote stale memories, the distinction matters:
exponential decay will eventually zero-out every memory that isn't recalled; a
true power-law preserves them at a slow-decaying residual. Given that the corpus
is small and demotion removes the 1.2x search boost, exponential is arguably more
aggressive than intended.

The naming is wrong in the discussion ID (019), in BL-008's title, and in the
topic title. If the design proceeds with an exponential formula — which may be
the right choice — it should be named "exponential decay" or "time-weighted
decay." Citing this discussion as "power-law decay" in future documents (reviews,
plans) will confuse anyone who looks up what power-law means.

---

### F5 — `DreamingResult` does not expose any decay or demotion counters; observability is currently zero

`src/core/dreaming.rs:40–46` shows `DreamingResult` has three fields: `promoted`,
`candidates_not_promoted`, `total_eligible`. There is no field for demoted count,
effective score distribution, or decay-floor triggers.

BL-008 ships forgetting "before there's user-visible evidence it's needed." If
observability is not added simultaneously, the operator has no way to know
whether decay is over-aggressive, under-aggressive, or malfunctioning — the
framing (key question #3) acknowledges this but the topic-05 summary marks it
as pending without a concrete baseline proposal.

Without at minimum a `demoted: usize` counter on `DreamingResult`, BL-008 is
unverifiable on the production corpus after ship. This isn't a soft concern — it
is a requirement for the stated "ship before it's needed, observe it working"
rationale to hold.

---

## Of-framing Challenges

**OFC-1: The prior-art memory that anchors attack surface #5 is factually wrong
and should be corrected before Round 2 locks conclusions.**

Prior-art §3 claims `is_longterm` is "never read by `search.rs`." This is false
as of commit `b59fbe0` (2026-04-05). The KB memory is stale at origin (created
2026-04-06, after the fix). Any Round 2 conclusion that uses this premise
(e.g., "demotion is a no-op for users") would be built on a false fact.
The memory should be invalidated in Mengdie before Round 2 begins.

**OFC-2: Framing marks "whether forgetting matters" as a non-question, but the
corpus size and absence of user-visible staleness makes premature optimization
a legitimate concern right now, not a closed debate.**

The framing explicitly says "not seeking debate on whether forgetting matters."
But the 238-memory corpus is <3 months old. No user-visible staleness problem
has been reported. The stated rationale is "ship before it's needed." That's a
reasonable engineering instinct, but it creates a real cost: adding exponential
decay to a corpus where `avg_relevance` clusters near 0.5 (F2 above) means the
first real Dreaming pass after ~76 days WILL demote a cohort of memories, and
those memories will lose their 1.2x search boost. If the framing is that we are
pre-shipping infrastructure, the demotion threshold should be set conservatively
(or demotion should be deferred) — and that IS a design question, not a tuning
knob. The non-question categorization is doing work here that hasn't been earned.

---

## Open Questions

1. **Threshold calibration given actual distribution**: Given `avg_relevance ≈ 0.5`
   and exponential decay, what demotion floor makes the first Dreaming pass
   after ship safe? `0.01` is probably too low to be meaningful as a demotion
   gate (everything will cross it eventually regardless of relevance). A floor
   calibrated to the actual distribution — e.g., "demote if effective drops
   more than 50% below the 10th-percentile observed `avg_relevance`" — would
   survive distribution shifts.

2. **Age input choice and the flapping surface**: If `last_recalled` resets
   decay, what prevents a memory from being recalled once (legitimately or
   incidentally) every 75 days indefinitely, permanently avoiding demotion?
   Is that acceptable behavior, or does it need an explicit floor on minimum
   decay regardless of recency?

3. **Is demotion actually needed for BL-008 to ship value?** Decay without
   demotion (promotion still uses `avg_relevance`, but the Dreaming pass just
   records `effective_relevance` in the result struct without acting on it)
   would let the team observe the decay distribution before committing to
   thresholds. That's a strictly smaller scope and doesn't foreclose demotion
   in a follow-up plan.

4. **Stale KB memory**: Who invalidates the `is_longterm` prior-art memory
   (prior-art §3) before Round 2? It is actively misleading and should not
   be allowed to anchor conclusions.
