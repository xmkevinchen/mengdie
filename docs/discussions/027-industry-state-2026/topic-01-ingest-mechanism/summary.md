---
id: "01"
title: "Ingest mechanism — delivery pattern from AE to mengdie"
type: open
gated_by: topic-04-ingest-source-boundary
status: converged
current_round: 2
created: 2026-05-05
decision: "Push-primary; AE skill explicitly calls memory_ingest after each pipeline phase. core/watcher.rs kept as opt-in library code, NOT wired to mcp_server.rs/cli.rs daemon. Cold-start uses cmd_import (cli.rs:361, already shipped)."
rationale: "7/7 convergence. resolves atomicity is push-only by construction (system-architect round-01:122-126). cmd_import covers cold-start replay (archaeologist round-02 verified cli.rs:361-424). Pull-fallback for AE-not-running produces no inputs anyway. Industry pattern: mem0 v1.0 async write path, LangMem ReflectionExecutor, Vector Stores API all push. challenger updated R2 after accepting resolves atomicity as design-merit (independent of v0.x execution gap)."
reversibility: high
reversibility_basis: "Both push and pull infrastructure exist in code. Switching directions is wiring change in mcp_server.rs/cli.rs, not data migration."
---

# Topic: Ingest mechanism — delivery pattern from AE to mengdie

## Current Status
**CONVERGED Round 2.** Push-primary; watcher.rs library kept but unwired; cold-start via cmd_import.

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | 4 push-primary (codex, ai-engineer, minimal-change, system-architect), 1 hybrid (gemini), 1 pull (challenger). archaeologist: watcher zero call sites, push only wired |
| 2 | converged | 7/7 push-primary. challenger updated on resolves-atomicity design-merit. gemini reverted hybrid. Watcher kept as opt-in library, not wired. |

## Context
How does AE pipeline output (plan.md / review.md / conclusion.md /
retrospect.md) reach mengdie's storage?

The choice shapes the AE ↔ mengdie integration contract, what runs
where (in the AE skill at write-time vs. in mengdie's process), and
what failure modes are visible.

mengdie's v0.x had a notify-based watcher library that was never
wired to a daemon — pull infrastructure exists at the library level
but has zero production miles. The MCP server already exposes a
`memory_ingest` tool capable of accepting structured input — push
infrastructure exists at the tool level and is exercised by the CLI
import path. Both directions are technically reachable from current
code; the question is which one becomes the v0.0.1 contract.

**Design space (not exhaustive — Round 1 may identify more):**
- **Push** — AE skill explicitly calls `memory_ingest` after each
  pipeline phase produces output. Synchronous, mirrors how MCP
  tools are normally driven.
- **Pull** — mengdie watcher daemon over `docs/` (or AE output
  dir). Asynchronous, decoupled from AE process lifecycle.
- **Hybrid** — both active with a designated primary (e.g.,
  push-primary, pull-fallback for environments where AE isn't
  running, or to recover from missed pushes).
- **Event-driven alternative** — message queue / event bus
  between AE and mengdie. Adds a third process surface; almost
  certainly out of scope for v0.0.1 but should be acknowledged
  rather than walled off.

This decision affects:
- AE plugin's per-skill responsibilities (does ae:work explicitly
  call memory_ingest after each commit, or does the file just land
  on disk?)
- Whose process owns the ingest pipeline (synchronous vs background)
- How errors surface (push: caller sees them; pull: daemon must log)
- Cold-start replay semantics (pull naturally replays; push needs a
  bulk-import path)

## Constraints
- mengdie is single-binary stdio MCP server; long-running watcher
  daemon adds a second deployment surface (process supervision,
  restart-on-crash, log rotation)
- AE plugin is the only ingest source v0.0.1 commits to (per topic 4
  ratification — if topic 4 revises this, topic 1 reopens)
- AE skills run inside Claude Code subprocesses; calling MCP tools
  mid-skill is the established pattern
- Storage is global `~/.mengdie/db.sqlite`; ingest must work
  identically regardless of which working directory the AE skill ran
  from
- Whatever is chosen must work for both online (live AE pipeline
  events) and bulk-import (cold-start: existing `docs/` content)

## Key Questions
- What are the failure modes of each, and which are easier to
  observe / debug from the operator's seat?
- What is the cost of building reliable bulk-import on top of a
  push-only design? (Cold start matters: mengdie needs a way to
  ingest pre-existing AE output without the AE plugin re-running.)
- If push wins: what's the minimum AE-plugin-side change to wire
  it up reliably, and which skill(s) own the call?
- If pull wins: how is the watcher daemon supervised, and what
  detects when it has silently stopped?
- Is hybrid reasonable (push as primary, watcher library kept as
  opt-in fallback for environments where AE isn't running) or does
  carrying both indefinitely just double maintenance?
- For event-driven alternatives (queue/bus): is there a v0.0.1-scoped
  benefit, or is this clearly post-v0.0.1 territory?
- What does prior art (mem0 v1.0 async write path, LangMem
  ReflectionExecutor, Graphiti MCP) actually do — and is there a
  pattern that maps cleanly onto AE+mengdie?
