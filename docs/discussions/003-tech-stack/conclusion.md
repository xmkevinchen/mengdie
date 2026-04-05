---
id: "003"
title: "Tech Stack Selection — Conclusion"
concluded: 2026-04-04
plan: ""
---

# Tech Stack Selection — Conclusion

8 agents across 3 rounds. Round 1-2 converged on TypeScript (human-centric). Round 3 overturned to Rust (agent-centric reframe, 4-0 unanimous).

---

## Decision Summary

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Language & runtime | **Rust** | Agent-centric: strictest compiler guardrail; single binary; sub-5ms startup; bundled SQLite; zero native module fragility; strongest Phase 2-3 ecosystem | high — ~1000 lines, protocol-stable JSON-RPC |

### Specific Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| MCP SDK | `rmcp` v0.16.0 (official `modelcontextprotocol/rust-sdk`) | Pre-1.0 but 16 releases deep; MCP protocol is stable JSON-RPC over stdio |
| SQLite + FTS5 | `rusqlite` with `--features bundled` | SQLite compiled into binary; FTS5 included; zero native module issues |
| Vector search | `sqlite-vec` behind `VectorStore` interface | FTS5 is primary search; vector is re-ranking signal; app-level cosine fallback |
| Embeddings | `fastembed-rs` / `ort` (local ONNX Runtime) | Local-first: 2-10ms inference, no API key, no cost, works offline; ~90MB model bundled. API fallback via `reqwest` if needed. |
| FS watcher | `notify` crate | Used by cargo, deno, rust-analyzer; supports multi-directory watching |
| Async runtime | `tokio` | De facto standard; required by reqwest, axum, rmcp |
| HTTP server (Phase 2) | `axum` | For daemon mode with HTTP/SSE MCP transport |

### Architecture

Two entry points sharing a core library (carried from Doodlestein Round 2):

```
src/
  core/           # DB, search, ingestion, dreaming logic, VectorStore interface
  mcp_server.rs   # stdio entry point (spawned by Claude Code)
  cli.rs          # standalone entry point (cron/launchd for Dreaming + batch import)
```

Evolution path:
- **Phase 1**: stdio per-session + CLI cron for Dreaming
- **Phase 2**: Long-running daemon (axum + notify) watching multiple projects; local embeddings via fastembed-rs
- **Phase 3**: Multi-user via SQLite WAL (2-5 users); PostgreSQL migration if write contention grows; distributed sync via automerge (CRDT, core written in Rust)

### Risk Mitigations

| Risk | Mitigation |
|------|-----------|
| rmcp pre-1.0 API changes | MCP protocol is stable JSON-RPC; SDK is convenience layer; pin version, migrate deliberately |
| sqlite-vec alpha instability | Abstract behind VectorStore interface; FTS5 primary; cosine fallback over float32 blobs |
| Agent stdout corruption | Rust ecosystem uses `tracing` crate → stderr by default; no `console.log` equivalent |
| SQLite single-writer at team scale | WAL mode handles 2-5 users; `busy_timeout` pragma; PostgreSQL migration path via sqlx |

### Vector Storage Format

Embeddings stored as plain `BLOB` column (IEEE 754 little-endian float32 array) with dimension count in separate column. sqlite-vec indexes as virtual layer on top.

---

## Why Rust Won (Round 3 Reframe)

