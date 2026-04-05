---
round: 3
date: 2026-04-04
score: converged
---

# Round 3 — Agent-Centric Reframe

## Trigger
User corrected a fundamental assumption: coding is done by AI agents, not humans. This invalidated all human-ergonomics arguments from Rounds 1-2 (iteration speed, cognitive overhead, type ceremony, compile-time frustration, IDE comfort).

## What Changed

### Arguments invalidated by agent-coding reframe:
- "Python iterates faster on heuristics" — agents don't iterate like humans
- "TypeScript type ceremony is overhead" — agents write types instantly
- "Rust compile times slow a solo dev" — agents don't mind waiting
- "IDE autocomplete / AI-assisted dev" — Claude Code works in all languages

### Arguments that became primary:
- Runtime performance (startup latency, throughput)
- Cross-platform distribution (macOS + Linux + CI)
- Team scale (Phase 3: concurrent MCP connections)
- Compiler as guardrail for agent-generated code
- Native module fragility across platforms
- Dependency footprint and supply chain

### Go eliminated
Official `modelcontextprotocol/go-sdk` truly just released (March 2026, ~4 weeks old). Pre-v1.0. Too immature for a project where MCP is the cornerstone delivery mechanism.

### Rust re-admitted
User challenged Rust's elimination. Valid: official `modelcontextprotocol/rust-sdk` at v0.16.0 (16 releases of iteration). Many Rust crates (tokio, serde) stay 0.x while being production-stable. MCP protocol is stable JSON-RPC over stdio — SDK is convenience layer, not protocol itself.

## Discussion

### Round 3a: Agent-centric evaluation (TS vs Python vs Go vs Rust)
- Architect: TypeScript (60/40 over Go) — SDK maturity is dominant criterion
- Challenger: Go > Python > TypeScript > Rust — pure-Go SQLite, single binary
- Codex: Leans Go — wins deployment, cross-platform, ops, supply chain
- Gemini: TypeScript — prior Python args invalidated; compile-time types help agents

### User constraint: "MCP is the cornerstone"
Go/Rust initially re-eliminated (pre-v1.0 SDKs). User challenged: "Why is Rust out? Isn't it the fastest?" — v0.16.0 is 16 releases deep, protocol is stable, Rust has the strongest agent guardrails.

### Round 3b: Final 3-way (TS vs Python vs Rust)
All 4 agents converged on **Rust** (4-0 unanimous):

- **Architect**: "Rust's compiler is the strongest guardrail — agents generate code that either compiles correctly or doesn't compile at all. rusqlite --features bundled eliminates the entire better-sqlite3 prebuild dependency chain."
- **Challenger**: "The compiler catches entire classes of bugs that neither TypeScript's runtime nor Python's Pydantic reach, and unlike TypeScript's any-escape-hatch, Rust's guarantees are not opt-out-able by an agent."
- **Codex**: "Agent-authored code benefits most from maximal compile-time rejection of invalid states. Single binary eliminates environment drift as a failure class."
- **Gemini**: "Rust's compiler enforces correctness guarantees that agents cannot violate. Single binary + bundled deps + sub-5ms spawn = deployment and reliability story that scales to team phase."

## Outcome
- Score: converged
- Decision: Rust
- Prior TypeScript decision overturned by agent-centric reframe + MCP-as-cornerstone constraint
