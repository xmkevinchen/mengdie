# CLAUDE.md

## What This Is

AI-native Second Brain — a knowledge management layer for AI development workflows.

Core loop: AI tools produce knowledge → Second Brain ingests and filters → feeds context back to AI tools → better output → richer knowledge → spiral upward.

## Architecture

- **Delivery**: MCP server (stdio), registered in Claude Code's `~/.claude/settings.json`
- **Storage**: `~/.second-brain/db.sqlite` (global, per-project search via git-inferred project_id)
- **Ingestion**: AE pipeline file watcher (conclusion.md, review.md, plan.md, retrospect.md)
- **Feedback**: ae:analyze post-research injection (Round 0 with provenance)
- **Filtering**: Simplified Dreaming (frequency + relevance scoring, daily promotion pass)
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
- **SQLite**: `rusqlite` with `features = ["bundled", "fts5", "load_extension"]`
- **Vector search**: `sqlite-vec` (optional, behind `VectorStore` interface); app-level cosine as primary fallback
- **MCP SDK**: `rmcp` v0.16 with `features = ["server", "macros", "transport-io"]`
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
    vector.rs        # VectorStore trait, cosine fallback, sqlite-vec optional
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
    mcp_server.rs    # stdio MCP entry point (spawned by Claude Code)
    cli.rs           # CLI entry point (dream, import, search, stats)
  lib.rs
tests/
  e2e.rs             # End-to-end smoke tests
resources/
  com.second-brain.dream.plist  # macOS launchd template for daily Dreaming
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

- All review findings without immediate action go to `docs/backlog/` — never silently dropped
- "Defer", "Note", "OK with caveat" = backlog item with a revisit trigger
- Backlog items have an explicit trigger condition ("revisit when X happens")

## Project Status

Phase 1 MVP — plan reviewed, ready for implementation.

- Plan: `docs/plans/001-mvp-phase1.md` (8 steps, 15 acceptance criteria)
- Discussions: `docs/discussions/002-mvp-phase1/` (scope) + `003-tech-stack/` (Rust)
- Backlog: `docs/backlog/001-qmd-learnings.md` (RRF, score normalization, metadata-in-chunk)