### The Pivotal Correction
User corrected a fundamental assumption in Rounds 1-2: **coding is done by AI agents, not humans.** This invalidated all human-ergonomics arguments:
- "Developer iteration speed" — irrelevant (agents don't iterate like humans)
- "Type ceremony overhead" — irrelevant (agents write types instantly)
- "Compile times slow development" — irrelevant (agents don't mind waiting)
- "IDE autocomplete advantage" — irrelevant (Claude Code works in all languages)

### What Matters for Agent-Written Code
Under agent-centric criteria, Rust wins 6 of 12 dimensions decisively:

| Dimension | Winner | Evidence |
|-----------|--------|----------|
| Ingestion performance | **Rust** | 4,700+ RPS vs 250-880 (Node) |
| Memory footprint | **Rust** | 5-15MB vs 50-80MB (Node) |
| Startup latency | **Rust** | 1-5ms vs 300-500ms (Node) |
| Cross-platform distribution | **Rust** | Single binary, zero deps |
| Local embeddings (Phase 2) | **Rust** | fastembed-rs: 3-5x faster, 60-80% less memory than Python |
| Distributed sync (future) | **Rust** | automerge core IS Rust |

TypeScript/Python win only on MCP SDK stability (v1.x vs v0.16.0) — a narrow advantage when the underlying protocol is stable JSON-RPC.

### Additional User Constraints Evaluated
- **MCP as cornerstone**: Protocol is stable; SDK is convenience. v0.16.0 with 16 releases is acceptable.
- **Daemon/service evolution**: axum + notify + tokio cover Phase 2 daemon architecture natively.
- **Multi-user extensibility**: SQLite WAL for 2-5 users; sqlx for PostgreSQL migration; automerge for distributed sync.
- **Hybrid possibility**: Rust core could expose C FFI, but monolith is simpler for Phase 1.

---

## Eliminated Candidates

| Language | Round 1-2 | Round 3 | Reason |
|----------|-----------|---------|--------|
| TypeScript | Won (4-1) | Lost (0-4) | Agent reframe: native module fragility, 300-500ms startup, stdout corruption risk, no single binary |
| Python | Runner-up | Eliminated | No compile guardrail for agents, GIL at team scale, no binary distribution |
| Go | Eliminated R1 | Eliminated R3 | Official MCP SDK only 4 weeks old (March 2026); too immature for MCP-as-cornerstone |

---

## Doodlestein Review (from Round 2, carried forward)

| Challenge | Resolution | Applies to Rust? |
|-----------|------------|-----------------|
| Dreaming lifecycle (two entry points) | Accepted — mcp_server.rs + cli.rs sharing core/ | Yes, carried forward |
| sqlite-vec abstraction | VectorStore interface + FTS5 primary | Yes, carried forward |
| Cold-start benchmark | Target <200ms | Rust: ~1-5ms, non-issue |
| ESM/CJS interop | Was TypeScript-specific | N/A for Rust |
| Embedding API batch backoff | Applies to all languages | Yes, carried forward |

---

## Team Composition

| Agent | Role | Backend | Rounds |
|-------|------|---------|--------|
| TL | Moderator | Claude | All |
| architect | Architecture design | Claude | All |
| standards-expert | Ecosystem research | Claude | R1-R2 |
| challenger | Risk analysis | Claude | All |
| codex-proxy | Tooling & deployment | Codex | All |
| gemini-proxy | Developer experience | Gemini | All |
| doodlestein-strategic | Strategic improvement | Claude | R2 Doodlestein |
| doodlestein-adversarial | Blind spot detection | Claude | R2 Doodlestein |
| doodlestein-regret | Regret prediction | Claude | R2 Doodlestein |

## Process Metadata
- Discussion rounds: 3 (R1-R2 human-centric, R3 agent-centric reframe)
- Topics: 1 (converged twice — TypeScript then Rust)
- User escalations: 3 (agent-centric correction, MCP-as-cornerstone, Rust re-admission)
- Consensus verification: 1 (R2, TypeScript — later overturned)
- Doodlestein challenges: 3 raised, 3 resolved (R2 — findings carried to Rust conclusion)
- Final vote: 4-0 unanimous for Rust

## Next Steps
→ `/ae:plan` — create implementation plan based on this + [002 MVP Phase 1 conclusion](../002-mvp-phase1/conclusion.md)
→ Day-one: scaffold Rust project with rusqlite --bundled, rmcp, notify, tokio
→ Go migration trigger: if `modelcontextprotocol/go-sdk` reaches v1.0 AND Rust SDK has blocking issues, re-evaluate
