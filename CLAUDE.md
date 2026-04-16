# CLAUDE.md

## What This Is

Mengdie (梦蝶) — AI-native knowledge memory for development workflows. Named after Zhuangzi's butterfly dream (庄周梦蝶).

Core loop: AI tools produce knowledge → Mengdie ingests and filters → feeds context back to AI tools → better output → richer knowledge → spiral upward.

## Architecture

- **Delivery**: MCP server (stdio), registered in Claude Code's `~/.claude/settings.json`
- **Storage**: `~/.mengdie/db.sqlite` (global, per-project search via git-inferred project_id)
- **Ingestion**: AE pipeline file watcher library (conclusion.md, review.md, plan.md, retrospect.md) — library ready, daemon integration deferred to Phase 2
- **Feedback**: ae:analyze post-research injection (Round 0 with provenance)
- **Filtering**: Dreaming (frequency + relevance scoring, daily promotion pass)
- **Contradiction**: Entity-tag directed comparison + temporal validity (valid_from/valid_until)
- **Search**: Hybrid FTS5 + vector similarity, merged via Reciprocal Rank Fusion (RRF)

## Key Design Decisions

See `docs/discussions/` for full context:

1. **MCP server, not plugin** — zero dependency on OpenClaw or any specific AI tool
2. **AE output is primary ingestion source** — highest signal-to-noise ratio, already structured
3. **Post-research injection** — avoid anchoring bias; agents research independently first
4. **Non-silent feedback** — Round 0 block shows what was injected with provenance
5. **Global storage, per-project default search** — avoid migration cost when adding cross-project later
6. **No AI judgment for cold start** — batch import AE discussions directly, avoid error amplification
7. **Entity-tag + temporal validity** — handle decision evolution, not just contradiction
8. **Agent-centric tech stack** — code written by AI agents; optimize for compiler guardrails, not human ergonomics

## Tech Stack

- **Rust** — strictest compiler guardrail for agent-written code, single binary, sub-5ms startup
- **SQLite**: `rusqlite` with `features = ["bundled", "load_extension"]` (FTS5 included via bundled SQLite)
- **Vector search**: App-level brute-force cosine similarity (sqlite-vec deferred; no VectorStore trait)
- **MCP SDK**: `rmcp` v1.3 with `features = ["server", "macros", "transport-io"]`
- **Async**: `tokio` (full features)
- **Embeddings**: `fastembed` v5 — local ONNX Runtime, all-MiniLM-L6-v2 (384d, ~90MB model, 2-10ms inference)
- **FS watcher**: `notify` v8
- **CLI**: `clap` v4

See `docs/discussions/003-tech-stack/conclusion.md` for full rationale.

## Project Structure

```
src/
  core/              # Shared library (DB, search, ingestion, dreaming, contradiction)
    db.rs            # SQLite connection, schema, migrations
    schema.rs        # Table definitions, FTS5 setup
    project.rs       # project_id inference from git remote
    embeddings.rs    # fastembed-rs integration, metadata-in-chunk encoding
    vector.rs        # Brute-force cosine similarity search (sqlite-vec deferred)
    search.rs        # Hybrid FTS5 + vector + RRF merge, score normalization
    parser.rs        # YAML frontmatter extraction, entity extraction from tags
    watcher.rs       # notify-based AE file watcher
    ingest.rs        # Watcher → parser → embed → store pipeline
    contradiction.rs # Entity-tag overlap + temporal validity checks
    dreaming.rs      # Promotion logic (recall_count + avg_relevance)
    mcp_tools.rs     # MCP tool implementations (search, ingest, invalidate)
    metrics.rs       # Observability counters
    mod.rs
  bin/
    mcp_server.rs    # stdio MCP entry point (mengdie-mcp, spawned by Claude Code)
    cli.rs           # CLI entry point (mengdie dream, import, search, stats)
  lib.rs
tests/
  e2e.rs             # End-to-end smoke tests
resources/
  com.mengdie.dream.plist  # macOS launchd template for daily Dreaming
```

