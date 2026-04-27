# CLAUDE.md

## Language Conventions

- **Chat / conversation responses**: Chinese (中文).
- **Git-tracked artifacts** (anything that ends up in a commit — `docs/`,
  `src/`, `tests/`, `CHANGELOG.md`, `README.md`, plans, reviews,
  conclusions, commit messages, code comments, etc.): English.
- **Non-archived working files** (anything gitignored — `.ae/backlog/`,
  `.ae/discussions/`, `.ae/roadmaps/`, `.ae/plans/` if any, scratchpads,
  local notes): Chinese (中文).

The boundary is "does it leave my machine via git." Tracked → English.
Untracked → Chinese. Spoken word → Chinese.

## What This Is

Mengdie (梦蝶) — AI-native knowledge memory for development workflows. Named after Zhuangzi's butterfly dream (庄周梦蝶).

Core loop: AI tools produce knowledge → Mengdie ingests and filters → feeds context back to AI tools → better output → richer knowledge → spiral upward.

## Architecture

- **Delivery**: MCP server (stdio), registered in Claude Code's `~/.claude/settings.json`
- **Storage**: `~/.mengdie/db.sqlite` (global, per-project search via git-inferred project_id)
- **Ingestion**: AE pipeline file watcher library (conclusion.md, review.md, plan.md, retrospect.md) — library ready, daemon integration deferred to Phase 2
- **Feedback**: ae:analyze post-research injection (Round 0 with provenance)
- **Filtering**: Dreaming — (1) frequency + relevance scoring with daily promotion pass, and (2) LLM-driven synthesis that clusters related memories and asks the model (via the claude CLI) to consolidate each cluster into a single synthesis memory
- **LLM provider**: Trait-based abstraction with a claude-CLI-subprocess implementation (shells out to `claude -p` and streams stdout); credentials delegated to the CLI (mengdie never touches secrets)
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
    dreaming.rs      # Promotion logic (recall_count + avg_relevance) + async LLM synthesis pass
    clustering.rs    # Seed-neighborhood cosine clustering (BL-006; feeds dream synthesis)
    synthesis.rs     # Pure prompt builder + brace-depth JSON parser for dream synthesis (BL-007)
    llm.rs           # LlmProvider trait + ClaudeCliProvider subprocess impl (BL-005)
    config.rs        # MengdieConfig — nested [llm] + [llm.claude_cli] TOML loader
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

**After every fresh clone**, enable project git hooks once:

```bash
git config core.hooksPath .githooks
```

The `.githooks/pre-commit` hook runs `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` (not `cargo test` — that's CI's job). See `.githooks/README.md` for details. `--no-verify` is NOT a normal escape hatch — fix the issue, don't skip.

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

**Sprint-plan discipline** (from discussion 021 Topic 4): before running
`/ae:roadmap plan v<ver>`, skim candidate BL bodies for explicit "not now" /
"filed for trigger" language. `/ae:roadmap remove` such items before
sprint-commit. Avoids repeating the v0.8.0 pattern where 2 defer-trigger
BLs got committed and had to be retroactively removed at close time. When
the upstream AE `admission_status: defer-until-trigger` feature ships (see
`../agentic-engineering/.ae/backlog/unscheduled/BL-admission-status-defer-until-trigger.md`),
mark new BLs whose body says "not now" with that frontmatter field.

## Project Status

Phase 1 complete, Phase 2 in progress. The intelligence layer (LLM synthesis
built on a clustering + provider primitive) shipped mid-April 2026; first
real `mengdie dream --synthesize` pass landed 13 syntheses against the
production DB (empirical results in `docs/backlog/BL-clustering-validation.md`).

**Completed plan cycles** (all reviewed PASS unless noted):

1. `docs/plans/001-mvp-phase1.md` — Core MVP (MCP server, search, ingest, dreaming, contradiction)
2. `docs/plans/002-close-the-loop.md` — AE integration (knowledge capture, watcher library, ae:analyze injection)
3. `docs/plans/003-phase-1.1.md` — API contract correctness + skill wiring (enums, Phase C capture)
4. `docs/plans/004-search-quality-fixes.md` — Dreaming threshold + FTS5 tokenization
5. `docs/plans/005-project-naming.md` — Human-readable project_id (survives git remote changes)
6. `docs/plans/007-llm-provider-claude-cli.md` (BL-005) — LlmProvider trait + ClaudeCliProvider (first of the Phase 2 intelligence primitives)
7. `docs/plans/008-ci-pipeline-and-lint-debt.md` — Clippy cleanup + local pre-commit hooks + Forgejo CI (shipped as fmt-only; clippy+test deferred via `BL-ci-full-clippy-test`)
8. `docs/plans/009-embedding-clustering.md` (BL-006) — Seed-neighborhood cosine clustering
9. `docs/plans/010-dream-synthesis.md` (BL-007) — `mengdie dream --synthesize`, the first caller of BL-005 + BL-006; first real run produced 13 syntheses

Plan 006 (dream MVP) was superseded by the 007/009/010 split and is `status: cancelled`.

**Next step (current — 2026-04-27)**: **v0.x frozen at v0.8.0; v1.0
rebuild planning**. v0.8.5 sprint cancelled (all 4 items archived);
all 13 BLs archived to `.ae/backlog/closed/v0.x-superseded-by-v1/`.

Strategic reframe (chat 2026-04-27): mengdie's v0.x is reinventing
mature Rust infrastructure (swiftide / rig / Qdrant / LanceDB) and
solving "production data is precious" problems that don't apply
(data is reproducible from `docs/`). New thesis: mengdie = AE 的大脑
(serves AE plugin first, post-v1 generic), AE plugin handles
in-session LLM-driven processing (Karpathy LLM-wiki style), mengdie
receives AE-distilled propositional facts as ingest input + does
retrieval + does on-demand reflection ("自成长" via meta-fact
abstraction).

**Phase 0 research items** (do BEFORE filing v1 BLs, see
`docs/v1-rebuild-plan.md`):
1. Survey open-source Rust libraries (swiftide, rig, Qdrant, LanceDB,
   sqlite-vec, Tantivy) — what each does, what mengdie reimplements.
2. Per-library role + integration strategy.
3. mengdie ↔ AE integration design (push pattern A vs B).
4. Reflection mechanism (depends on item 2).

Three deferred open questions: reflection trigger, meta-fact
confidence, single-table vs split-table.

`feature/v1-rebuild` branch exists but empty. Will resume when Phase 0
research completes and Phase 1 BLs filed. Until then: no code changes
to `src/`. v0.8.0 continues to run for operator use; receives no new
development.

**Advisory rule for closing plans**: when `/ae:work` completes all plan
checkboxes, the completion commit must also update the parent discussion's
`status:` and `pipeline.work:` frontmatter — otherwise the dashboard and
`/ae:next` see phantom-active discussions. See
upstream AE plugin backlog (`../agentic-engineering/.ae/backlog/unscheduled/BL-038-work-closes-parent-discussion.md`) for the real fix.

**Backlog**: `docs/backlog/` — see the directory for the canonical,
trigger-annotated list. Formerly-deferred discussions (006 SQLite
concurrency, 007 embedding model tradeoffs, 009 dreaming tuning, 010
cross-project, 011 MCP API design) remain `status: deferred` in
`docs/discussions/`; their findings are already captured in
`docs/backlog/004-analyze-findings.md` with trigger conditions.
