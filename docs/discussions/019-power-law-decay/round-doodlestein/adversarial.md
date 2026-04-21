---
author: doodlestein-adversarial
role: post-conclusion adversarial reviewer
date: 2026-04-20
---

# Adversarial Review — Discussion 019 Conclusion

## Finding

**Decision 3 (demotion semantics & threshold) is the most likely first-fail point.**

Specifically: the NULL `last_recalled` skip rule combined with the claim that
"zero demotions on first pass under any floor" makes the design empirically
safe.

### The failure mode

The conclusion cites Archaeologist V3 to assert the design is empirically
safe: zero demotions on first pass at floor 0.10, 0.20, or 0.30. But this
safety result is load-bearing on a **circular assumption**: if today's corpus
has very few (or zero) memories with both `is_longterm = 1` AND
`last_recalled IS NOT NULL` AND sufficient staleness to cross the floor, then
of course zero demotions occur. The conclusion does not report how many of
the 41 long-term memories actually *have* a non-NULL `last_recalled`, nor
what their oldest stale gap is.

The 1/41 NULL case is noted, but the remaining 40 are silently assumed to
have `last_recalled` values recent enough to keep `effective_relevance`
above 0.20. If a significant subset of the 40 have `last_recalled` values
from months ago (plausible for a corpus seeded with batch-imported AE
discussions), the first real Dreaming pass could demote several long-term
memories at once — not after 77 days of silence, but *immediately on first
run*, depending on how stale those timestamps already are.

### Why this matters more than the other decisions

- Decision 1 (formula & constants): pure math, tested with table-driven
  regression cases. Can't silently misbehave.
- Decision 2 (compute location): purely structural; both sites use the same
  clock by design. No hidden state.
- Decision 4 (promotion unchanged): explicitly narrowed scope; no surprise
  there.
- Decision 5 (observability): `--dry-run-decay` exists precisely to catch
  this. But only if someone runs it before the first live Dreaming pass.

Decision 3 is the one where the empirical safety claim ("zero demotions")
may not generalize past the snapshot date of the Archaeologist's corpus
query. The `--dry-run-decay` flag (Decision 5) is the correct mitigation,
but the conclusion does not mandate running it before the first live pass.
It only says "operators need a pre-mutation validation path in the interim."
That's advisory, not procedural.

### Concrete risk

First `mengdie dream` post-ship, without a prior `--dry-run-decay` run,
could silently demote multiple long-term memories. No error, no warning —
`DreamingResult.demoted` would report the count, but only if the caller
inspects it. The CLI output format for this field is not specified in the
conclusion.

### Verdict

Not a reopen signal. The design is correct. The gap is operational: the
conclusion should add a **mandatory `--dry-run-decay` step before the first
live Dreaming pass** as a plan pre-condition, and the plan should specify
that `DreamingResult.demoted` is surfaced visibly in CLI output (not just
in the struct). Both are plan-level guards, not design changes.
