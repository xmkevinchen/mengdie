---
id: "01"
title: "BL-009 design — whether + what"
status: converged
current_round: 2
created: 2026-04-27
decision: "Build BL-009 in v0.9.0 as McpSessionProvider (new LlmProvider impl using rmcp Peer<RoleServer>::create_message sampling) + thin memory_dream tool wrapper. Construction-time capability check at MengdieServer::new selects McpSessionProvider (if Claude Code advertises sampling) or ClaudeCliProvider fallback. Both flow through existing run_synthesis_pass → insert_synthesis_with_links, satisfying cluster-hash invariant. ~50-150 LOC. memory_ingest's SourceType::Synthesis path removed as cleanup."
rationale: "4/5 strong convergence on McpSessionProvider after evidence-driven Round 2 movements: architect retracted Shape B (rmcp sampling verified by TL); gemini conceded new-tool + prompt-level-context-reuse claims; minimal-change FLIPPED defer to yes-build (gating condition rmcp sampling resolved); codex confirmed Path C ≡ McpSessionProvider. Challenger conceded F1/F4/F6/F7 technical points; held defer-until-trigger but TL judgment: silent fallback + bounded cost + Phase 2 chain benefit from tested sampling path. McpSessionProvider satisfies discussion 008 'extend, don't add' precedent (zero new write paths; thin trigger tool only). Pressure-tests LlmProvider trait extensibility per plan 010 first-caller pattern. Does not foreclose BL-010 daemon (orthogonal LlmProvider paths)."
reversibility: high
reversibility_basis: "Provider pattern means impl swap is trivial — McpSessionProvider can be deleted; memory_dream tool can be unregistered. No schema changes. ClaudeCliProvider remains canonical fallback."
---

# Topic: BL-009 design — whether + what

## Current Status

**CONVERGED** in Round 2.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | revisit | 5 distinct positions surface; rmcp sampling support is the deciding open question (3/5 reviewers flag it); cluster-hash bypass invariant universally accepted as binding constraint. |
| 2 | converged | TL verified rmcp v1.3 supports server-initiated sampling. Architect retracted Shape B; gemini conceded new-tool position; minimal-change flipped defer; codex confirmed Path C convergence. Challenger held defer-until-trigger but conceded all 4 technical findings. McpSessionProvider with construction-time capability check + ClaudeCliProvider fallback emerged as the converged design. |

## Context (preserved from creation)

mengdie's existing `mengdie dream --synthesize` shells out to `claude` CLI. When mengdie runs as MCP server inside a Claude session, the host Claude IS the LLM. Discussion 023 conclusion gated v0.8.5 sprint commit on running this discussion first.

## Constraints (preserved + reaffirmed)

- Cluster-hash NOT NULL invariant (v0.8.5 BL): satisfied transparently by routing through `run_synthesis_pass` → `insert_synthesis_with_links`.
- Coexistence with `dream --synthesize` CLI: preserved — ClaudeCliProvider remains canonical for batch CLI.
- Don't foreclose BL-010 daemon: confirmed orthogonal — daemon uses ClaudeCliProvider, in-session uses McpSessionProvider.
- MCP transport limits (rmcp v1.3 stdio): server-initiated sampling supported via `Peer<RoleServer>::create_message()` (TL verified).

## Decision details

See `docs/discussions/024-bl-009-mcp-dream-tool/conclusion.md` for full Decision Summary table + Doodlestein review.
