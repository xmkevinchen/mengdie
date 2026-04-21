---
id: "02"
title: "Computation location — eager (Dreaming pass) vs lazy (search time) vs hybrid"
status: converged
current_round: 2
created: 2026-04-20
decision: "Hybrid: compute in Rust (not SQL) at two sites — Dreaming pass (demotion gate) AND search post-fetch re-rank multiplier. Stored avg_relevance never mutated. Both sites MUST use the same age clock (last_recalled)."
rationale: "search.rs:142 already has a post-fetch re-rank site (LONGTERM_BOOST); decay fits same pattern with one multiplier line. Dreaming-pass demotion is O(demoted count), not O(corpus). Codex explicitly rejected SQL pow() for portability; computing in Rust avoids SQLite math-function dependency. Same-age-clock invariant is a correctness requirement (challenger Q4), not a preference — otherwise search ranking and demotion disagree."
reversibility: "high"
reversibility_basis: "Each compute site is independent; either can be disabled or relocated without schema change. No stored state."
---

# Topic: Computation location

## Current Status
Converged at Round 2. Hybrid compute (Dreaming + search post-fetch) with same-age-clock invariant.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | explore | Architect proposed hybrid split; codex/others OK if computed in Rust |
| 2 | converged | Same-age-clock invariant added (challenger correctness check) |

## Context
`effective_relevance` could be computed at (a) the daily Dreaming pass
— batch, affects promotion/demotion only; (b) at query time in
`search.rs` — per-row SQL or post-fetch; (c) at ingest-time
contradiction-check; (d) some combination. The choice affects where
decay's behavior is visible to end users and how much write/read cost
is incurred.

## Constraints
- BL-008 literally says "at promotion/demotion time" — leaves open
  whether search time uses it.
- Stored `avg_relevance` must NOT be mutated (hard constraint from BL-008).
- SQLite single-writer model: writes must not fan out across corpus on
  every search call.
- Existing prior finding: `is_longterm` is not currently read by search
  (prior-art §3). If decay is Dreaming-internal only, its effect is
  confined to promotion/demotion ledger.

## Key Questions
1. Do we compute at Dreaming-pass only (simplest, demotion-only surface)
   or also at search time (exposes decay to ranking)?
2. If search-time, is the computation a SQL expression on
   `memory_entries` or a post-fetch pass in Rust?
3. If we expose decayed score to search, does it replace `avg_relevance`
   or layer on top (e.g., RRF input adjustment)?
4. What's the write-amplification cost of each option under the
   `Arc<Mutex<Connection>>` model?
