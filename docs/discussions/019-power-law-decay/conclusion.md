---
id: "019"
title: "Exponential Decay for Dreaming (BL-008) — Conclusion"
concluded: 2026-04-20
plan: ""
entities: [decay, exponential, exponential-decay, dreaming, demotion, promotion, long-term-memory, is-longterm, bl-008, last-recalled, half-life, avg-relevance, longterm-boost, dreaming-result, dry-run, clock-injection, observability]
---

# Exponential Decay for Dreaming (BL-008) — Conclusion

**Status**: all 5 topics converged in 2 rounds. Zero deferred, zero
revisit, zero user escalations. No Sweep required.

**Scope recap**: BL-008 adds a forgetting mechanism to the Dreaming pass —
an effective-relevance computation that decays with time since the last
recall, and a demotion path that clears `is_longterm` for memories whose
effective relevance falls below a floor. Stored `avg_relevance` is never
mutated. The decay computation also feeds search-time ranking as a
post-fetch re-rank multiplier.

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Decay formula & constants | `effective = avg_relevance × 2^(-d/H)` with `H = 60` days. Implement in Rust as `(2.0_f64).powf(-days / 60.0)`. Age input = `last_recalled` only; memories with `last_recalled IS NULL` are excluded from the decay path entirely. Floor = **0.20** (demotion trigger). Discussion and BL-008 renamed from "power-law" to "exponential decay" (the formula was never a power-law). | Architect + codex independently picked H=60; codex flagged `exp(-d/H)` ≠ half-life semantics, adopt `2^(-d/H)` form. Floor=0.20 from architect+challenger independent convergence (77-day trigger, observable within a quarter, conservative against 2-month hiatus). Codex's 0.10 rested on an arithmetic slip (claimed 96-day trigger; actual 137 days); corrected rationale collapses to 0.20. Archaeologist V2 (IQR=0.015) killed percentile-based floor — the distribution is too tight to distinguish tiers. | Medium — formula is a pure function, half-life and floor are single constants. Revisit triggers: `avg_effective_relevance` <0.25 OR corpus age >90 days OR IQR widens past 0.05. |
| 2 | Computation location | Compute in Rust (not SQL) at **two sites**: (a) Dreaming pass demotion gate, and (b) search-time post-fetch re-rank multiplier. Stored `avg_relevance` is never mutated — decay is always derived on the fly from `(avg_relevance, last_recalled, now)`. Both sites MUST use the SAME age clock (`last_recalled`); otherwise search ranking and demotion disagree. | `search.rs:142` already has a post-fetch re-rank site (`LONGTERM_BOOST=1.2`); decay fits the same pattern with one multiplier line. Dreaming-pass demotion cost is O(demoted), not O(corpus). Codex rejected SQL `pow()` for portability; computing in Rust avoids SQLite math-function dependency. Same-age-clock is a correctness requirement (challenger Q4), not a preference. | High — each compute site is independent; no schema change, no stored state. |
| 3 | Demotion semantics & threshold | Asymmetric rule (demotion ≠ inverted promotion). Demote when `is_longterm = 1 AND last_recalled IS NOT NULL AND effective_relevance < 0.20`. Clears `is_longterm` flag; no new `was_longterm` state (that would require migration, outside BL-008 scope). Memories with `last_recalled IS NULL` are skipped — no decay, no demotion — because their staleness signal doesn't exist. Natural hysteresis: promotion requires `last_recalled` within 14 days; demotion triggers at ~77 days of silence → structural gap of 63 days prevents flapping. | Archaeologist V3: zero demotions on first pass under any floor (0.10, 0.20, 0.30) — the design is empirically safe to ship. The 1/41 long-term memory with NULL `last_recalled` (2.4%, probably a synthesis row or test artifact) gets the skip rule. Clearing `is_longterm` is the minimum change with user-visible effect (removes 1.2× LONGTERM_BOOST at `search.rs:142`). | Medium — threshold is a single constant. Full reversal to add `was_longterm` state requires a migration — deferred to BL if needed. |
| 4 | Interaction with existing promotion | Promotion predicate is **UNCHANGED**: still reads raw `avg_relevance >= 0.45`, still keeps the `recall_count >= 3` gate, still keeps the `last_recalled within 14d` gate. Decay/demotion are the only new behavior. BL-008's scope claim is NARROWED to staleness only — the conclusion does NOT claim to fix the `recall_count` burst-inflation pathology (prior-art §1; that's a separate session-dedup concern). | Codex computation: a 15-day-old memory with `avg=0.50` has `effective = 0.50 × 2^(-15/60) ≈ 0.420` — would fail an `effective_relevance >= 0.45` promotion gate. Switching promotion to `effective` would immediately mass-demote the 41 current long-term memories. Keeping promotion on raw `avg_relevance` makes BL-008 a single-surface addition. Challenger + codex agree decay cannot fix a lifetime-mean bias; narrowing scope is honest and doesn't block orthogonal session-dedup work. | High — zero code change to the promotion path; only additive change in the demotion path. Fully reversible by skipping the demotion code path. |
| 5 | Observability & testing strategy | Clock injection: add `now: Option<DateTime<Utc>>` parameter to `run_dreaming_with_config`; when `None`, default to `chrono::Utc::now()`. `DreamingResult` gains **4 new fields**: `demoted: usize`, `avg_effective_score_before: f64`, `avg_effective_score_after: f64`, `decay_floor_breaches: usize`. Ship `mengdie dream --dry-run-decay` CLI flag (writes nothing; counts breaches; reports distribution). Table-driven regression tests under `2^(-d/60)` with `avg=0.487` at `d=1, 15, 44, 75, 137` days (covers the no-decay, small-decay, codex-trigger, converged-trigger, architect-trigger points). Code comments at BOTH the demotion site (`dreaming.rs`) and the boost site (`search.rs:142`) documenting the one-time LONGTERM_BOOST cliff when `is_longterm` clears. | Gemini R1's `Option<>` form is cleaner than architect's required param — single-line default. Archaeologist V1: signature change is contained (2 production callers, 3 trivial edits, zero viral spread). The 4-counter set closes a baseline-event-mechanism-delta loop: `avg_effective_score_before` watches distribution drift; `demoted` watches mutation volume; `avg_effective_score_after` confirms the pass actually shifted what it claimed (Doodlestein strategic addition); `decay_floor_breaches` watches would-be demotions in dry-run (distinct from `demoted` only when writes suppressed). Dry-run flag is load-bearing because first real demotion arrives ~77 days post-ship under floor=0.20 — operators need a pre-mutation validation path in the interim. Cliff-comment requirement came from challenger Q4 correctness review. | High — counters are additive fields; dry-run flag is isolated. Clock-injection signature revert is ≤3 line edits. |

