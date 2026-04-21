---
id: "019"
title: "Power-Law Decay — Framing"
round_0: approved
round_0_date: 2026-04-20
---

# Power-Law Decay — Framing

## Problem

The current Dreaming promotion pass (`src/core/dreaming.rs`, promotion thresholds
`recall_count ≥ 3`, `avg_relevance ≥ 0.45`, `last_recalled` within `window_days = 14`)
has no counterpart for forgetting. Once a memory is promoted to long-term
(`is_longterm = 1`), nothing demotes it. Once a memory's `avg_relevance` is high,
nothing reduces its weight over time even if the memory is never recalled again.
The system has a monotonic "remembering" ratchet with no "forgetting" mechanism,
so two failure modes compound as the corpus grows:

1. **Stale long-term memories.** A memory promoted early in a project's life keeps
   its long-term badge and high `avg_relevance` indefinitely, even after the
   underlying decision has been superseded or the code it described has been
   rewritten. Searches and ingestion-time contradiction checks continue to weigh
   it as if it were still current.
2. **Dreaming promotes by raw `avg_relevance`, not by recent signal.** A memory
   recalled 10 times in week 1 and never again scores identically to a memory
   recalled 10 times over the past two weeks at the moment of the next dreaming
   pass. Early-bursty-then-abandoned memories bias against recently-relevant
   ones.

Empirically the corpus is small enough (238 memories, 13 syntheses after the
2026-04-18 run) that neither failure mode has produced a user-visible problem
yet. But the intent of BL-008 is to ship forgetting **before** either does — so
that by the time synthesis layers (BL-009 onwards) or the daemon (BL-010) exist,
the recall/relevance signal they consume already reflects temporal reality.

## Scope

This discussion decides the design of the decay + demotion primitive added to
the existing Dreaming pass. In scope:

- When and how `avg_relevance` is combined with a time factor to produce an
  effective weight for decision-making
- Whether stored `avg_relevance` is mutated or preserved
- What demotion (if any) is triggered, and on what signal
- How the change interacts with the existing promotion thresholds
- How the mechanism can be validated empirically and in tests

Out of scope (explicitly deferred to later backlog items):

- Replacing `avg_relevance` with a different base scoring function (that's
  BL-014 / RL feedback territory)
- Entity-extraction or synthesis-layer decay (those consume this primitive,
  they don't redefine it)
- Cross-project decay sharing (BL-010 cross-project item)
- Removing or hard-deleting memories (out of scope regardless — `valid_until`
  remains the soft-delete signal)

## Constraints

- **Hard constraint (from backlog):** stored `avg_relevance` MUST NOT be
  mutated by decay logic. Decay must be a read-time or event-time derivation.
  Rationale captured in BL-008: destructive mutation makes the signal
  irreversible and conflicts with the contradiction/temporal-validity model.
- **Hard constraint (SQLite single-writer):** any per-row recomputation path
  must fit inside the existing `Arc<Mutex<Connection>>` model without
  introducing new write amplification proportional to corpus size. Daily
  Dreaming pass is an acceptable write point; every search/ingest is not.
- **Hard constraint (no new external dependency):** implementation lives in
  `src/core/dreaming.rs` with the same `chrono` + `rusqlite` surface already
  in use. No new crate.
- **Soft constraint (scope):** BL-008 target is ~50–100 LOC. A design that
  requires a new schema migration, a new background task, or a new MCP tool
  is evidence the scope has been exceeded — surface that as a finding, don't
  silently absorb it.
- **Precondition to avoid breaking:** plan 011's residual-reduction work and
  plan 010's synthesis rely on `avg_relevance` and `is_longterm` as stable
  signals. Whatever decay design lands must either leave those signals
  reading the same way (ideal) or explicitly enumerate the downstream
  readers it changes.

## Key Questions

1. What does "effective relevance" mean operationally — is it a value
   computed by the dreaming pass and compared against thresholds, a value
   exposed to search ranking, or both?
2. Under what condition is a memory demoted? Is demotion symmetric with
   promotion (same signal, inverted threshold), or is it a distinct concept
   (e.g., "was long-term, now decayed below floor")?
3. What empirical signal would tell us, after shipping, that decay is
   behaving as intended vs. over- or under-aggressive? What would a
   regression look like?
4. How does decay interact with the existing `last_recalled` and
   `recall_count` — do we still need both, or does a decayed recency
   measure subsume one?
5. What is the minimum change that produces the intended behavior? Is
   there a version strictly smaller than the backlog item's sketch that
   still earns the "forgetting" property?

## Non-questions (not seeking debate on)

- Whether forgetting matters — BL-008 was committed at roadmap time; this
  discussion is how to implement it, not whether to.
- Whether `valid_until` / soft-delete should replace decay — the two are
  orthogonal (valid_until is authored invalidation; decay is passive
  irrelevance).
- Which power-law exponent to use as a final constant — the backlog sketch
  suggests `0.95^days` but the discussion may argue for any monotonically
  decreasing function; picking the final number is a tuning step, not a
  design question.
