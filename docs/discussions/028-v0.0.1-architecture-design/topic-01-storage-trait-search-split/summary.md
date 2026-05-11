---
id: "01"
title: "Storage trait introduction + search-split refactor scope"
status: converged
current_round: 2
created: 2026-04-27
decision: "Search-split refactor IN v0.0.1 (alongside two-ingest-paths defect fix). Storage trait NOT introduced in v0.0.1; mechanism = free functions over &Db. Trait deferred to Tier 2 trigger (Kuzu adoption)."
rationale: "4-of-5 majority converged on free functions + no trait. arch-reviewer's 'decisions are independent' argument decisive (search-split is cleanup, trait is YAGNI question). minimal-change-engineer's Rust nominal-typing-cost argument refutes gemini's Go-style 'design interfaces early' reasoning. challenger's YAGNI rule (≥2 impls in same sprint) is satisfied for trait deferral."
reversibility: high
reversibility_basis: "Free functions can be wrapped in a trait later when 2nd Storage impl materializes. No data migration cost; localized refactor of call sites."
gemini_dissent: "CONDITIONAL ACCEPT trait if search-split in v0.0.1. Recorded in conclusion."
---

# Topic: Storage trait introduction + search-split refactor scope

## Current Status
**Converged** (Round 2). Search-split YES; Storage trait NO; mechanism = free functions over `&Db`.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | exploratory | 4-of-5 free fns; gemini outlier with conditional trait |
| 2 | converged | Decision reached; gemini dissent recorded; reversibility high |

## Context

archaeologist's empirical finding: `src/core/search.rs:80` defines
search functions as `impl Db { fn memory_search() }` — search is
grafted onto the Db struct's method surface, not a module-level API.
Callers use `self.db.memory_search()` not `search::memory_search(&db, ...)`.
At the type level, the Retrieval layer does not exist as a
boundary.

architecture-reviewer + codex-proxy: accept the proposed `Storage`
trait conditionally — only if search is split out so `Storage` can
be narrow (CRUD + temporal only).

challenger's YAGNI rule (introduce trait only when ≥2 concrete
impls exist or are committed in the same sprint): SQLite Tier 1 has
1 impl. Tier 2 Kuzu has no commit date. By rule, premature. But
challenger conditionally accepts: if v0.0.1 includes the search-split
refactor, defining the trait in the same change is justified
because the refactor is the work that makes the boundary real.

The decision affects v0.0.1 scope: search-split changes all callers
in mcp_tools.rs and cli.rs. Including it expands v0.0.1 work;
excluding it defers the trait to a future trigger.

## Constraints

- The `mcp_tools.rs` two-ingest-paths defect MUST be fixed in v0.0.1
  (4-of-4 convergence in analyze phase). Fixing it touches
  mcp_tools.rs anyway.
- Blueprint §7 ladder requires `Storage` trait by Tier 2 (Kuzu).
  Tier 2 has no commit date.
- Blueprint §6 implementation principle: do not introduce
  abstractions that are not earned by current need.

## Key Questions

- Is the search-split refactor (`search.rs` functions →
  module-level API) within v0.0.1 scope, given the operator's stated
  minimum (wire AE Round-0 + fix two-ingest-paths + sqlite-vec spike
  + ship)?
- If a Storage abstraction is introduced, **what mechanism** is right
  at v0.0.1 — Rust trait (proposed), struct (lighter, no
  swappability), free functions over a `Connection` handle (lightest,
  no abstraction), or no abstraction at all (commit to SQLite-direct
  until Tier 2 trigger fires)? Trait is one option, not the
  pre-decided one.
- If search-split is not in scope, what concrete trigger fires the
  decision to do it later?
- Is rejecting both the search-split refactor AND the Storage
  abstraction (i.e., keep v0.8.0 Db-extension shape until Tier 2
  forces the boundary) a defensible v0.0.1 stance? Under what
  conditions?
