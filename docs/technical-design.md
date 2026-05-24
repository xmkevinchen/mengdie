# Mengdie — Technical Design Document

**Status**: living document
**Last updated**: 2026-05-23
**Audience**: contributors + AI agents reading mengdie's source. For end-user "how to install + use" → see [README](../README.md). For shipped-version changelog → see [CHANGELOG](../CHANGELOG.md).

---

## Part 1 — Goal & Vision

### 1.1 What mengdie is

> Mengdie (梦蝶) — AI-native knowledge memory for development workflows. Named after Zhuangzi's butterfly dream (庄周梦蝶).
>
> AI produces knowledge, knowledge feeds AI — who is the dreamer?

mengdie is a persistent memory layer for AI-assisted development. It stores structured artifacts produced by AI workflows (decisions, reviews, plans, retrospects, synthesis), filters them via a daily promotion pass, and serves them back as searchable context with provenance.

The core loop is a spiral:

```
AI tools produce knowledge → mengdie ingests + filters → feeds context back → richer AI output → richer knowledge → ...
```

### 1.2 Who it's for

- **Primary user**: a single-operator AI-assisted developer running mengdie alongside one or more AI tools (Claude Code, codex CLI, gemini, etc.).
- **Primary integration**: the [Agentic Engineering plugin](https://github.com/xmkevinchen/agentic-engineering) ("AE"). AE provides in-session LLM workflow orchestration; mengdie provides the persistent cross-session memory + filtered context retrieval that closes the loop.
- **Not for**: multi-tenant SaaS, generic full-text search, real-time event streaming, public-facing knowledge bases.

The single-operator scope is load-bearing: many design simplifications (no auth, global SQLite DB, no per-row RBAC, no concurrency-coordination beyond the rusqlite mutex) only hold under this assumption. Multi-tenant would be a v1.x+ effort.

### 1.3 What problem it solves

Without persistent memory across AI sessions:

- Every session re-discovers prior context — high re-work cost (research workflows keep re-reading the same files; multi-agent discussions repeat positions already settled).
- LLM context windows force a choice: dump everything (signal-to-noise collapse) or dump nothing (start cold).
- Decisions reverse silently because the prior decision wasn't visible at decision time.

mengdie addresses this with:

- **Structured artifact ingestion** (high signal-to-noise) — only output of upstream review pipelines, not raw notes
- **Filtered retrieval** with explicit provenance — agents see what was pulled and why, never silent injection
- **Temporal validity** + **entity-tag contradiction detection** — decisions don't just stack; they supersede with audit trail
- **Daily Dreaming pass** — frequency + relevance promotion, LLM-driven cluster synthesis

### 1.4 Core design principles (8 invariants)

These are load-bearing across the codebase. Reasoning that violates one of these is usually a sign to redesign.

1. **MCP server, not plugin**. Zero dependency on any specific AI tool. `mengdie-mcp` is a stdio MCP server registered in the host AI tool's MCP config (`~/.claude/settings.json`, `~/.codex/config.toml`, etc.).
2. **Structured-artifact ingestion is primary**. The filesystem watcher targets markdown artifacts produced by upstream AI pipelines (conclusion / review / plan / retrospect / synthesis files). These are already filtered by upstream review.
3. **Post-research injection** (avoid anchoring bias). Agents research independently first, then see prior memory as supplemental context — never up-front injection.
4. **Non-silent feedback**. Every injection block shows what was pulled with provenance (source file, knowledge type, valid_from). No silent context injection.
5. **Global storage, per-project default search**. One SQLite DB at `~/.mengdie/db.sqlite`; queries default to current project's `project_id` scope; explicit `scope: "global"` for cross-project search. Avoids migration cost when adding cross-project later.
6. **No AI judgment for cold start**. Existing notes are batch-imported directly without LLM filtering — error amplification at the seeding step is the worst place to introduce it.
7. **Entity-tag + temporal validity**. Decisions have entities (lowercase comma-separated tags, materialized into a normalized `entities` table since v0.0.2); contradictions are detected via directed entity-tag overlap; supersession is recorded with `valid_until` + `superseded_by`. Handles decision evolution, not just instantaneous contradiction.
8. **Agent-centric tech stack**. Code is written by AI agents; optimize for compiler guardrails (Rust strictest), single binary (no runtime install dance), sub-1s warm startup (MCP stdio path; first-run cost is ~10s for the one-time fastembed model download — see §2.9).

### 1.5 Out of scope (explicit)

- **Multi-tenant access control — v0.x scope; v1.x+ future-phase, NOT permanently rejected.** Single-operator is the entire current security boundary, and several design simplifications hold only under this assumption (no auth, global `~/.mengdie/db.sqlite`, no per-row RBAC, no concurrency-coordination beyond the rusqlite mutex, **no DB-layer `project_id` predicate on destructive ops** — see §2.8 I6). **Prerequisites if pursued in v1.x+**:
  - Upgrade I6: enforce `project_id` in SQL WHERE clause for ALL destructive ops (`Db::invalidate_memory`, future delete paths) — defense-in-depth at DB layer, not just MCP layer
  - Add per-row RBAC or per-tenant DB partitioning (single shared `db.sqlite` won't work cross-tenant)
  - Resolve MCP server lifecycle: one `mengdie-mcp` per tenant vs shared with tenant routing
- Generic vector database (sqlite-vec is implementation detail; mengdie's value is the loop, not the storage layer)
- Real-time streaming / pub-sub (consumers poll; no WebSocket or SSE)
- Web UI or visualization (`mengdie audit-stats` CLI + LLM-generated summaries are the inspection surface)
- Plugin auto-install or marketplace integration (mengdie ships as a single binary; users install + register manually)

---

## Part 2 — Technical Implementation (current state)

### 2.1 Tech stack

| Layer | Choice | Version | Rationale |
|---|---|---|---|
| Language | Rust | 1.79+ | Strictest compiler guardrail for agent-written code; single binary; sub-1s warm startup |
| Storage | SQLite via `rusqlite` | features = ["bundled", "load_extension"] | FTS5 included via bundled SQLite; extension API for sqlite-vec |
| Vector search | `sqlite-vec` v0.1.9 | `vec0` virtual table | Replaces in-house `vector.rs` brute-force since v0.0.1 |
| MCP SDK | `rmcp` v1.3 | features = ["server", "macros", "transport-io"] | Official Rust MCP SDK; stdio transport |
| Async | `tokio` | full features | MCP server is async; spawn_blocking wraps sync fastembed |
| Embeddings | `fastembed` v5 | local ONNX, all-MiniLM-L6-v2, 384d | ~90MB model, 2-10ms inference, no Node.js dependency |
| FS watcher | `notify` v8 | — | Cross-platform file watching |
| CLI | `clap` v4 | derive | Standard derive macros |

Cargo.toml net delta:
- During v0.0.1 development: **+1 line** (`sqlite-vec = "0.1.9"` adopted shortly before v0.0.1 tag `463c2f4` shipped on 2026-05-09)
- v0.0.1 ship → current: **0 lines** (v0.0.2 and post-v0.0.2 work added zero new crates; all functionality rides existing transports)

### 2.2 Architecture

**Process model**:

- `mengdie-mcp` is a stdio MCP server. The host AI tool (Claude Code, codex, etc.) spawns it as a subprocess and communicates via JSON-RPC over stdin/stdout.
- Globally installed at `~/.cargo/bin/mengdie-mcp`; registered ONCE in `~/.claude/settings.json` `mcpServers` block, with no per-workspace `cwd` override.
- **Key consequence**: the same `mengdie-mcp` process persists across user project switches within the same Claude Code window. `project_id` inferred from cwd at startup is therefore the SERVER's cwd at the moment of launch — typically the FIRST project the user opens, not necessarily the current one. This is why each MCP tool accepts an optional caller-supplied `project_id` override (see §2.5).

**Storage**:

- `~/.mengdie/db.sqlite` — single global SQLite DB
- `Arc<Mutex<rusqlite::Connection>>` shared across handlers; serializes DB access
- `~/.cache/fastembed/` — embedding model cache (~90MB, downloaded on first run)

**Project scope**:

- `project_id` is inferred from cwd at startup via the precedence chain in `src/core/project.rs`:
  1. `.mengdie.toml` `project.name` field (if file exists + value non-empty)
  2. Git remote URL hash (`format!("proj_{:016x}", ...)`)
  3. Canonical path hash (fallback)
- All three paths produce non-empty strings; **inferred `project_id` cannot be `""` in normal usage** (verified by inspection of `infer_project_id()` in `src/core/project.rs`). **Caveat**: a caller can still pass `Some("")` to MCP tools via the `project_id` override field; only `memory_invalidate` filters it (post-v0.0.2), the other 6 tools pass `Some("")` through to scope queries. This input-normalization asymmetry is tracked as a deferred follow-up (see §3.3).
- Default search scope is the resolved `project_id`; explicit `scope: "global"` for cross-project; explicit `project_id: Some("other-project")` override per-call (incrementally added across all 7 tools through v0.0.3-unreleased)

### 2.3 Module layout

```
src/
  core/                  # ~13K LoC shared library
    db.rs                # SQLite connection, migrations, helpers, MemoryEntry struct
    schema.rs            # Table definitions, FTS5 setup, version migrations (currently user_version = 8)
    project.rs           # project_id inference from git remote
    embeddings.rs        # fastembed-rs integration
    vector.rs            # sqlite-vec adapter (thin wrapper since v0.0.1)
    search.rs            # Hybrid FTS5 + vector + RRF merge, score normalization, audited orchestrator
    parser.rs            # YAML frontmatter extraction, entity extraction
    watcher.rs           # notify-based file watcher
    ingest.rs            # parse → embed → store pipeline + contradiction check
    contradiction.rs     # Entity-tag overlap + temporal validity checks
    dreaming.rs          # Promotion logic (recall_count + avg_relevance) + async LLM synthesis
    decay.rs             # Exponential-decay re-rank for stale memories
    clustering.rs        # Seed-neighborhood cosine clustering (feeds dreaming)
    synthesis.rs         # Prompt builder + structured-output handling
    reembed.rs           # backfill embeddings for legacy synthesis rows (mengdie reembed-synthesis CLI; post-v0.0.2)
    lint.rs              # memory_lint health checks
    llm.rs               # LlmProvider trait + ClaudeCliProvider subprocess impl
    config.rs            # MengdieConfig TOML loader
    mcp_tools.rs         # MCP tool implementations (7 tools)
    metrics.rs           # Observability counters
  bin/
    mcp_server.rs        # stdio MCP entry point (mengdie-mcp)
    cli.rs               # CLI entry point (mengdie ...)
  lib.rs
tests/                   # Integration + e2e suites (369 tests as of 2026-05-23)
resources/
  com.mengdie.dream.plist             # macOS launchd template for daily Dreaming
  synthesis-output-schema.json        # claude-CLI --json-schema payload contract
```

Total source: ~15.4K LoC across `src/core/*.rs` + `src/bin/*.rs`.

### 2.4 Data model

**`MemoryEntry`** — the canonical fact (one row per memory):

```rust
pub struct MemoryEntry {
    pub id: String,                       // UUID v4
    pub project_id: String,               // git-inferred; never empty in normal usage
    pub source_file: String,              // relative path to the source markdown
    pub source_type: String,              // conclusion | review | plan | retrospect | synthesis | factual
    pub knowledge_type: String,           // factual | decisional | experiential
    pub title: String,
    pub content: String,
    pub entities: String,                 // comma-separated lowercased tags; also materialized into normalized entities table since v0.0.2
    pub valid_from: String,
    pub valid_until: Option<String>,      // Set when superseded or explicitly invalidated
    pub superseded_by: Option<String>,    // memory_entries.id of replacement
    pub recall_count: i64,
    pub avg_relevance: f64,               // EMA of relevance scores from search audits
    pub last_recalled: Option<String>,
    pub embedding: Option<Vec<u8>>,       // 384 × 4 bytes little-endian f32
    pub embedding_dim: Option<i64>,
    pub is_longterm: bool,                // promoted by dreaming pass
    pub created_at: String,
}
```

**SQLite schema** (current: `user_version = 8`):

- `memory_entries` — core table (one row per `MemoryEntry`)
- `entities` (added v0.0.2) + `fact_entity` link table (FK to `memory_entries.id` + `entities.id`) — replaces LIKE-scan over `entities` TEXT column for contradiction queries
- `memory_entries_fts` — FTS5 virtual table indexing `title + content + entities`
- `vec_memories` — sqlite-vec `vec0` virtual table for ANN queries (added v0.0.1; replaces 264-LoC vector.rs brute-force)
- `memory_search_audit` (added v0.0.1) + `audit_returned_facts` link table — every `memory_search` call logs query + scope + took_ms + per-call returned fact IDs
- `metrics` — persistent counters (`search_count`, `ingest_count`, etc.)

Schema migrations are idempotent and gated by `user_version` PRAGMA. v0.0.1 shipped with `user_version = 7`; v0.0.2 entity-graph migration bumped to `user_version = 8`.

### 2.5 MCP tool surface (7 tools)

All tools accept an optional `project_id: Option<String>` override (caller-authority precedence) and respect the server's startup-cached `default_project_id` as fallback.

| Tool | Added | Purpose |
|---|---|---|
| `memory_search` | v0.0.1 | Hybrid FTS5 + vector + RRF; `limit`, `min_score`, `scope` (current/global) |
| `memory_ingest` | v0.0.1 | Parse + embed + store; contradiction check; `resolves` for atomic supersession |
| `memory_invalidate` | v0.0.1 (+ post-v0.0.2: `project_id` override field + cross-project guard on full-UUID branch, both added in commit `e8122a9`) | Mark `valid_until`; full-UUID + 8+ char prefix supported; full-UUID branch's cross-project guard mirrors `memory_get`'s pattern — prevents accidental cross-project invalidation when an operator passes a UUID belonging to a different project |
| `memory_get` | v0.0.2 | Fetch full `MemoryEntry` by full UUID or 8+ char prefix; bumps `recall_count` |
| `memory_status` | v0.0.2 | DB health snapshot: row counts, last-ingest timestamp, persistent metrics, audit pipeline view |
| `memory_lint` | v0.0.2 | Three deterministic checks: orphan GC (dangling FK), unresolved contradictions, embedding drift |
| `memory_entity_facts` | v0.0.2 | Facts tagged with a given entity name; uses materialized `entities` table |

**`project_id` resolution chain** (canonical pattern; 7-tool consistency is a deferred follow-up — see §3.3):

```rust
// memory_invalidate (post-v0.0.2): caller-authority + empty-string normalization
let scope = params.project_id
    .as_deref()
    .filter(|s| !s.is_empty())
    .unwrap_or(&self.default_project_id);
```

Currently `memory_invalidate` is the only tool with the `.filter(|s| !s.is_empty())` normalization; the other 6 retain pre-existing `Some("")` passes-through. A deferred follow-up will unify the pattern across all 7 tools (trigger: next touch of any non-invalidate tool's `project_id` resolution site, OR operator-observed `project_id = ""` row in DB).

### 2.6 CLI surface

10 subcommands in `src/bin/cli.rs`:

```
mengdie-mcp                        # stdio MCP server entry point (main runtime)
mengdie audit-stats [--since 7d]   # audit pipeline observability
mengdie dream                      # daily promotion + LLM synthesis pass
mengdie import <dir>               # cold-start: batch-ingest markdown without LLM judgment
mengdie search <query>             # CLI mirror of memory_search (for ops)
mengdie list [filters]             # browse memories with simple filtering
mengdie rename <old> <new>         # rename project_id across memory_entries (one-way-door operational command — use carefully)
mengdie stats                      # high-level corpus stats (row counts, recall distribution)
mengdie synthesis-audit            # inspect synthesis rows + their source links
mengdie reembed-synthesis          # backfill embeddings for legacy synthesis rows (post-v0.0.2)
```

Run `mengdie --help` for current flags + subcommand details.

### 2.7 Filesystem layout

```
~/.mengdie/
  db.sqlite                       # global DB (single-operator scope)

~/.cache/fastembed/                # embedding model cache (~90MB)
  models--Qdrant--all-MiniLM-L6-v2-onnx/

# Optional per-project override
<project-root>/
  .mengdie.toml                   # if present, overrides git-derived project_id via project.name field

# macOS daily Dreaming
~/Library/LaunchAgents/
  com.mengdie.dream.plist         # launchd template for daily promotion+synthesis pass
```

### 2.8 Key invariants

These are enforced by code or documented contracts. Violations are bugs.

| # | Invariant | Where enforced |
|---|---|---|
| I1 | All stdout is MCP transport; logging via `tracing` → stderr only | All `tracing::info!` / `tracing::error!` calls use stderr; `println!` is banned |
| I2 | fastembed inference is sync/blocking — wrap in `tokio::task::spawn_blocking` | `mcp_tools::ingest`, `mcp_tools::search` |
| I3 | DB connection: `Arc<Mutex<Connection>>` shared across handlers | Constructor at `MengdieServer::new`; all access via lock |
| I4 | `project_id` derivation: git-remote-derived; never empty in normal usage | `src/core/project.rs` `infer_project_id()` — verified by direct inspection |
| I5 | Cross-project guards at MCP layer for read+destructive ops: `memory_get` (approx lines 657-675 from tool entry at line 563), `memory_invalidate` (added in commit `e8122a9`, landed on `main` 2026-05-23) | `src/core/mcp_tools.rs` |
| I6 | DB-layer destructive ops are NOT project-scoped at SQL level — **load-bearing only under single-operator scope** (§1.5). `Db::invalidate_memory` SQL has no `project_id` predicate; intentional asymmetry under v0.x; defense-in-depth lives at MCP layer. See SAFETY comment at `src/core/db.rs:415`. If multi-operator pursued (v1.x+), this MUST be upgraded — see §1.5 prerequisites | `src/core/db.rs:415` (SAFETY comment) + §1.5 multi-op prerequisite |
| I7 | Schema migrations are idempotent + `user_version` PRAGMA gated | `src/core/schema.rs` |
| I8 | All committed artifacts (code, comments, docs, commit messages) are in English | project convention; see [CLAUDE.md](../CLAUDE.md) |

### 2.9 Operational notes

- **First-run cost**: ~10s (fastembed model download). Subsequent starts < 1s.
- **DB migration**: idempotent on every startup; `user_version` PRAGMA bump only when migration completed successfully.
- **Cross-family review tooling**: dev workflow uses codex (OpenAI) + gemini (Google) MCP servers in addition to the host AI tool — multiple model families review the same change for orthogonal blind spots. Mengdie's source repo is agnostic to this; it's a contributor-workflow concern, not a runtime dependency.
- **Dreaming pass**: daily via launchd on macOS (template at `resources/com.mengdie.dream.plist`); cron equivalent on Linux. **Output classification per filter module** (clarifies the §1.1 spiral-upward claim):
  - `dreaming.rs`: produces NEW `MemoryEntry` rows of `source_type = synthesis` (LLM-clustered + consolidated content), embedded + indexed identically to ingested rows; ALSO mutates `is_longterm` + `recall_count` on inputs. **This is the synthesis arm of the spiral.**
  - `clustering.rs`: intermediate only — seed-neighborhood cosine clusters feed `dreaming.rs`; no DB writes.
  - `decay.rs`: retrieval-time re-rank via exponential decay on `last_recalled`; affects search ordering, no row mutation.
  - `lint.rs`: detection only — produces a `LintReport` for operator review (orphan / unresolved contradictions / embedding drift); does NOT mutate DB. Operator action via separate `memory_invalidate` / fix-commit.

### 2.10 Known operational failure modes

- **Concurrent `mengdie-mcp` instances** — if the operator opens two projects in Claude Code in parallel windows, BOTH spawn `mengdie-mcp` and compete for `~/.mengdie/db.sqlite` via the rusqlite mutex. Second instance can block or error with SQLITE_BUSY. Diagnosis: `tracing` logs on stderr show `database is locked`. Workaround: close the second window or use a separate SQLite via env override (`MENGDIE_DB_PATH` if implemented; not currently supported — file a BL if hit).
- **fastembed cache corruption** — interrupted model download (network drop, process crash during cold-start ~10s window) can leave a corrupt ONNX file in `~/.cache/fastembed/`. Symptom: `Embedder::new()` errors at startup OR returns silent zero-vectors. Recovery: `rm -rf ~/.cache/fastembed/` and let the next startup re-download.
- **MCP tool wire compatibility** — mengdie ships additive-only tool schema changes: new tools are added at server side; existing tools have new `Option<T>` fields gated with `#[serde(default)]` (e.g., post-v0.0.2 `InvalidateParams.project_id`). Pre-update clients (e.g., a Claude Code window started before `mengdie-mcp` upgrade) that omit new optional fields work unchanged. **Not supported**: removing or renaming a tool / changing a required field's type. Stale clients calling a removed tool receive an rmcp "unknown tool" error; restart the host AI tool to re-load the tool schema.

---

## Part 3 — Pointers & Open Questions

### 3.1 Version history

| Version | Tag | Shipped | Theme |
|---|---|---|---|
| v0.0.1 | `463c2f4` | 2026-05-10 | Minimum-viable knowledge-memory MCP server: 3 tools (`memory_search`, `memory_ingest`, `memory_invalidate`), sqlite-vec adoption, persisted audit pipeline, daily Dreaming pass |
| v0.0.2 | `152ba97` | 2026-05-19 | Entity-graph upgrade (schema `user_version 7 → 8`) + 4 new MCP tools (`memory_get`, `memory_status`, `memory_lint`, `memory_entity_facts`) + retroactive test harness |
| v0.0.3 | unreleased | 2026-05-22 → 2026-05-23 | Synthesis-row embedding fix (+ `reembed-synthesis` CLI); `InvalidateParams.project_id` override field; cross-project guard on full-UUID `memory_invalidate`. Tag pending. |

See [CHANGELOG](../CHANGELOG.md) for shipped-release entries.

### 3.2 Cross-references

- **End-user install + usage** → [README](../README.md)
- **Per-release notes** → [CHANGELOG](../CHANGELOG.md)
- **Contributor conventions** (Rust style, testing patterns, commit hygiene, doc language) → [CLAUDE.md](../CLAUDE.md)

Per-tool MCP specs and the operator's internal design archive live under `.ae/docs/` (gitignored — local to the maintainer's working copy, not shipped to public). The canonical source-of-truth for current MCP tool behavior is `src/core/mcp_tools.rs` itself.

### 3.3 Open questions & deferred work

Tracked outside this doc; brief snapshot as of 2026-05-23:

- **MCP cross-tool `project_id` input normalization** (P2 correctness gap) — extend `memory_invalidate`'s `.filter(|s| !s.is_empty())` pattern to the other 6 tools as a shared helper. **Direction is silent no-op fallback to default**, NOT reject-with-error. Trigger: next touch of any non-invalidate tool's `project_id` resolution site, OR observed `project_id = ""` row in DB. Item (c) — `memory_ingest(Some(""))` can currently seed `project_id=""` rows because `unwrap_or_else` doesn't fire on `Some` — is the load-bearing reason this is P2 rather than P3.
- **Server-side `roots/list` runtime-refresh spike** — gated on a non-blocking probe of the host MCP client's `roots` capability returning a usable URI shape. Out of scope for v0.x; companion caller-authority pattern (per-call `project_id` override across all 7 tools) is the v0.x stand-in.
- **`resolved_project_id` echo on tool responses** — gated on a real consumer of the echoed field landing in calling clients (no AI workflow currently inspects it; LSP-style echo without an immediate consumer is observability cost without observability value).
- **Schema migration for multi-operator** — see §1.5 prerequisites. Hard-gated on multi-operator scope being explicitly opened in a v1.x+ design pass.

### 3.4 Reading order for newcomers

1. Skim [README](../README.md) for elevator pitch + install instructions
2. Read Part 1 of this doc for goal + design principles
3. Skim Part 2 sections 2.1–2.5 for the current implementation surface
4. For deeper data model: read `src/core/db.rs` (`MemoryEntry` struct) + `src/core/schema.rs` (table layout + migration history) directly — they are the canonical source
5. For AI-agent contributors: read [CLAUDE.md](../CLAUDE.md) for project conventions (Rust style, testing patterns, commit hygiene, doc language)
