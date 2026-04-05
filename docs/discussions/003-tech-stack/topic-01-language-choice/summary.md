---
id: "01"
title: "Language & runtime choice"
status: converged
current_round: 3
created: 2026-04-04
decision: "Rust with rusqlite (bundled SQLite+FTS5), rmcp v0.16.0, sqlite-vec behind VectorStore interface"
rationale: "Agent-centric: strictest compiler guardrail (non-opt-outable); single binary distribution (zero runtime deps, true cross-platform); sub-5ms startup (60-100x faster than Node/Python); bundled SQLite eliminates native module fragility; strongest local embedding + distributed sync ecosystem for Phase 2-3"
reversibility: "high"
reversibility_basis: "~1000 line project; MCP protocol is stable JSON-RPC regardless of language"
---

# Topic: Language & runtime choice

## Current Status
Converged — Rust selected after 3 rounds (2 rounds human-centric → TypeScript; Round 3 agent-centric reframe → Rust unanimous 4-0).

## Round History
| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | Go/Rust eliminated (human-centric); TypeScript vs Python |
| 2 | converged (overturned) | TypeScript (4-1); Challenger conceded; Gemini held Python |
| 3 | converged | Agent-centric reframe: Rust (4-0 unanimous). TypeScript overturned. |
