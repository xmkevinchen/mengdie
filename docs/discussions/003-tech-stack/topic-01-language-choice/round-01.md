---
round: 1
date: 2026-04-04
score: pending
---

# Round 1 — Independent Research

## Discussion

### Architect
- **Position**: TypeScript
- Strongest argument: Claude Code ecosystem, MCP SDK is reference impl, "SQLite+FTS5+sqlite-vec+better-sqlite3" proven by Memento project
- Eliminated Go (MCP SDK not stable until ~Aug 2026) and Rust (unnecessary complexity for I/O-bound use case)
- Raised open question: embedding model choice (local vs API) unresolved in conclusion doc

### Standards-Expert
- **Position**: TypeScript
- Verified MCP SDK maturity: TS v1.x stable (7.2K stars), Python stable (13.5K stars), Rust pre-1.0 (v0.16.0), Go just released March 2026
- Key finding: sqlite-vec is alpha (v0.1.7-alpha.2) across ALL languages — language-agnostic risk
- Transformers.js viable for local embeddings in Node, FastEmbed for Python
- chokidar v5 most battle-tested FS watcher (30M dependents)

### Challenger
- **Position**: Python (contrarian) / Rust (for correctness)
- Challenged "ecosystem alignment" as cargo-cult reasoning — MCP is language-agnostic JSON-RPC
- Flagged stdout corruption risk highest in TypeScript (console.log habit, transitive npm deps)
- Documented better-sqlite3 Node v24 breakage (Issues #1376, #1384)
- Python uv solves packaging; FastMCP 3.0 production-grade
- Raised Dreaming cron architecture question (stdio server lifecycle)

### Codex-Proxy
- **Position**: TypeScript (23/25 score)
- Best AI-assisted development, smoothest npm distribution, best IDE tooling
- Dependency management advantage to Rust (vendored SQLite), but TS manageable
- Noted better-sqlite3 needs per-platform prebuilds for distribution (less relevant for personal tool)

### Gemini-Proxy
- **Position**: Python
- Fastest for heuristic-heavy logic (Dreaming scoring, contradiction detection)
- Lowest cognitive overhead and crunch risk for solo dev
- watchfiles (async) better than watchdog for MCP event loop
- Python MCP SDK v1.x stable, pin to avoid v2 migration

## Outcome
- Score: pending (moved to Round 2)
- Consensus: Go and Rust eliminated. TypeScript vs Python needs cross-examination.
