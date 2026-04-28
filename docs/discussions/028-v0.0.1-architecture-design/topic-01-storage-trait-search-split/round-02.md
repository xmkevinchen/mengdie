---
round: 02
date: 2026-04-28
score: converged
---

# Round 02 — Topic 1

## Discussion

**Position evolution**:

| Agent | Round 1 | Round 2 |
|---|---|---|
| architecture-reviewer | NO trait, free fns | same; sharpened: search-split + Storage trait are independent decisions |
| minimal-change-engineer | NO trait, free fns | same; new arg: Rust nominal-typing cost asymmetry vs Go's structural typing makes trait costlier |
| challenger | NO trait default | HOLD; engaged gemini's Google pattern as pattern-evidence-not-mengdie-evidence |
| codex-proxy | DEFER trait, "concrete internally" | clarified: "concrete internally" = free fns over `&Db`; convergent with majority |
| gemini-proxy | CONDITIONAL ACCEPT trait | HOLD with refinement: "if search-split in v0.0.1, define trait" |

**Key arguments**:

- arch-reviewer (round-02): "search-split is IN scope (as its own
  cleanup), Storage trait is NOT, and these decisions are
  independent."
- minimal-change-engineer (round-02): gemini's "search-split makes
  trait shape clean" is a reason to *defer* trait until shape is
  stable, not introduce it now. Rust nominal typing means trait
  introduction is heavier than Go structural typing.
- challenger (round-02): Apply YAGNI rule — one impl in v0.0.1, no
  2nd impl committed in-sprint. gemini's "Google favors designing
  interfaces early" is pattern evidence, not mengdie-specific
  evidence.
- gemini (round-02): "Google's API-first design supports trait
  introduction for architectural boundaries, not just swappability."

**Disagreement at close**: 4-of-5 vs gemini. gemini's position
reduces to "search-split entry triggers trait." Majority position is
"search-split + trait are independent decisions; even with search-
split in v0.0.1, no trait."

## Outcome

- **Score**: converged
- **Decision**:
  1. **Search-split refactor**: IN v0.0.1 scope (as part of fixing
     the `mcp_tools.rs` two-ingest-paths defect — fixing the defect
     touches mcp_tools.rs callers anyway, making the search-split
     marginal cost). search.rs functions move from `impl Db { fn
     memory_search() }` to module-level `search::memory_search(&db, ...)`.
  2. **Storage trait**: NOT introduced in v0.0.1. Mechanism =
     **free functions over `&Db`** (or, equivalently, methods on a
     concrete `Db` struct, depending on author preference). Trait
     introduction deferred to Tier 2 trigger (Kuzu adoption, when
     a 2nd Storage impl exists).
- **Rationale**:
  1. 4-of-5 majority converged on free functions + no trait.
  2. arch-reviewer's "decisions are independent" argument is
     decisive — search-split is a code-organization cleanup that
     doesn't necessitate trait abstraction.
  3. minimal-change-engineer's Rust nominal-typing cost argument
     refutes gemini's Go-style "design interfaces early" reasoning.
  4. challenger's YAGNI rule (≥2 impls in same sprint) is satisfied
     for the trait deferral path: only 1 impl exists; Tier 2 has no
     commit date.
  5. gemini's CONDITIONAL ACCEPT is a minority position grounded in
     a Google-ecosystem pattern that does not transfer cleanly to
     mengdie's Rust single-binary context.
- **Reversibility**: high
- **Reversibility basis**: Free functions can be wrapped in a trait
  later when a 2nd Storage impl materializes (Tier 2 Kuzu adoption).
  No data migration cost. The trait introduction is a localized
  refactor of the call sites that today use free functions.

## gemini dissent (recorded)

gemini holds CONDITIONAL ACCEPT trait if search-split is in v0.0.1
scope. Decision proceeds against this position. If gemini's "Google
API-first" argument turns out to apply (e.g., a 2nd Storage impl
appears unexpectedly within v0.0.1), the trait can be introduced
incrementally per the high reversibility basis above.
