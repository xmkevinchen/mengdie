---
discussion: "024"
step: "doodlestein"
type: "regret"
date: 2026-04-24
---

# Regret Analysis — Discussion 024

## Most Likely Reversed Decision

**Construction-time capability check (vs per-call detection)**

This is the decision most likely to be reversed in 6 months — not because it's wrong in principle, but because it resolves a runtime uncertainty at the wrong boundary.

## Why This One

The construction-time check resolves provider selection once at `MengdieServer::new`. The conclusion explicitly flags two unvalidated open questions:

> Exact rmcp v1.3 client-capability detection API surface (`peer_info().capabilities.sampling` is plausible but needs verification at code time)
> Whether `MengdieServer` gets a per-server or per-request `Peer<RoleServer>` handle

If the `Peer<RoleServer>` handle is per-request (not per-server), then construction-time resolution is impossible. There is no peer at construction time — the connection doesn't exist yet. The conclusion acknowledges this gap ("plausible but needs verification at code time") but defers it to plan-time. That means a load-bearing architectural choice (single-resolution provider selection) depends on an assumption that may not survive first contact with the rmcp internals.

The other candidates are more stable:

- **Build-now vs defer-until-trigger**: 4/5 convergence + bounded cost + silent fallback makes reversal low-probability. The challenger's "defer" position required runtime fallback to be pointless — it isn't, because ClaudeCliProvider still works.
- **McpSessionProvider as new LlmProvider impl**: The trait-based swap is low-risk by design. If sampling proves unreliable, the impl swaps to ClaudeCliProvider with no interface change.
- **ClaudeCliProvider as silent fallback**: This is an operational safety property. Nothing makes it worse over time.
- **Sequencing BL-009 with v0.8.5**: Already resolved by the "AFTER v0.8.5 ships" rule. Low reversal risk.
- **SourceType::Synthesis cleanup**: This is a bug fix, not a design choice. No reversal surface.

## Likely Reversal Scenario

At plan-time, the implementer discovers that `Peer<RoleServer>` is scoped per MCP session/connection, meaning `MengdieServer::new` is called before any client connects. At that point:

1. Construction-time resolution is impossible — there is no peer to query at init.
2. The team reverts to per-call detection: check sampling capability on each `memory_dream` invocation.
3. Per-call detection introduces the branching that architect + minimal-change specifically argued against ("no per-call branching; no test-matrix doubling").

This is a 6-month reversal because the gap won't surface until BL-009 plan starts, which is sequenced after v0.8.5 ships.

## Severity

Medium. The construction-time design is an optimization (avoiding per-call branching), not a correctness requirement. If it reverses to per-call, the architecture still works — McpSessionProvider + ClaudeCliProvider both remain; the branching logic just moves from `new()` to `memory_dream()`. The trait-based design survives either way. Code complexity increases modestly; the conclusion's 50-150 LOC estimate may grow.

## Recommended Follow-Up

File a BL targeting the plan-time verification step.

**Shape**: Add to BL-009 plan as a mandatory pre-step (not a new backlog item):

> Before writing any code, confirm via rmcp v1.3 source whether `Peer<RoleServer>` is available at server construction time. If not, pivot construction-time resolution to lazy initialization (first `memory_dream` call) or per-call detection. This gates the rest of the implementation.

This doesn't need a new discussion — it's a single-question API-surface check that the plan can absorb. Mark it as a hard gate on step 3 ("Add construction-time provider selection in `MengdieServer::new`") in the Next Steps list.

If the check fails, the plan's step 3 becomes: "Add per-call provider selection in `memory_dream` handler" with a note that test-matrix doubling risk is acceptable at this LOC scale (~50-150 LOC total, not a large branching surface).
