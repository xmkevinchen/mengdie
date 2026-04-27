---
round: 1
date: 2026-04-27
type: tl-synthesis
---

# Round 1 — TL Synthesis

## Per-agent files (REQUIRED READING for Round 2)

- `round-01/challenger.md`
- `round-01/minimal-change-engineer.md`
- `round-01/software-architect.md`
- `round-01/codex-proxy.md`
- `round-01/gemini-proxy.md`

This synthesis is orientation only. Round 2 agents must read peer files
directly and cite by file:line.

## Position summary (orientation only)

| Agent | Q1 (whether) | Q2 (mechanism if yes) | Open blocker |
|-------|--------------|----------------------|--------------|
| **gemini-proxy** | YES — UX win (conversation context, structured feedback per cluster) | New tool with structured `{synthesized, failed, residuals_count}` output | none surfaced |
| **challenger** | NO — stub broken, no measurable win, no empirical support, BL-009 may be a parallel-path dead-end | If forced: cluster-read only, defer execution to BL-010 | none surfaced |
| **minimal-change-engineer** | NO — no quantified pain; CLI works; defer-until-trigger | If forced: McpSessionProvider (NEW LlmProvider impl, ~50 LOC) reusing 100% of run_synthesis_pass — NOT new tool | **rmcp sampling support** + **Claude Code client sampling capability** |
| **software-architect** | (didn't take Q1 explicit position; structural analysis) | Shape B (two-tool: read clusters + dedicated commit) — maps to BL-010 queue model | **rmcp v1.3 sampling presence** (forecloses option E if absent) |
| **codex-proxy** | YES — risk of deferral substantial; Phase 2 chain divergence | Path C (McpLlmProvider + reverse tool call = same as McpSessionProvider) — unifies CLI + MCP over single backend | **rmcp v1.3 reverse-tool-call support** |

## Pruned

- **Pruned**: BL-009 stub literal text ("Claude calls memory_ingest"). All 5 agents converged: stub is mechanically broken because memory_ingest path bypasses `synthesis_cluster_hash` invariant (verified in discussion 023; v0.8.5 enforces at DB layer). Round 2 reasons from "what shape replaces the stub" not "how to honor the stub." UAG-style finding: independently reached by all 5.
- **Pruned**: gemini-proxy's "context reuse" benefit, IF challenger F4 holds. Challenger F4: synthesis prompt is fully self-contained (3-6 sentences); prior session context adds nothing. Gemini's argument was conversation-level UX, not prompt-level context. Round 2 should clarify whether the win is "Kai sees structured feedback in his current chat" (UX-conversational) or "Claude has more relevant context for synthesis" (prompt-level). The prompt-level claim is pruned.
- **Pruned**: nothing else; all positions advance to Round 2 cross-fire.

## Of-framing disposition

Three challenges raised that touch framing rather than topic:

1. **Challenger F2 (no measurable latency win)**: framing's Q1 implicitly assumed "indirection is real cost." Challenger argues subprocess spawn is dominated by LLM inference. **Disposition: integrate as live disagreement.** Round 2 should engage gemini-proxy (UX win not perf) vs. challenger (no perf win) directly. Framing's "whether worth eliminating" is correctly open to both readings.
2. **Architect F2 (cluster-hash invariant belongs at DB layer, not MCP-tool layer)**: framing's Reference says "v0.8.5 BL closes at DB layer" — architect goes further, saying "BL-009 must NOT reimplement enforcement; it must call insert_synthesis_with_links." **Disposition: integrate.** This is a hardening of framing's existing constraint, not a reframe.
3. **Minimal-change F4 (NOT a new tool, McpSessionProvider)**: framing's Q2 left mechanism open to "new MCP tool / extend memory_ingest / new LlmProvider impl / something else." Minimal-change argues NEW LlmProvider impl wins on minimum-machinery + zero new MCP tool surface. **Disposition: integrate as primary candidate.** Strong evidence (~50 LOC vs 100-150) + reuse of run_synthesis_pass + alignment with discussion 008 "extend, don't add" precedent.

## Verification artifact

| Claim | Source | Verified? |
|-------|--------|-----------|
| rmcp v1.3 supports server-initiated sampling | TL context7 query of rmcp docs (2026-04-27) | ✓ — `Peer<RoleServer>::create_message()` exists; `CreateMessageRequestParams` type defined; `enable_sampling_tools` available under `server` or `macros` features (mengdie has both) |
| Claude Code MCP client advertises sampling capability | not verified by TL | ✗ runtime question; cannot be answered at design time. Plan must include runtime check + fallback path |
| memory_ingest bypasses cluster-hash invariant | discussion 023 architect F1 + TL code grep at db.rs:122-163 | ✓ (confirmed in discussion 023) |
| Production dream pass produced 13 syntheses via ClaudeCliProvider | minimal-change Round 1 citing CLAUDE.md | ✓ (recorded in CLAUDE.md Project Status) |
| dreaming.rs:399 run_synthesis_pass already separates 4 responsibilities | architect Round 1 | ✓ (architect cited file:line) |
| `insert_synthesis_with_links` (db.rs:349) computes hash from source_ids | architect Round 1 | ✓ (matches discussion 023 verification) |
| BL-010 (daemon) queue model is foreclosed by some BL-009 shapes | architect Round 1 | unvalidated by TL — architect's structural argument; round 2 should attempt to falsify (find a shape that supports both in-session AND queue) |
| Subprocess spawn cost dominated by LLM inference time | challenger F2 | unvalidated — claim plausible but no benchmark cited; Round 2 doesn't need to settle, just acknowledge |

**Critical verification result**: rmcp sampling IS available. This means McpSessionProvider/Path C is architecturally feasible. The remaining open question is runtime (does Claude Code advertise sampling) — answerable only via integration test.

## Frame-challenge disappearance self-check

Framing Round 0 raised 5 reviewer concerns. Status check:

- ✓ "Whether to build" question (adversarial Round 0): explicitly engaged — challenger, minimal-change argued NO; gemini, codex argued YES; architect didn't take explicit position. Live disagreement, not silent drop.
- ✓ "Invariant boundary" reframe (strategic Round 0): adopted by architect (F2 "cluster-hash belongs at DB layer; BL-009 must call insert_synthesis_with_links") and by minimal-change (McpSessionProvider preserves invariant via run_synthesis_pass reuse).
- ✓ "Decouple constraint from mechanism" (gemini Round 0): both YES-camp positions (gemini, codex) and NO-camp positions reasoned about constraint-fulfillment shape independently of MCP-vs-CLI choice.
- ✓ "Mechanism trade-off in framing" (gemini Round 0 attempt 3, overridden): NOT silently injected. Round 1 surfaced 5 distinct mechanism candidates (new tool A, two-tool B, stateful session C, McpSessionProvider, no-build) — proves the override didn't lose design space.
- ✓ "Don't foreclose Phase 2 chain": architect F5 + challenger F7 both engaged BL-010 compatibility explicitly.

## Convergence map

**Convergent (4-5 agree)**:
- ✓ **BL-009 stub is mechanically broken** (5/5). memory_ingest path cannot persist synthesis rows correctly post-v0.8.5.
- ✓ **Cluster-hash invariant must NOT be reimplemented in any BL-009 design** (5/5 implicit). Whatever ships must call `insert_synthesis_with_links` (db.rs:349) or equivalent that computes hash from source_ids.
- ✓ **rmcp sampling availability is the deciding question** for whether McpSessionProvider is feasible (3/5 explicitly surface this; TL verified ✓).

**Contested (split)**:
- ✗ **Q1: build or defer**: 2 NO (challenger, minimal-change) vs 2 YES (gemini, codex) vs 1 conditional (architect, leans Shape B if Q1=yes).
- ✗ **Q2 mechanism shape if yes**:
  - Gemini: new tool with structured feedback
  - Architect: Shape B (two-tool: read + commit)
  - Codex: Path C (McpLlmProvider = McpSessionProvider)
  - Minimal-change: McpSessionProvider (= Path C) ONLY IF rmcp sampling works
  - Challenger: cluster-read only, defer execution to BL-010

**McpSessionProvider has 2 advocates (codex + minimal-change conditional)** + becomes architect-feasible per TL verification (rmcp supports sampling). Round 2 should test this convergence.

## Round 2 directive

Each agent reads ALL of: challenger.md, minimal-change-engineer.md, software-architect.md, codex-proxy.md, gemini-proxy.md.

**Specific cross-cuts each agent must address in their round-02/<name>.md**:

- Where do you AGREE with another agent's evidence-backed claim that contradicts your Round 1 position?
- Where do you DISAGREE — citing the peer's file:line — and why does your evidence override theirs?

**Resolved by TL verification (no further work needed)**:
- rmcp v1.3 sampling: **AVAILABLE**. McpSessionProvider is feasible. Round 2 reason from this fact.
- Claude Code client sampling capability advertisement: **UNVERIFIABLE at design time**. Round 2 must include "runtime fallback to ClaudeCliProvider if sampling unavailable" in any McpSessionProvider proposal.

**Specific Round 2 questions to converge on**:

1. Q1 binary: build BL-009 in v0.9.0 or defer? Now that McpSessionProvider is feasible (~50 LOC + runtime fallback), does the minimal-machinery cost change minimal-change/challenger's NO position?
2. If yes-build: McpSessionProvider (codex Path C / minimal-change preferred) vs Shape B two-tool (architect preferred) vs new tool with structured feedback (gemini preferred). Read peer arguments, defend or concede.
3. Discussion 008 "extend, don't add" precedent — does it apply to BL-009 or break here? McpSessionProvider doesn't add tools at all; new tool / Shape B do.
4. Failure-mode handling: McpSessionProvider with runtime detection + ClaudeCliProvider fallback — is this enough? Or does it just re-create the parallel-path debt challenger flagged?
5. BL-010 daemon compatibility: which shape is most compatible? (architect Round 1 F5: Shape B trivially maps to queue; McpSessionProvider in-tool blocks the queue model UNLESS daemon uses CLI path always.)

**Open questions NOT for Round 2 to resolve** (defer):
- Synthesis prompt content (out of scope per framing)
- Exact rmcp sampling API ergonomics (plan-time concern)
- Claude Code's actual sampling support (runtime concern; design must accommodate either)
