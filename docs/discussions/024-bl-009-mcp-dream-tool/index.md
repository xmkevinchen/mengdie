---
id: "024"
title: "BL-009: MCP Dream Tool — synthesis loop inside a Claude session"
status: active
created: 2026-04-27
pipeline:
  analyze: skipped
  discuss: done
  plan: pending
plan: ""
tags: [bl-009, mcp-tool, dream, synthesis, in-session, claude-mediated, v0.9.0]
---

# BL-009: MCP Dream Tool — synthesis loop inside a Claude session

The 6-line stub at `docs/backlog/005-phase2-roadmap.md:66-71` proposes a
`memory_dream` MCP tool: run decay + promote + cluster, return clusters
to Claude, Claude synthesizes inline and calls `memory_ingest`. This
discussion converts the stub into a concrete design before
`/ae:roadmap plan v0.8.5` (per discussion 023's conclusion: BL-009
sequencing gate before sprint commit).

## Topics

| # | Topic | File | Status | Decision |
|---|-------|------|--------|----------|
| 1 | BL-009 design — whether + what | [topic-01-bl-009-design/](topic-01-bl-009-design/) | converged | Build in v0.9.0 as McpSessionProvider + thin memory_dream tool; construction-time capability check + ClaudeCliProvider fallback |

## Documents
- [Framing](framing.md)
- [Conclusion](conclusion.md) *(after discussion complete)*
