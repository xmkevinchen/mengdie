---
id: "05"
title: "Observability & testing strategy"
status: converged
current_round: 2
created: 2026-04-20
decision: "Clock injection: add `now: Option<DateTime<Utc>>` param to run_dreaming_with_config (defaults to Utc::now() when None). DreamingResult gains 4 new fields: `demoted: usize`, `avg_effective_score_before: f64`, `avg_effective_score_after: f64`, `decay_floor_breaches: usize`. Ship `mengdie dream --dry-run-decay` CLI flag (~40 LOC). Table-driven regression tests under 2^(-d/60) with d=1/15/44/75/137 and avg=0.487. Code comments at demotion site (dreaming.rs) AND boost site (search.rs:142) documenting the one-time score cliff when is_longterm clears."
rationale: "Gemini R1's Option<> clock form is cleaner than architect's required-param (one-line default in Rust). Archaeologist V1: signature change is contained — only 2 production callers, 3 trivial edits. 3-counter set closes baseline-distribution-mechanism loop (per Gemini R1). decay_floor_breaches is distinct from demoted in dry-run scope (writes suppressed, breaches still counted). Dry-run flag is load-bearing because first real demotion is ~75 days post-ship under floor=0.20 — operators need pre-mutation validation path. Cliff-comment requirement came from challenger Q4 correctness review."
reversibility: "high"
reversibility_basis: "Counters are purely additive fields; can be added/removed freely. `--dry-run-decay` is an isolated CLI flag. Clock-injection signature change is trivial to revert (≤3 sites)."
---

# Topic: Observability & testing strategy

## Current Status
Converged at Round 2. Clock param, 3 counters, dry-run flag, code-comment doc requirement.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | explore | Architect: required now param, 2 counters, reject dry-run. Gemini: Option<>, 3 counters, accept dry-run. |
| 2 | converged | Architect reversed: accepts Option<>, accepts all 3 counters, accepts dry-run flag. Gemini R2 unavailable (API auth error); R1 positions all integrated. |

## Context
BL-008 ships "forgetting" before there's user-visible evidence it's
needed (corpus is still young). So the shipped feature is its own
observation instrument — we need to be able to tell, at inspection time,
whether decay is over-aggressive, under-aggressive, or well-calibrated.
Testing time-dependent logic is a known trap (frozen clock vs real
clock, timezone drift, daily-pass edge cases).

## Constraints
- `tracing` → stderr is the logging channel (never stdout). Metrics
  table (`src/core/metrics.rs`) already exists as a place for counters.
- Tests use `chrono::Utc::now()` today — whatever strategy we pick must
  allow deterministic time for assertions without bringing in a new
  crate.
- `DreamingResult` struct is a public surface — any additions are
  semi-visible API changes (CLI output, downstream consumers).

## Key Questions
1. What new counters/fields on `DreamingResult` or `metrics.rs` would
   let an operator tell whether decay is behaving as intended? (e.g.,
   demoted count, average effective score, distribution buckets)
2. How do we write deterministic tests for a time-dependent formula
   without a heavy mock-clock dependency? Inject a `now: DateTime<Utc>`
   parameter? Use a `TimeProvider` trait?
3. What constitutes the "empirical signal of over-aggressive decay" vs
   "under-aggressive" that would appear in metrics? Is there a baseline
   to compare against before/after shipping?
4. Should we ship a `mengdie dream --dry-run-decay` mode that reports
   what *would* happen without mutating state? Or is that scope creep?
