---
role: challenger
round: 2
date: 2026-04-20
---

# Round 2 — Challenger

## Peer citations used

- `architect.md:93–116` — demotion floor 0.10 with natural hysteresis argument
- `architect.md:137–149` — demotion-only first-ship safety argument (~93-day trigger)
- `archaeologist.md:50–73` — `is_longterm` boost confirmed; distribution stats
- `archaeologist.md:105–176` — age data: all 110 recalled memories within 15 days
- `codex-proxy.md:26–51` — formula rejection of `0.95^days`, 60-day half-life
- `codex-proxy.md:260–273` — 44-day trigger at floor 0.30
- `gemini-proxy.md:143–160` — dry-run defense for population-regression risk
- `synthesis.md` — OFC-1 and OFC-2 disposition confirmed

---

## Question 1 — Floor calibration (D1)

**Position: Reject both 0.10 and 0.30 as first-ship defaults. Use 0.20 as a fixed floor, explicitly documented as provisional.**

The percentile-based rule I proposed in Round 1 is impractical at BL-008 scope. A percentile query requires either (a) a full corpus scan at Dreaming time to compute P10 dynamically, or (b) a stored percentile refreshed separately — both exceed the ~50-100 LOC scope and the "no new stored state" hard constraint. I withdraw the percentile approach.

But neither 0.10 nor 0.30 survives the archaeologist's data.

**Against 0.30 (Codex)**:

With `H=60` and `avg≈0.48`, 0.30 triggers demotion at ~44 days (`codex-proxy.md:260–273`). The archaeologist shows the oldest `last_recalled` in the corpus is 15 days (`archaeologist.md:162–176`). That means the ENTIRE currently-recalled corpus was recalled within the past 15 days. The 44-day trigger looks safe now, but "safe now" is not a design argument — it is "the corpus is too young to have demoted anything yet." The moment the corpus ages past 44 days without fresh recalls (which will happen if the user takes a break, changes projects, or the ae:analyze pipeline goes quiet), the 0.30 floor will trigger a bulk demotion sweep. The 1.2x search boost will be stripped from memories that may still be relevant. There's no empirical signal justifying this aggressiveness against a tightly compressed distribution.

**Against 0.10 (Architect)**:

The architect's 93-day trigger is conservative, but the rationale contains a circular dependency. `architect.md:99–103` says "with H=60 days, effective ≈ 0.5 × avg after 60 days... an unreferenced 6-month-old memory *should* be considered marginal." That judgment is intuitive, not empirical. The corpus has 323 memories and is 15 days old. We don't yet know whether 6-month-old memories *should* be marginal in this specific workflow. The claim comes from general intuition about memory systems, not from observation of Mengdie's use pattern.

A 93-day demotion trigger also interacts badly with the `last_recalled` age input. Once 66% of memories have `last_recalled IS NULL` (`archaeologist.md:147`: 213 of 323 live memories), the fallback to `created_at` means those memories start decaying from birth. The oldest memories in the corpus were created 15 days ago (`archaeologist.md:154–176`). They would have `effective ≈ 0.48 × exp(-15×ln2/60) ≈ 0.48 × 0.84 ≈ 0.40`. With a 0.10 floor, that's still safe. But this is masking the real question: **what should happen to never-recalled memories?**

**Concrete proposal: floor = 0.20, explicit `last_recalled`-only decay (no `created_at` fallback)**

- Floor 0.20 with H=60: triggers at `avg × 2^(-d/60) < 0.20`, i.e., `2^(-d/60) < 0.42`, i.e., `d > 75 days`. That is conservative enough to survive a 2-month project hiatus without bulk demotion.
- **No `created_at` fallback for decay**. If `last_recalled IS NULL`, the memory has never surfaced in search — it has never been confirmed relevant. Apply decay from `created_at` is wrong because it penalizes never-tested memories on an arbitrary timeline. The correct behavior: memories with `last_recalled IS NULL` do NOT decay. They remain candidates for promotion (they can still accumulate `recall_count` and `avg_relevance`) but their effective relevance for DEMOTION purposes is not yet established. Demotion requires evidence of decline, not just absence of recall.
- This also fixes the scope creep in the NULL-fallback: eliminating the fallback simplifies the formula to a single code path.

**Residual risk I acknowledge**: 75 days is still based on `avg≈0.48`. If the distribution shifts (e.g., future improvements to the recall pipeline raise `avg_relevance` toward 0.65), the effective trigger moves to ~100 days. That's acceptable drift. If the distribution compresses further downward (hypothetically), re-examine the floor. The trigger condition for revisit is: "if `avg_effective_relevance` across the corpus drops below 0.25 on a Dreaming pass, recalibrate floor."

---

## Question 2 — Decay-only first ship (D2)

**Position: Concede — ship decay + demotion together. Narrow the original "observe first" argument to one specific precondition.**

Architect's case (`architect.md:137–149`) is well-constructed. The 93-day demotion trigger under H=60 + floor=0.10 (or 75 days under my proposed floor=0.20) means first-pass demotion on a 15-day-old corpus is zero. The "observe first" argument assumed first-pass demotion was a real risk. With accurate corpus age data (`archaeologist.md:154–176`: oldest memories are 15 days old), that risk is empirically zero for weeks.