## Doodlestein Review

All 3 post-conclusion reviews complete. None triggered a new round. Full
per-agent findings in `round-doodlestein/{strategic,adversarial,regret}.md`.

### Strategic (PASS with 2-LOC amendment)

**Finding**: Topic 5's 3-counter set omits `avg_effective_score_after`.
Without the after value, operators see `demoted: N` but no distribution
delta to confirm the mechanism shifted what it claimed to shift.

**Disposition**: **AMENDMENT ACCEPTED**. `DreamingResult` now gains
**4 new fields** (not 3):
- `demoted: usize`
- `avg_effective_score_before: f64`
- `avg_effective_score_after: f64`  ← added per Doodlestein strategic
- `decay_floor_breaches: usize`

Topic 5 decision is updated accordingly. Cost: 2 LOC, no new behavior,
fully within scope.

### Adversarial (no reopen; plan-level guard added)

**Finding**: The "zero demotions on first pass" empirical safety claim
(archaeologist V3) is load-bearing on the snapshot date. Archaeologist's
R2 data already confirmed all 40 long-term memories with non-NULL
`last_recalled` are within 15 days and min effective = 0.397 (well above
any proposed floor), but the conclusion did not flag what happens if the
corpus drifts between conclusion date (2026-04-20) and ship date.
Concrete risk: first `mengdie dream` post-ship, without a prior
`--dry-run-decay`, could silently demote memories whose timestamps drifted.

