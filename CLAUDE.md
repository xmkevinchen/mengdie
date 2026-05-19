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

**v0.0.1 cut 2026-05-10**: minimum-viable AE-brain shipped on
`feature/v0.0.1-rebuild` (73 commits ahead of pre-rebuild `main`).
Theme: narrow OSS adoption, keep working in-house code (per
`docs/v0.0.1-rebuild-plan.md` thesis + `docs/discussions/026-rust-oss-
survey/analysis.md` 14-library scorecard).

**Phase 1 history** (kept for context — these are v0.x plans before
the v0.0.1 rebuild branch existed; the code they describe is part of
the "keep in-house" substrate):

1. `docs/plans/001-mvp-phase1.md` — Core MVP (MCP server, search, ingest, dreaming, contradiction)
2. `docs/plans/002-close-the-loop.md` — AE integration (knowledge capture, watcher library, ae:analyze injection)
3. `docs/plans/003-phase-1.1.md` — API contract correctness + skill wiring
4. `docs/plans/004-search-quality-fixes.md` — Dreaming threshold + FTS5 tokenization
5. `docs/plans/005-project-naming.md` — Human-readable project_id
6. `docs/plans/007-llm-provider-claude-cli.md` (BL-005) — LlmProvider trait + ClaudeCliProvider
7. `docs/plans/008-ci-pipeline-and-lint-debt.md` — Clippy cleanup + Forgejo CI
8. `docs/plans/009-embedding-clustering.md` (BL-006) — Seed-neighborhood cosine clustering
9. `docs/plans/010-dream-synthesis.md` (BL-007) — `mengdie dream --synthesize`

Plan 006 (dream MVP) was superseded by 007/009/010 split (`status: cancelled`).

**v0.0.1 ship contents** (feature dirs, all `status: done`):

| Feature | Scope |
|---|---|
| F-001 | sqlite-vec compatibility spike (BL-026 prereq) |
| F-002 | Persisted domain audit + link tables (`memory_search_audit`, `audit_returned_facts`) |
| F-003 | Retrieval & ingest layer consolidation (free-fn refactor; `memory_search_audited` orchestrator) |
| F-004 | Project doc structure overhaul |
| F-005 | `mengdie audit-stats` CLI subcommand (shipped as `audit-stats` not `doctor`) |
| F-006 | sqlite-vec adoption replaces `vector.rs` brute-force |
| Plan 019 | synthesis CLI `--json-schema` adoption (BL-027 Path B) |

**v0.0.1 thesis** (carried from operator clarification 2026-05-05):

> v0.0.1 的目标就是要有个最小可能用的，但避免以后自己重复造轮子的 AE 大脑.
> *(Minimum-viable AE-brain that avoids re-inventing wheels in future.)*

OSS adoption outcomes (per 026 analysis verdicts):

- **Kept all working in-house code** as planned (fastembed-rs / FTS5 /
  db.rs / schema.rs / ingest.rs / mcp_tools.rs / parser.rs /
  dreaming.rs / clustering.rs / synthesis.rs main pipeline /
  contradiction.rs / llm.rs::ClaudeCliProvider / F-002 audit
  substrate). Karpathy "don't refactor things that aren't broken"
  honored.
- **OSS adopted**: `sqlite-vec` v0.1.9 (F-006); `synthesis.rs` JSON
  parser replaced with **claude-CLI `--json-schema`** (Path B) — NOT
  `rig::Extractor` (Path A spike failed 2026-05-08, post-spike
  re-investigation found CLI-native flags worked). `rig::Extractor`
  re-evaluation deferred to BL-039 with code-artifact tripwire.
- **Rejected (026 analysis)**: swiftide / Qdrant / candle / arroy /
  duckdb-rs / mistral.rs / ollama-rs / community Anthropic clients.
- **Deferred-with-trigger** (post-v0.0.1): LanceDB (corpus >100k OR
  p95 vector latency >50ms), Tantivy (multilingual F1 <0.7 OR corpus
  >5M tokens), `rig::Extractor` re-evaluation (BL-039 — fires on
  second `LlmProvider` impl or non-claude LLM SDK dep landing).

**Cargo.toml net delta**: +1 line (`sqlite-vec = "0.1.9"`). Within
the +1~+3 budget set at plan time. BL-027 Path B added 0 deps (CLI
flags only, no rig adoption).

**Plan 019 retrospective findings** (recorded in
`docs/reviews/019-synthesis-cli-json-schema.md`):

- **R1**: plan-review must run actual API probes for provider-specific
  schema assumptions (`oneOf`/`anyOf`/`allOf`/`const`/
  `additionalProperties:false`/conditional required/wrapper shape/
  error-shape). Citation alone OK only for plain object schemas. The
  9-reviewer panel missing Anthropic's input_schema subset on Plan
  019 cost 6 commits + ~400 LoC before mid-execution schema
  redesign. Future plan-reviews should fold this as a gate.
- **R2**: reject path-out-of-scope arguments framed by phantom metered
  cost when operator runs on flat-fee subscription (Claude Code Pro).
- **R3**: anchor deferred-decision BL triggers to code artifacts
  (build_provider arm count / impl LlmProvider count / Cargo.toml
  deps), not vague human-readable external events.

mengdie's role unchanged: **AE 的大脑** (serves AE plugin first;
post-v1 generic). AE plugin handles in-session LLM-driven processing
(Karpathy LLM-wiki style); mengdie receives AE-distilled propositional
facts + does retrieval + does on-demand reflection ("自成长" via
meta-fact abstraction).

**Phase 0 research closed** (per `docs/v0.0.1-rebuild-plan.md` — kept
for audit trail of decisions feeding v0.0.1 ship):

| Item | Outcome |
|------|---------|
| 1. OSS library survey (14 candidates) | `docs/discussions/026-rust-oss-survey/analysis.md` scorecard. ADOPT × 2 (sqlite-vec, --json-schema-via-CLI). DEFER × 3 (LanceDB, Tantivy, rig::Extractor). SKIP × 9. |
| 2. mengdie ↔ AE integration | `docs/discussions/027-industry-state-2026/conclusion.md` — push-primary, watcher.rs as opt-in library, `cmd_import` for cold-start. |
| 3. Reflection mechanism | `027 T2` — on-demand default + `ReflectionTrigger` trait; salience/composite/debounced filed as deferred BL-024. |
| 4. Storage shape for facts + meta-facts | `028 conclusion` — split-table (F-002 link table shipped). |

**Branch state at v0.0.1 ship**:
- 73 commits ahead of pre-rebuild `main` (2026-04-30 → 2026-05-10).
- `src/` net ~2-3K LoC across 7 features (sqlite-vec replaces ~264
  LoC brute-force; F-002 audit ~600 LoC new; F-005 audit-stats CLI
  ~400 LoC; plan 019 ~400 LoC LLM provider + schema; doc/test the
  rest).
- 13 deferred BLs (`docs/backlog/unscheduled/BL-029 ~ BL-041`), all
  trigger-annotated.

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