However, one precondition must hold for this concession to be valid:

**The `demoted: usize` counter MUST be in `DreamingResult` at ship**. If demotion runs silently and the operator has no way to see how many memories were demoted, the "observe it working" rationale collapses. The architect proposes this (`architect.md:174–184`) and the Gemini proxy adds `decay_floor_breaches` as a second signal (`gemini-proxy.md:40–64`). Both should ship. Without them, shipping demotion is shipping an unobservable mutation on the production knowledge base.

If both counters are in scope for BL-008, "decay-only first" is premature caution. If the counters get deferred to a follow-up, demotion should also be deferred.

---

## Question 3 — F3 recall_count inflation: narrow scope or block?

**Position: (a) Narrow scope claims. Do not block.**

The evidence supports narrowing, not blocking.

Codex states it directly (`codex-proxy.md:275–279`): "decay is being asked to fix two problems at once, staleness and burst-biased averaging. It can handle staleness. It cannot fully correct a lifetime mean that overweights sessions."

BL-008's framing bullet 2 reads: "A memory recalled 10 times in week 1 and never again scores identically to a memory recalled 10 times over the past two weeks at the moment of the next dreaming pass." This is a claim BL-008 CANNOT fully satisfy because `avg_relevance` encodes lifetime signal, not recency signal. The time-decay multiplier softens this — an old burst memory does age out eventually — but it does not eliminate the bias.

The conclusion document for BL-008 must not claim to fix burst bias. It should state: "BL-008 addresses staleness by reducing `effective_relevance` for memories not recently recalled. It does not address recall_count inflation from intra-session burst hits; that is a separate pathology (prior-art §1) tracked separately."

Blocking would be wrong because session-dedup (BL-014 or equivalent) has no ship date and the staleness problem is real and independent. Blocking BL-008 until session-dedup lands couples two independent problems and delays a useful, bounded mechanism for no gain.

---

## Question 4 — Search-time decay + LONGTERM_BOOST interaction

**Position: Not double-counting. But the formula has an ordering dependency that is not clearly stated.**

Architect proposes: `final = normalized_rrf × decay_factor × longterm_boost_if_applicable` (`architect.md:70–82`).

The argument that this is correct: demotion and decay ARE different things. `is_longterm = 1` is a discrete state (the memory passed promotion thresholds); `decay_factor` is a continuous time function. They encode orthogonal information. A recently-promoted memory that was just recalled yesterday has `decay_factor ≈ 1.0` AND `is_longterm = 1` — it gets the full 1.2x boost appropriately. A promoted memory not recalled in 80 days has `decay_factor ≈ 0.42` AND `is_longterm = 1` until demotion fires — it gets `0.42 × 1.2 = 0.504` of its normalized score, which is less than an undecayed non-longterm memory at normalized=0.55. That is the correct ordering.

**But there is a non-obvious interaction the architect's formula does not address**: once demotion fires and `is_longterm` is cleared, the memory no longer receives the 1.2x boost. On the next search, it drops from `effective × 1.2` to `effective × 1.0` — a cliff at exactly the same Dreaming pass that demoted it. That cliff is a one-time discontinuity that IS intentional (the memory lost its long-term badge), but it should be named explicitly in the design so a future maintainer doesn't read it as a bug.

The formula itself is NOT double-counting: decay measures time-since-recall; is_longterm measures whether the memory ever earned promoted status. They are genuinely orthogonal. The concern I'd press: **the search-time decay must use the same age input as the Dreaming-pass decay** — otherwise a memory could appear boosted at search time (because search-time decay uses `last_recalled`) while simultaneously passing the Dreaming-pass demotion threshold (if Dreaming uses `created_at`). The formula is correct only if both computation sites use the SAME age clock. This is an implementation correctness requirement, not a design disagreement.

---

## Of-framing challenges (Round 2)

**OFC-3: The `created_at` NULL-fallback is a hidden design decision, not a minor implementation detail.**

Both architect (`architect.md:40–44`) and synthesis accept the `created_at` fallback for `last_recalled IS NULL` without flagging it as consequential. But 213 of 323 live memories (66%) have `last_recalled IS NULL` (`archaeologist.md:147`). That means the fallback is not an edge case — it applies to the MAJORITY of the corpus on first pass. Decaying 213 memories from their creation date is a significant behavior that has not been explicitly argued for. I'm calling this out as a design decision requiring explicit acceptance, not an implementation default to absorb silently.

---

## Open Questions (Round 2 residuals)

1. **`run_dreaming` callers that would break on signature change**: Archaeologist flagged this as a Round 2 research target (`synthesis.md`, UAG section). The callers need to be enumerated before the clock-injection approach is locked.

2. **Formula form lock**: `exp(-ln2·d/H)` vs `2^(-d/H)` — these are identical in output but differ in LOC. The architect's `exp(-d/H)` spelling (WITHOUT `ln2`) is wrong and was flagged by Codex (`codex-proxy.md:64–65`). This must be resolved to a single canonical spelling in the conclusion to prevent an implementation bug.

3. **Demotion cliff documentation**: The one-time score discontinuity when `is_longterm` is cleared should appear as a comment at the demotion site in `dreaming.rs` and at the boost site in `search.rs`. Not a blocker, but without it the behavior will appear as a bug to a future reader.