**Disposition**: **PLAN-LEVEL GUARD**. Not a design reopen. Add to
BL-008's acceptance criteria during `/ae:plan`:
1. First live `mengdie dream` post-ship MUST be preceded by a
   `mengdie dream --dry-run-decay` run, with `decay_floor_breaches`
   inspected before enabling writes. Document this as an operator
   procedure in the plan's "Acceptance / Rollout" section.
2. `DreamingResult.demoted` MUST surface visibly in the `mengdie dream`
   CLI output (not silently in the struct). Spec the exact output format
   in the plan — e.g., `Dreaming pass: N promoted, M demoted
   (K floor breaches, avg effective 0.XX → 0.XX)`.

### Regret (accepted fragility; revisit trigger already logged)

**Finding**: Topic 1's floor = 0.20 is the decision most likely to be
reversed in 6 months. The assumption most likely to fail: IQR stays at
~0.015. Two drift forces: (a) synthesis memories consolidate high-signal
content and score higher than raw ingest, creating bimodality; (b) the
`recall_count` burst-inflation pathology inflates `avg_relevance` for
burst-hit memories, adding a high-end tail. A wider IQR moves the
effective trigger — a memory at `avg=0.22` crosses floor=0.20 after only
~11 days of silence, which could demote recently-ingested briefly-quiet
memories.

**Disposition**: **ACCEPTED FRAGILITY**. Revisit triggers are already
logged in Topic 1's summary (`reversibility_basis`): reassess when
`avg_effective_relevance` < 0.25 OR corpus age > 90 days OR IQR widens
past 0.05. Reversal shape explicitly noted as a candidate: lower floor
to 0.15 OR switch to a corpus-relative rule like `mean - 1.5 × IQR`.
The floor is a single pure-function constant; reversal costs one-line
edit + regression-table re-run. Pre-emptive defense: `--dry-run-decay`
already on the ship list (Topic 5) provides early-warning signal through
`decay_floor_breaches` before the first real demotion arrives at ~77
days.

### Net result of Doodlestein

- 0 decisions reopened.
- 1 scope addition: 4th counter `avg_effective_score_after` (2 LOC).
- 2 plan-level guards: `--dry-run-decay` precondition + CLI output spec
  for `demoted` — both will be captured in `/ae:plan` acceptance
  criteria, not this conclusion.
- 1 accepted fragility: floor=0.20 tuning value; revisit triggers
  already in place.

## Spawned Discussions

None. No topic required splitting off into a separate deep-dive.

## Deferred Resolutions

None. All topics converged; no Sweep needed.

## Team Composition

| Agent | Role | Backend | Joined | Note |
|-------|------|---------|--------|------|
| host (team-lead) | TL / moderator | Claude | Start | — |
| architect | System design | Claude | Start | Core design owner |
| archaeologist | Code evidence | Claude | Start | Delivered 3 empirical verifications in R2 |
| challenger | Adversarial scrutiny | Claude | Start | Raised 3 OFCs (OFC-1/2/3), all integrated |
| codex-proxy | Cross-family (OpenAI) | Codex — medium effort | Start | Independent formula/correctness lens; revised R2 given data |
| gemini-proxy | Cross-family (Google) | Gemini → local gemma4 R1 silent-fallback → Gemini R2 auth error | Start | R1 observability positions all adopted; R2 protocol correctly followed (report + stop) |

