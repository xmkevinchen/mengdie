---
id: BL-synthesis-result-struct-promotion
status: closed
origin: plan 011 /ae:review (architecture-reviewer P2)
created: 2026-04-18
closed: 2026-04-19
closed_reason: "Superseded by plan 012. The 'second display-layer counter' (pair_clusters_skipped) landed as a field on the existing SynthesisResult struct instead of triggering wrapper-struct promotion. Tuple return eliminated entirely (run_synthesis_pass now returns Result<SynthesisResult>). Trigger avoided; no wrapper needed."
scope: mengdie (ergonomic refactor)
---

# Promote `(SynthesisResult, usize)` tuple → `SynthesisPassResult` struct

## Finding

`run_synthesis_pass` in `src/core/dreaming.rs` currently returns
`anyhow::Result<(SynthesisResult, usize)>` where the second element is
`pair_clusters_processed` — a local counter tracked for CLI display-layer
use (AC3 denominator in plan 011). The tuple shape is defensible now: one
callsite (`cli.rs:237`), one internal test consumer, zero external users.

Architecture-reviewer (plan 011 review) noted: if a future plan adds a
second display-layer counter (e.g., `singleton_residuals_observed`,
`max_cluster_size_observed`, `llm_total_latency_ms`), the tuple becomes a
struct by necessity. Proactively promote to
`SynthesisPassResult { result: SynthesisResult, pair_clusters_processed: usize }`
at that time so the API surface doesn't grow ad-hoc tuple arities.

## Trigger

Fires when ANY ONE of:

1. A second plan adds a display-layer counter that would otherwise become
   tuple element `.2`.
2. An external caller (outside `cli.rs`) needs the `pair_clusters_processed`
   value for any purpose (e.g., a new `mengdie stats` subcommand).
3. `SynthesisResult` itself gets promoted into a public API surface that
   downstream tools depend on (the tuple return becomes a breaking boundary).

## Why not fixed in plan 011

Premature — CLAUDE.md explicitly says "don't design for hypothetical
future requirements." The tuple is honest about its scope (display-only,
single consumer). A struct with one data field alongside `SynthesisResult`
would be over-engineered for the current shape.

## Fix direction

Trivial when trigger fires:

```rust
pub struct SynthesisPassResult {
    pub result: SynthesisResult,
    pub pair_clusters_processed: usize,
    // future fields land here
}

pub async fn run_synthesis_pass(...) -> anyhow::Result<SynthesisPassResult>;
```

Migrate:
- One `let (r, pair_count) = ...` → `let pass = ...; let pass.result; let pass.pair_clusters_processed` at CLI
- Test destructures update (4 test sites in `src/core/dreaming.rs`)
- The `tests/dream_synthesis.rs` e2e test (one site)

One commit, no schema change, no semantic change.
