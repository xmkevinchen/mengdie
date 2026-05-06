---
id: "03"
title: "Cross-project default retrieval scope — ratify §5 with rationale refinement"
type: ratify-or-defer
prior_commitment: "CLAUDE.md Key Design Decisions §5 — Global storage, per-project default search"
status: converged
current_round: 2
created: 2026-05-05
decision: "Ratify §5 per-project default retrieval scope unchanged at the storage/query level. AE skills should specify scope explicitly per-skill (default-when-omitted is fallback, not load-bearing surface). Cross-project synthesis explicitly deferred to P2 with trigger condition."
rationale: "v0.0.1 RATIONALE (recorded in conclusion as the new authoritative reason): cross-project contamination risk — project-A-specific decisions surfaced in project-B contexts; AI agents won't reliably filter by provenance — outweighs cross-project recall benefit at solo-operator scale (ai-engineer cluster contamination argument round-01:476-494). The original §5 'avoid migration cost' framing is superseded since storage is already global (archaeologist verified mcp_tools.rs:192-195 1-line diff). 7/7 ratify (challenger updated R2 with new rationale framing)."
reversibility: high
reversibility_basis: "Per-project default is mcp_tools.rs:192-195 4-line conditional; storage already global; flipping default = 1-line change. Trigger for revisit recorded below."
---

# Topic: Cross-project default retrieval scope — ratify §5

## Current Status
**CONVERGED Round 2 — ratify §5 per-project default with rationale refinement.** Reopening trigger recorded below.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 5 ratify, 1 dissent (challenger argued §5 was migration-cost deferral that disappears in rebuild) |
| 2 | converged | 7/7 ratify. challenger updated rationale to "contamination risk outweighs recall benefit" (accepting ai-engineer's cluster contamination argument round-01:476-494). project_id cwd-staleness bug filed as separate BL. |

## Reopening trigger (record in conclusion)
- ≥10% of F-002 audit-table queries observed using `scope: 'global'` opt-in over 30 days, OR
- 3 retrospect-reported incidents of "I knew this was decided in another project but mengdie didn't surface it"

## Type: ratify-or-defer
This is **not an open from-scratch decision**. CLAUDE.md Key Design
Decisions §5 already commits: "Global storage, per-project default
search — avoid migration cost when adding cross-project later."
Round 1 is an evidence-check, not a 5-option deliberation.

Acceptable outcomes (in order of expected likelihood):
- **Ratify §5 unchanged** — confirm per-project default; record
  reversibility (high, since storage is already global).
- **Defer with trigger** — keep §5 in force; record an explicit
  trigger that would reopen this (e.g., "revisit when N
  cross-project queries observed in F-002 audit table" or "when
  the operator works across ≥2 active projects simultaneously").
- **Revise** — only with surfaced evidence that §5 is actively
  damaging (e.g., audit data showing X% of queries genuinely
  benefit from cross-project sources but lose them).

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|

## Context
mengdie stores memories globally in `~/.mengdie/db.sqlite`, tagged
with `project_id` inferred from each ingest's git context. Searches
are scoped to project-by-default with cross-project opt-in.

This default works when the operator is single-domain inside one
project. It fights the operator when:
- A new project re-discovers a decision already made elsewhere
  (lost institutional memory)
- A cross-cutting concern (Rust idioms, MCP protocol patterns,
  CLAUDE.md conventions) is genuinely shared infrastructure
- The operator works across multiple projects on the same problem
  (e.g., AE plugin development + mengdie + agency-agents curation
  all share "agent governance" context)

The blueprint §8 phrasing is open: "Is the default a policy
decision or a user-config option per call?" That is, two
sub-questions:
1. What is the right default?
2. Should the default be a global setting, or expressed at search
   call time?

## Constraints
- `project_id` inference is git-remote-based (when remote present)
  with a fallback path-derived id; this is established in
  `core/project.rs` and not changing in v0.0.1
- MCP tool API surface is constrained — adding parameters per call
  works, but every parameter is one more thing the AE plugin and
  ae:analyze post-research injection must teach the agents to use
- Cross-project defaults risk cross-contamination — a memory that
  was true in project A may be wrong in project B (different stack,
  different convention)
- Per-project defaults risk siloing — exactly the loss-of-memory
  problem mengdie is supposed to solve
- Whatever is chosen must be observable: the operator should be
  able to tell from a search result whether cross-project sources
  contributed

## Key Questions
- What does the data say about how often the operator's actual
  queries genuinely benefit from cross-project sources? (Reading
  recent ae:analyze invocations or ae:discuss conclusions for
  cross-project signal would be evidence here.)
- Is there a structural difference between memory types that should
  be cross-project by default (e.g., "Rust idiom" type) versus
  project-only by default (e.g., "decision specific to project X")?
- If per-call config: who decides — the calling skill (ae:analyze
  always cross-project), the agent (LLM judgment), or the operator
  (explicit flag)?
- How do existing OSS frameworks handle multi-namespace search
  (mem0's `user_id`, Graphiti's `group_id`, LangMem namespacing)
  and is there a transferable pattern?
- What's the simplest implementation that lets the operator change
  this answer cheaply if they're wrong?
