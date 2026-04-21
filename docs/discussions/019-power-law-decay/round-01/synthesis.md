---
round: 01
date: 2026-04-20
author: team-lead
---

# Round 1 Synthesis

Orientation layer only. Authoritative positions are in the per-agent files
(`architect.md`, `archaeologist.md`, `challenger.md`, `codex-proxy.md`,
`gemini-proxy.md`). Claims below cite per-agent file + line numbers.
Agents: read the per-agent files directly; do not derive arguments from
this synthesis.

## Convergences (pre-consensus-verification)

1. **Formula family = half-life exponential on `last_recalled`**.
   - Architect recommends `max(floor, avg × exp(-d/H))`, `H=60`
     (`architect.md:13–15, 40–50`).
   - Codex independently recommends `avg × 2^(-d/H)`, `H=60`
     (`codex-proxy.md:13–15, 26–51`). Note form equivalence: `exp(-ln2·d/H) = 2^(-d/H)`.
   - Codex **explicitly flags a Round-2 risk** in the architect's spelling:
     `exp(-d/half_life)` (without `ln 2`) makes half-life `H·ln 2 ≈ 0.693·H`,
     not `H` (`codex-proxy.md:64–65, 180–182`). Convergence is on
     intent; Round 2 must lock the exact form.
   - Age input = `last_recalled` (architect explicit;
     codex implicit via sawtooth analysis `codex-proxy.md:96–99`).