## Development

```bash
cargo build              # Build debug
cargo build --release    # Build release (single binary)
cargo test               # Run all tests
cargo clippy             # Lint
```

Key conventions:
- All logging via `tracing` → stderr (never stdout — stdio is MCP transport)
- fastembed inference is sync/blocking — wrap in `tokio::task::spawn_blocking`
- DB connection shared via `Arc<Mutex<Connection>>`
- Embedding model (~90MB) downloaded on first run, cached at `~/.cache/fastembed/`

## rmcp MCP Server Patterns

rmcp v1.3 — the official Rust MCP SDK. Patterns used in this project (from tests/common/calculator.rs):

```rust
use rmcp::{ServerHandler, tool_router, tool,
    handler::server::{router::tool::ToolRouter, wrapper::{Parameters, Json}},
    model::{ServerCapabilities, ServerInfo}, schemars};

// 1. Struct with tool_router field
struct MyServer {
    tool_router: ToolRouter<Self>,
}

// 2. Params: Deserialize + schemars::JsonSchema; Output: Serialize + schemars::JsonSchema
#[derive(Deserialize, schemars::JsonSchema)]
struct SearchParams { query: String }

// 3. Tools via #[tool_router] + #[tool]
#[tool_router]
impl MyServer {
    #[tool(name = "memory_search", description = "Search memories")]
    async fn search(&self, Parameters(p): Parameters<SearchParams>) -> String {
        "result".to_string()  // or Json<T> for structured output
    }
}

// 4. Constructor calls Self::tool_router()
impl MyServer {
    fn new() -> Self { Self { tool_router: Self::tool_router() } }
}

// 5. Implement ServerHandler — only get_info() required
impl ServerHandler for MyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("description")
    }
}

// 6. Start stdio server
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = MyServer::new();
    let transport = rmcp::transport::io::stdio();
    let service = rmcp::serve_server(server, transport).await?;
    service.waiting().await?;
    Ok(())
}
```

Key notes:
- `Parameters<T>` extracts tool input; `Json<T>` wraps structured output
- Tools can be `async fn` — use `spawn_blocking` for fastembed
- `ServerHandler` trait: only `get_info()` required; `#[tool_router]` provides tool dispatch via the `tool_router` field
- Cargo features: `server`, `macros`, `transport-io`

## Review Rules

- **OK (unconditional)** — confirmed no issue. No tracking needed.
- **OK with caveat** ("for MVP", "at scale", "if X happens") — has an implicit "but". Goes to `docs/backlog/` with explicit trigger condition.
- **Warning/Block + defer** — goes to `docs/backlog/` with trigger condition.
- **Warning/Block + fix now** — fix immediately.

The test: **does the finding contain "but"?** If yes → backlog. If no → done.
Backlog items always have: what to do, why it matters, when to revisit (trigger).

## Project Status

Phase 1 complete — MVP built, validated, and iterating. At validation gate (2-week forced-use scorecard pending).

**Completed plan cycles** (all reviewed PASS):
1. `docs/plans/001-mvp-phase1.md` — Core MVP (MCP server, search, ingest, dreaming, contradiction)
2. `docs/plans/002-close-the-loop.md` — AE integration (knowledge capture, watcher library, ae:analyze injection)
3. `docs/plans/003-phase-1.1.md` — API contract correctness + skill wiring (enums, Phase C capture)
4. `docs/plans/004-search-quality-fixes.md` — Dreaming threshold + FTS5 tokenization

**Next step**: 2-week forced-use validation scorecard (from discussion 013) — not more features.

**Deferred discussions** (findings in `docs/backlog/004-analyze-findings.md` with trigger conditions):
006 (SQLite concurrency), 007 (embedding models), 009 (dreaming tuning), 010 (cross-project), 011 (MCP API design)

**Backlog**: `docs/backlog/` (4 files — review deferred items, analyze findings, qmd learnings)