Note on gemini-proxy: R1 silently fell back to local gemma4 without
TL notification (protocol violation), but R1 content was substantive
and integrated. R2 correctly reported API auth error and stopped. TL
decision: do not reroute — R1 observability positions were fully
integrated in architect R2, and the only outstanding R2 item
(regenerated test cases under new formula) is plan-work.

## Process Metadata

- Discussion rounds: 2 (Round 0 framing APPROVED; Round 1 independent
  research; Round 2 share/explore/converge)
- Topics: 5 total — 5 converged, 0 spawned, 0 explained-with-assumption,
  0 escalated
- Autonomous TL decisions: 1 (D1 floor value, via evidence preponderance
  at 0.20 with codex math error documented; no user escalation because
  disagreement was narrow and codex's own rationale collapses to 0.20
  with corrected math)
- User escalations: 0
- Of-framing challenges (OFCs): 3 raised (OFC-1/2/3), 3 integrated (0
  rejected, 0 deferred)
- Prior art retrieved from Mengdie: 5 items (1 corrected mid-discussion
  via `memory_invalidate`)
- Doodlestein challenges: 3 raised (1 strategic / 1 adversarial / 1 regret), 1 amendment accepted (`avg_effective_score_after` counter), 2 plan-level guards logged (dry-run precondition, CLI output spec), 1 accepted-fragility (floor=0.20). 0 rounds reopened.
- Deferred resolved in Sweep: 0 (no deferred)

## Knowledge Updates to Mengdie

Action taken DURING the discussion (Round 1 → Round 2 handoff):

1. **Invalidated** memory `2c3122ff-748e-412c-bee9-d54b6cdbabbe`
   ("is_longterm flag has zero effect on search — Dreaming subsystem is
   disconnected from retrieval") — factually wrong at origin, caused by
   KB ingesting analysis written one day BEFORE commit `b59fbe0` wired
   the boost. Independently caught by archaeologist and challenger in
   Round 1 (OFC-1).

2. **Ingested** correction memory `e2b0bb63-8776-4c9b-8c02-ce7f6412940e`
   ("is_longterm IS read by search.rs — applies 1.2× LONGTERM_BOOST
   post-RRF-normalization"), with `resolves=[2c3122ff]` atomic transaction.

Additional knowledge capture (Step 8.5) follows this conclusion.

## Open Tails (not blockers)

These items are tracked for later but do not hold up `/ae:plan`:

1. **`recall_count` burst inflation** (prior-art §1, unresolved). Decay
   softens staleness but does not fix the lifetime-mean bias from
   intra-session burst recalls. Tracked as a separate backlog item (likely
   a session-dedup BL). Challenger + codex both agreed in R2.
2. **1 long-term memory with `last_recalled IS NULL`** (archaeologist open
   Q1). Likely a synthesis row or test artifact. Under the converged
   design this memory is skipped from decay entirely. Investigate when
   corpus hygiene surfaces; not a correctness issue.
3. **Half-life tuning**. `H = 60` is a first-ship estimate. Revisit triggers
   (logged in topic-01 summary `reversibility_basis`): `avg_effective_relevance`
   < 0.25, corpus age > 90 days, or IQR widens past 0.05.
4. **Percentile-based floor** was withdrawn for this ship. Revisit when
   distribution widens (if corpus aggregates past 90–180 days with real
   tail).

## Next Steps

1. Run Step 8.5 knowledge capture (automatic).
2. Step 9 post-conclusion Doodlestein — 3 fresh agents review THIS
   conclusion document.
3. If Doodlestein raises a valid finding requiring response → new round
   on specific item; update conclusion; re-run Doodlestein on revised
   version.
4. After Doodlestein settles: **ready for `/ae:plan`** against BL-008.
   Suggested plan skeleton in `docs/backlog/005-phase2-roadmap.md`
   already notes this as the "most-ready next unit of work (no
   dependency gate, ~50–100 LOC)". The ~40 LOC for `--dry-run-decay`
   pushes this closer to the top of the budget but still inside it if
   the decay formula is extracted as a pure function (required for
   testability anyway).
