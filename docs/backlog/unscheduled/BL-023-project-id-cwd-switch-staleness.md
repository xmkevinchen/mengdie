---
id: BL-023
title: "project_id is inferred once at MCP server startup — stale for operators who switch projects without restarting"
status: open
created: 2026-05-05
origin: "discussion 027 archaeologist round-01 verification (mcp_server.rs:32-34)"
trigger: "Operator runs MCP server across multiple project working directories within a single session AND observes search results scoped to the wrong project"
depends_on: []
size: S
v_target: "v0.0.1 — independent of T3 ratify decision"
---

# BL-023 — project_id cwd-switch staleness

## Origin

Surfaced in discussion 027 by `archaeologist` (Round 1, reproduced in `round-02/archaeologist.md`):

> "project_id staleness is real (`mcp_server.rs:32-34`, one-time startup inference) but not decisive for T3."

The MCP server infers `project_id` once at startup from the cwd's git context. If the operator launches `mengdie-mcp` in project A and later their AE skill invokes it from project B's directory (without restarting the MCP server), all subsequent ingest and search calls are scoped to project A.

## Why this matters

Independent of discussion 027 Topic 3's ratify decision (per-project default scope ratified). The bug exists regardless:

- Per-project default → search results are scoped to the wrong project (silently)
- Cross-project default (counterfactual; not chosen) → bug becomes irrelevant

Per Topic 3 conclusion, AE skills should specify `scope` per-skill explicitly. That partially mitigates the search side. The ingest side remains affected — facts from project B's pipeline output get tagged with project A's `project_id`.

## Implementation sketch

Two viable paths:

- **Lookup-on-call**: re-infer `project_id` from each MCP tool call's context (e.g., `cwd` of the AE skill that invoked the tool). Pros: always correct. Cons: requires plumbing the caller's cwd into the MCP request schema; rmcp may not expose this directly.
- **Explicit per-call parameter**: add `project_id: Option<String>` to `SearchParams` and `IngestParams`. Caller (AE skill) supplies it from its own context. Pros: clean schema; matches Topic 3's "AE skills specify scope explicitly". Cons: every caller must compute and pass the project_id.

Explicit per-call parameter aligns with the Topic 3 decision's directional preference for explicit scope per AE skill.

## Acceptance criteria

- MCP tools accept an explicit `project_id` parameter (with fallback to startup inference for backward compat)
- Documented as part of Topic 1's AE plugin wiring (per-skill `memory_ingest` BL)
- `cmd_import` (cli.rs:361) re-infers `project_id` per directory walk, not at startup

## Trigger

Fires when:
- Topic 1's AE-plugin per-skill `memory_ingest` wiring lands (this BL is a natural co-commit)
- Operator observes a project-A search result while running in project B (manual reproduction)