2. **Store nothing new**. Compute from `avg_relevance` + `last_recalled`
   on the fly (architect + codex + framing's hard constraint).
3. **Clock injection via parameter, not trait**. Architect + Gemini both
   land on `now: DateTime<Utc>` parameter threaded into
   `run_dreaming_with_config` (`architect.md:155–168`,
   `gemini-proxy.md:14–32`). Archaeologist confirms no existing clock
   abstraction exists (`archaeologist.md:105–123`).
4. **Compute location = Dreaming pass + search re-rank, read-only**.
   Architect explicit (`architect.md:54–86`). Codex OK with search-path
   usage if done in Rust, not SQL (`codex-proxy.md:82–88, 227–231`).
   Archaeologist confirms `search.rs:142` already has a post-fetch
   re-rank site (`LONGTERM_BOOST`).

### UAG status (Unanimous Agreement Gate)
None of the above have passed UAG yet. Round 2 falsification questions:
- *Formula*: Find a memory pattern where `2^(-d/60)` is obviously wrong
  (e.g., a legitimate burst-then-silent memory that SHOULD stay
  promoted but will demote under this formula; or vice versa). If none,
  formula converges.
- *Clock injection*: Find a caller today that would break under an
  `Option<DateTime<Utc>>` signature change. Archaeologist: trace all
  `run_dreaming*` callers.

## Active disagreements (to resolve in Round 2)

### D1. Demotion floor value (most important unresolved number)
| Position | Value | Trigger | Evidence |
|---|---|---|---|
| Architect | 0.10 | `avg=0.48` demotes at ~93 days | `architect.md:93–103, 110–112` |
| Codex | 0.30 | `avg=0.5` demotes at ~44 days | `codex-proxy.md:117–118, 260–273` |
| Challenger | not a fixed constant | calibrate to distribution percentile | `challenger.md:171–178` |

Round 2 must decide. Recommend re-reading archaeologist's distribution
stats (`archaeologist.md:137–145`: 86% of recalled memories in
0.46–0.50, min 0.462, max 0.746) against both positions.

### D2. Ship demotion in BL-008, or decay-only first ship?
- Architect: ship both; demotion is ~10 LOC on top of decay
  (`architect.md:114–117, 137–141`).
- Challenger: ship decay-only, observe distribution, add demotion in a
  follow-up BL (`challenger.md:185–190`).
- Codex: asymmetric promotion + demotion (implicitly both-in)
  (`codex-proxy.md:100–113`).

### D3. `mengdie dream --dry-run-decay` flag
- Architect: scope creep, reject (`architect.md:191–194`).
- Gemini: critical for validating population-level mass-demotion risk
  BEFORE mutating DB (`gemini-proxy.md:143–160`).
- Tie-breaker: depends on D2. If decay-only first ship (Challenger), a
  dry-run isn't needed because there's no mutation. If demotion is in
  scope (Architect/Codex), a pre-mutation dry-run is the safety net
  Gemini describes.

### D4. Observability counter set
- Architect: `demoted`, `avg_effective_relevance` (2 new fields on
  `DreamingResult`) (`architect.md:174–184`).
- Gemini: `demoted_count`, `avg_effective_score_before`,
  `decay_floor_breaches` (3 new fields) (`gemini-proxy.md:40–64`).
- Reconciliation question: is `avg_effective_relevance` (architect)
  the same as `avg_effective_score_before` (gemini)? Probably yes.
  Is `decay_floor_breaches` distinct from `demoted`? Only if "breach"
  is computed but demotion is guarded (dry-run, promotion-pass-only,
  etc.) — link to D2/D3 resolution.

### D5. Naming — "power-law" vs "exponential" decay
- Challenger: the formula is exponential, not power-law. Title is
  wrong in discussion ID, topic title, BL-008
  (`challenger.md:97–118`). No other agent commented.
- Low cost to rename. Worth resolving so future-self doesn't cite this
  discussion under a false label.

## Of-framing disposition (mandatory)

| # | Challenge | Raised by | TL disposition | Rationale |
|---|---|---|---|---|
| OFC-1 | Prior-art §3 memory is stale — `is_longterm` IS read by search | challenger (`challenger.md:141–150`) | **INTEGRATE — acted this round** | Stale memory invalidated via `memory_invalidate` with `superseded_by` pointing to a corrected memory. Evidence: `search.rs:9, 142-146`. |
| OFC-2 | "Forgetting matters" non-question — framing forecloses the threshold calibration question | challenger (`challenger.md:152–166`) | **INTEGRATE — reopened D1 as design question** | Framing's non-question #1 stands for "whether to build forgetting at all," but the demotion threshold has load-bearing user-visible effect (removes 1.2× boost). Threshold is a design question, not a tuning knob. D1 now tracks this. |

Frame-challenges from earlier rounds: N/A (Round 1 is first round).

## Verification artifacts (mandatory)

All claims of "verified / computed / checked" in this synthesis cite
concrete artifacts:

| Claim | Artifact |
|---|---|
| `is_longterm` read at search.rs:142 with 1.2× boost | `src/core/search.rs:9, 142–146`; commit `b59fbe0` dated 2026-04-05 (verified by archaeologist, challenger) |
| 86% of `avg_relevance` in 0.46–0.50 | live query on `~/.mengdie/db.sqlite` (`archaeologist.md:137–145`) |
| `0.5 × 0.95^90 = 0.005` | Codex computation (`codex-proxy.md:166–171`) |
| `0.5 × 2^(-90/60) ≈ 0.177` | Codex computation (`codex-proxy.md:34–38`) |
| ~93-day demotion trigger at floor=0.10, H=60 | Architect computation (`architect.md:99–103`) |
| ~44-day demotion trigger at floor=0.30, H=60 | Codex computation (`codex-proxy.md:266–273`) |
| Clock abstraction absent | `archaeologist.md:105–123` (enumerated callsites) |

Unvalidated (held out as Round 2 research targets):
- Gemini's test cases still use the rejected `0.95^days` numbers
  (`gemini-proxy.md:83–106`). Regenerating numbers under
  `2^(-d/60)` is Round 2 work, not a claim.

## Frame-challenge disappearance self-check (mandatory)

Round 0 produced 1 framing concern (the `avg_relevance MUST NOT be
mutated` hard constraint, flagged as the one solution-class closure
but defensibly pre-settled). Regex/keyword scan of Round 1 per-agent
files confirms: no agent proposed mutating `avg_relevance`; constraint
held. Constraint carried through; nothing silently dropped.

Round 1 produced 2 OFCs (tracked above). Neither silently dropped —
both explicitly dispositioned this round.

## Pruned

Pruned: **nothing** — all 5 agents' Round 1 inputs advanced to Round 2.
No claims were rejected as off-topic, redundant, or unevidenced.

- Gemini's test pseudocode was produced against the rejected formula
  but is not "pruned" — regenerated cases under the convergent formula
  are a Round 2 task.

## Round 2 agenda

Topic priority order for Round 2 (sub-questions resolved inside the team;
only genuine dilemmas reach the user):

1. **D5 first** (5-min decision, unblocks naming everywhere else): rename
   to "exponential decay" or "time-weighted decay". Architect + codex +
   gemini should weigh in with one line each; challenger owns the claim.
2. **D1 floor value** — decide between 0.10 and 0.30 (or a distribution-
   derived percentile). Requires joint architect + codex + challenger
   position, with archaeologist data.
3. **D2 ship scope** — decay-only first, or decay+demotion. Tight
   interaction with D1 and D3.
4. **D3 dry-run** — follows from D2 outcome.
5. **D4 counter set** — consolidate to a single agreed set; minor.
6. **Formula form lock** — `exp(-ln2·d/H)` vs `2^(-d/H)` (identity in
   output, differs in LOC/readability). Codex's caveat must be resolved.

No user escalation from Round 1.
