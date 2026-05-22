# CLAUDE.md

Guidance for Claude Code (and other AI assistants) working in this repo.

## What This Is

Mengdie (梦蝶) — AI-native knowledge memory for development workflows. Named
after Zhuangzi's butterfly dream (庄周梦蝶).

Core loop: AI tools produce knowledge → Mengdie ingests and filters → feeds
context back to AI tools → better output → richer knowledge → spiral upward.

## Architecture

- **Delivery**: MCP server (stdio), registered in the host AI tool's MCP
  config (e.g. Claude Code's `~/.claude/settings.json`).
- **Storage**: `~/.mengdie/db.sqlite` (global, per-project search via
  git-inferred `project_id`).
- **Ingestion**: file watcher library plus an MCP `memory_ingest` tool; the
  watcher targets structured markdown artifacts produced by upstream AI
  pipelines (e.g. conclusion / review / plan files).
- **Feedback**: hosts query `memory_search` and inject results with explicit
  provenance — agents see what was pulled and why.
- **Filtering (Dreaming)**: (1) frequency + relevance scoring with a daily
  promotion pass; (2) LLM-driven synthesis that clusters related memories
  and consolidates each cluster into a single synthesis row.
- **LLM provider**: trait-based abstraction with a claude-CLI-subprocess
  implementation (shells out to `claude -p` and streams stdout);
  credentials delegated to the CLI (mengdie never touches secrets).
- **Contradiction**: entity-tag directed comparison + temporal validity
  (`valid_from` / `valid_until`).
- **Search**: hybrid FTS5 + vector similarity, merged via Reciprocal Rank
  Fusion (RRF).

## Key Design Decisions

1. **MCP server, not plugin** — zero dependency on any specific AI tool.
2. **Structured-artifact ingestion is primary** — highest signal-to-noise
   ratio; the artifacts are already filtered by upstream review.
3. **Post-research injection** — avoid anchoring bias; agents research
   independently first, then see prior memory as supplemental context.
4. **Non-silent feedback** — injection blocks show what was pulled with
   provenance, never invisible.
5. **Global storage, per-project default search** — avoids migration cost
   when adding cross-project later.
6. **No AI judgment for cold start** — batch-import existing notes directly,
   avoid error amplification at the seeding step.
7. **Entity-tag + temporal validity** — handles decision evolution, not just
   instantaneous contradiction.
8. **Agent-centric tech stack** — code is written by AI agents; optimize for
   compiler guardrails over human ergonomics.

## Tech Stack

- **Rust** — strictest compiler guardrail for agent-written code, single
  binary, sub-5ms startup.
- **SQLite**: `rusqlite` with `features = ["bundled", "load_extension"]`
  (FTS5 included via bundled SQLite).
- **Vector search**: `sqlite-vec` v0.1.9 (`vec0` virtual table loaded via
  rusqlite's extension API).
- **MCP SDK**: `rmcp` v1.3 with
  `features = ["server", "macros", "transport-io"]`.
- **Async**: `tokio` (full features).
- **Embeddings**: `fastembed` v5 — local ONNX Runtime, all-MiniLM-L6-v2
  (384d, ~90MB model, 2-10ms inference).
- **FS watcher**: `notify` v8.
- **CLI**: `clap` v4.

## Project Structure

```
src/
  core/              # Shared library
    db.rs            # SQLite connection, migrations, helpers
    schema.rs        # Table definitions, FTS5 setup, version migrations
    project.rs       # project_id inference from git remote
    embeddings.rs    # fastembed-rs integration
    vector.rs        # sqlite-vec adapter
    search.rs        # Hybrid FTS5 + vector + RRF merge, score normalization,
                     #   audited orchestrator
    parser.rs        # YAML frontmatter extraction, entity extraction
    watcher.rs       # notify-based file watcher
    ingest.rs        # parse → embed → store pipeline + contradiction check
    contradiction.rs # Entity-tag overlap + temporal validity checks
    dreaming.rs      # Promotion logic (recall_count + avg_relevance) +
                     #   async LLM synthesis pass
    decay.rs         # Exponential-decay re-rank for stale memories
    clustering.rs    # Seed-neighborhood cosine clustering (feeds dreaming)
    synthesis.rs     # Prompt builder + structured-output handling
    lint.rs          # memory_lint health checks
    llm.rs           # LlmProvider trait + ClaudeCliProvider subprocess impl
    config.rs        # MengdieConfig TOML loader
    mcp_tools.rs     # MCP tool implementations
    metrics.rs       # Observability counters
  bin/
    mcp_server.rs    # stdio MCP entry point (mengdie-mcp)
    cli.rs           # CLI entry point (mengdie dream / import / search / ...)
  lib.rs
tests/               # Integration + e2e suites
resources/
  com.mengdie.dream.plist     # macOS launchd template for daily Dreaming
  synthesis-output-schema.json # claude-CLI --json-schema payload contract
```

## Development

```bash
cargo build              # Build debug
cargo build --release    # Build release (single binary)
cargo test               # Run all tests
cargo clippy             # Lint
```

After every fresh clone, enable project git hooks once:

```bash
git config core.hooksPath .githooks
```

The `.githooks/pre-commit` hook runs `cargo fmt --check` +
`cargo clippy --all-targets -- -D warnings` (not `cargo test` — that's CI's
job). `--no-verify` is not a normal escape hatch — fix the issue, don't
skip.

Key conventions:

- All logging via `tracing` → stderr (never stdout — stdio is the MCP
  transport).
- fastembed inference is sync/blocking — wrap in
  `tokio::task::spawn_blocking`.
- DB connection shared via `Arc<Mutex<Connection>>`.
- Embedding model (~90MB) downloaded on first run, cached at
  `~/.cache/fastembed/`.
- All committed artifacts (code, comments, docs, commit messages) are in
  English.

## rmcp MCP Server Patterns

rmcp v1.3 — the official Rust MCP SDK. Patterns used in this project:

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

- `Parameters<T>` extracts tool input; `Json<T>` wraps structured output.
- Tools can be `async fn` — use `spawn_blocking` for fastembed.
- `ServerHandler` trait: only `get_info()` required; `#[tool_router]`
  provides tool dispatch via the `tool_router` field.
- Cargo features: `server`, `macros`, `transport-io`.
