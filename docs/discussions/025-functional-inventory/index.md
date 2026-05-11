---
id: "025"
title: "v0.0.1 Step A1 — mengdie functional inventory"
status: active
created: 2026-04-27
pipeline:
  analyze: done
  discuss: pending
  plan: pending
  work: pending
plan: ""
parent_plan: "docs/v0.0.1-rebuild-plan.md"
tags: [v0.0.1, inventory, refactor, src-audit, capability-mapping]
---

# v0.0.1 Step A1 — mengdie functional inventory

What does mengdie src/ actually do today, broken down by module, with
explicit capture of:
- input/output contracts of each module
- which behaviors are pure logic (portable to v0.0.1) vs which are
  baked-in v0.x assumptions (e.g., "production data is precious",
  app-level brute-force vector search, hand-rolled clustering)
- empirical evidence of usage (CLI commands wired up, MCP tools
  exposed, tests covering each path)

Step A1 of the v0.0.1 redesign migration outline. Pairs with A2
(Rust open-source library survey) — together they feed Step B
(integration strategy `/ae:discuss`).

## Topics
*Created by `/ae:discuss`*

## Documents
- [Analysis](analysis.md)
