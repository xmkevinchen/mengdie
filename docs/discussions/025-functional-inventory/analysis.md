---
id: "025"
title: "Analysis: v0.0.1 Step A1 — mengdie functional inventory"
type: analysis
created: 2026-04-27
tags: [v0.0.1, inventory, refactor, src-audit, capability-mapping]
---

# Analysis: v0.0.1 Step A1 — mengdie functional inventory

## Question

What does mengdie src/ actually do today, broken down by module, with explicit capture of (1) public input/output contracts, (2) which behaviors are pure logic portable to v0.0.1 vs which are baked-in v0.x assumptions, and (3) empirical evidence of usage. Step A1 of the v0.0.1 redesign migration outline; pairs with A2 at `docs/discussions/026-rust-oss-survey/analysis.md` to feed Step B integration discussion.

## Findings

### Prior Art from Project Knowledge Base

Prior context: unavailable (memory_search MCP tool not registered in this session).

### Relevant Code

Three archaeologists split src/ into semantic slices:

- **Foundation slice** (db / schema / project / config / parser / metrics) — 6 modules, ~1300 LoC.
- **Pipeline + I/O slice** (watcher / ingest / mcp_tools / mcp_server / cli) — 5 files, ~1400 LoC.
- **Intelligence slice** (embeddings / vector / search / clustering / synthesis / llm / dreaming / contradiction / decay) — 9 modules, **4972 LoC** (the substantive bulk).

Total src/ (excluding tests): roughly 8500–9000 lines of Rust across 19 modules.

#### Module-by-module inventory

| Module | LoC | Slice | Portability tier | Replacement candidate (see 026) | Wired up via |
|---|---|---|---|---|---|
| db.rs | ~215 | foundation | PURE | — | mcp_tools, cli, ingest |
| schema.rs | ~520 | foundation | PURE (schema) / PORTABLE-WITH-CLEANUP (v5 migration) | — | db.rs init |
| project.rs | ~90 | foundation | PORTABLE-WITH-CLEANUP | `toml` crate already in tree | mcp_server, cli |
| config.rs | ~150 | foundation | PURE | — | mcp_server, cli |
| parser.rs | ~280 | foundation | PORTABLE-WITH-CLEANUP | gray_matter / serde_yml fork | ingest, watcher |
| metrics.rs | ~50 | foundation | DELETE | tracing + AtomicU64 | mcp_tools, cli |
| watcher.rs | ~75 | pipeline | KEEP-IF-DAEMON-SURVIVES | — (notify pattern is correct) | unwired (library only, daemon deferred to Phase 2) |
| ingest.rs | ~70 | pipeline | PORTABLE-WITH-CLEANUP | swiftide Pipeline (contested — see 026) | mcp_tools, cli, watcher (unwired) |
| mcp_tools.rs | ~440 | pipeline | PORTABLE-WITH-CLEANUP | rmcp; remove FTS-only fallback | mcp_server |
| mcp_server.rs | 43 | pipeline | PURE | — | bin entrypoint |
| cli.rs | ~770 | pipeline | PORTABLE-WITH-CLEANUP | clap fine; remove synthesis-audit subcommand | bin entrypoint |
| embeddings.rs | 210 | intelligence | PORTABLE-WITH-CLEANUP | fastembed-rs stays (KEEP) | ingest, search, clustering, dreaming |
| vector.rs | 265 | intelligence | REPLACEABLE (qualified) | sqlite-vec — pending static-vs-dynamic distribution check | db.rs, search.rs |
| search.rs | 723 | intelligence | PORTABLE-WITH-CLEANUP | RRF tiny; FTS5 stays | mcp_tools, cli |
| clustering.rs | 626 | intelligence | CONDITIONAL-DELETE | sqlite-vec ANN ≠ batch clustering — challenger pushback | dreaming.rs |
| synthesis.rs | 450 | intelligence | PORTABLE-WITH-CLEANUP | rig::Extractor — unverified for subprocess streaming | dreaming.rs |
| llm.rs | 794 | intelligence | TRAIT REPLACEABLE / IMPL KEEP | rig CompletionModel — near-zero v0.0.1 value | synthesis.rs, dreaming.rs |
| dreaming.rs | 1327 | intelligence | PORTABLE-WITH-CLEANUP | none — orchestration is project-specific | cli.rs |
| contradiction.rs | 357 | intelligence | PORTABLE-WITH-CLEANUP (add DB index) | none (logic is project-specific) | ingest.rs, mcp_tools.rs |
| decay.rs | 220 | intelligence | PURE | — | dreaming.rs, search.rs |

### Architecture & Patterns

**Data flow as it actually runs:**

```
AE pipeline output (conclusion.md, plan.md, ...)
  → parser.rs (YAML frontmatter, entity extraction, source_type inference)
  → ingest.rs ingest_file
      → embeddings.rs embed_with_context (metadata-in-chunk encoding)
          → fastembed all-MiniLM-L6-v2 → Vec<f32> (384d)
      → db.rs insert_memory + vector.rs store_embedding
      → contradiction.rs check_contradictions (entity overlap + cosine)
  → IngestResult { entry_id, conflicts }

Search path (MCP tool memory_search):
  query → embed → search.rs search_fts (FTS5 BM25) ∥ vector.rs search_vector (Rust brute-force cosine)
  → search.rs rrf_merge (k=60, 2 rankers)
  → search.rs apply_boost_and_decay (LONGTERM_BOOST=1.2 × decay_factor)
  → db.rs record_recall

Dream pass (CLI: mengdie dream [--synthesize]):
  Promotion: SQL UPDATE WHERE recall_count >= 3 AND avg_relevance >= 0.45 AND last_recalled >= cutoff
  Decay: SELECT longterm rows → decay.rs effective_relevance → demote if < 0.20
  Synthesis: clustering.rs (O(N²) seed-neighborhood) → synthesis.rs prompt → llm.rs subprocess (claude -p)
              → parse JSON → store with link table
```

The watcher and ingest are deliberately decoupled by `on_file: FnMut(&Path)` callback. The watcher exists as a library but is **not wired into mcp_server.rs** — daemon integration was explicitly deferred to Phase 2 per CLAUDE.md.

**MCP tool surface** (the actual external contract):

- `memory_search` — query + scope + limit + min_score → results with degraded fallback
- `memory_ingest` — title + content + source_file + source_type enum + knowledge_type enum + entities + resolves[] → entry_id + conflicts
- `memory_invalidate` — entry_id + reason + superseded_by → success

**CLI subcommand surface**: `dream`, `import`, `list`, `search`, `rename`, `stats`, `synthesis-audit`. The `dream` subcommand alone has 9 flags (decay thresholds, cluster sizes, synthesize toggle, dry-run modes).

#### v0.x assumptions baked in (concrete file:line citations)

| # | Assumption | Citations |
|---|---|---|
| 1 | Production-data-precious migrations: ~300 lines of v5 defensive pre-checks for a 27-row corpus | schema.rs:214–516 (comment at 346–350 self-acknowledges) |
| 2 | Multi-writer SQLite locking (`Arc<Mutex<Connection>>` + WAL + busy_timeout=5000) for single-writer stdio MCP | db.rs:19–23, schema.rs:86–88 |
| 3 | Hand-rolled TOML parser despite `toml = "0.8"` already in tree | project.rs:29–86 |
| 4 | Fleet-sized observability: persisted SQLite metrics counters with unused `value_float` column | metrics.rs:1–53, schema.rs:145–150 |
| 5 | AE-specific source_type names hardcoded in parser inference | parser.rs:136–155 |
| 6 | FTS-only fallback in memory_search designed as if local fastembed is unreliable remote call | mcp_tools.rs:221–244 |
| 7 | Watcher exported but never integrated into MCP server (Phase 2 daemon deferred) | mcp_server.rs:1–43, watcher.rs:1–75 |
| 8 | synthesis-audit CLI subcommand is scaffolding for v0.x plans 017/022 with no v0.0.1 analog | cli.rs:145–148 |
| 9 | `EMBEDDING_DIM = 384` hardcoded in clustering.rs separately from embeddings.rs (dual source of truth; model swap silently produces zero results via `WHERE embedding_dim = ?`) | clustering.rs:43 vs embeddings.rs::Embedder::new |
| 10 | `RRF_MAX = 2/61` hardcoded to exactly 2 rankers (silent under-estimation if a 3rd ranker is added) | search.rs:175 |
| 11 | `DEFAULT_THRESHOLD = 0.75` in clustering uncalibrated against AE corpus (borrowed from sentence-transformers community_detection default) | clustering.rs:22 |
| 12 | Decay `HALF_LIFE_DAYS = 60`, `DEMOTION_FLOOR = 0.20` calibrated to current `avg_relevance` distribution; will shift if v0.0.1 changes ingestion to propositional facts | decay.rs:14, 21 |
| 13 | ClaudeCliProvider is the only LlmProvider impl; build_provider hard-fails on any other string (oMLX endpoint exists but no provider) | llm.rs:219+ |
| 14 | `CONTENT_CHAR_LIMIT = 4000` truncates ~13% of memories in production runs | synthesis.rs:7 |
| 15 | contradiction.rs full table scan, no DB index on (project_id, valid_until, entities) | contradiction.rs:48 (backlog 004-11 trigger: >1K memories) |

#### Empirical results (synthesis runs against production DB)

- **First synthesis run** (threshold=0.75, min_size=3, N=198 eligible): 14 clusters, 133 residuals (76% residual rate), 13 syntheses created, 1 parse error, 0 LLM errors, 26 truncations. Manual spot-check of 3 syntheses: topically tight, no hallucinations.
- **Second run** (min_size 3→2, null-escape-hatch added, N=237): 25 clusters, 125 residuals, 14 syntheses, 11 LLM-skipped (44% skip rate), 0 parse errors, 30 truncations. Skip quality: 9/11 correct, 2/11 unclear, 0 false negatives.

The intelligence layer demonstrably works at current scale. v0.x's complexity is largely justified for what it produces; the architectural critique is about overhead (production-data-precious migrations, fleet observability) and dual-source-of-truth bugs (assumptions 9–11), **not core algorithmic correctness**.

#### Test coverage gaps

- watcher.rs: 3 unit tests; ingest.rs: 1 smoke test (mock embedder); mcp_tools.rs: 4 serde-validation tests
- e2e.rs: 2 tests (full pipeline `#[ignore]` requires fastembed model; decay smoke runs in CI)
- **Gaps**: no integration test exercising MCP server over real stdio; no test covering `memory_invalidate`; no watcher↔ingest integration test.

#### CLAUDE.md drift

- mod.rs:5 exports `pub mod decay`; CLAUDE.md project structure table omits decay.rs.
- schema.rs:16 `ALLOWED_SOURCE_TYPES` includes `"analysis"`; parser.rs:137–155 `infer_source_type` has no `"analysis"` branch (returns `"unknown"`).
- `serde_yaml` dep has known soundness issues; `serde_yml` fork or a frontmatter-specific crate would eliminate.

### Industry Practice Comparison

Detailed in companion analysis at `docs/discussions/026-rust-oss-survey/analysis.md`. Module-relevant verdicts:

- **vector.rs** → ADOPT sqlite-vec (qualified). standards-expert: LOW adoption cost. Challenger pushback: "LOW" needs static-vs-dynamic distribution verification before commit.
- **llm.rs (trait)** → near-zero value to wrap in rig CompletionModel; ClaudeCliProvider works.
- **synthesis.rs JSON parser** → rig::Extractor is unverified for subprocess streaming. Don't adopt on survey alone; need a 50-line spike.
- **ingest.rs + watcher.rs** → swiftide rejected (2-of-3: standards-expert + challenger SKIP; standards-rag ADOPT). v0.0.1-rebuild-plan defers "generic ingestion from other sources"; framework adoption is for a use case not in scope.
- **embeddings.rs (fastembed-rs)** → KEEP. Confirmed best-in-class.
- **search.rs (FTS5)** → KEEP. Tantivy is HIGH cost for no measurable gain at current scale.

### Challenges & Disagreements

#### The do-nothing case (challenger, unaddressed in rebuild plan)

v0.8.0 already runs as the AE brain: search, ingest, synthesis all work. 214 memories in production. The genuine gap toward "AE 的大脑" thesis is **daemon integration + ae:analyze Round-0 injection** — both are *AE plugin* changes, not mengdie src/ changes. Filing v0.0.1 BLs for sqlite-vec and rig wrapping does not move the needle on the actual missing piece.

Step B should explicitly compare:
- **(a) Do nothing on src/, wire AE plugin daemon + injection** — may ship faster, delivers the thesis directly.
- **(b) Surgical library replacements first, then wire AE integration** — current implicit plan.

This option has been unexamined in the rebuild plan as written.

#### Coherence concern (challenger)

The rebuild-plan thesis (AE 的大脑 + 自成长 + reflection) is **more ambitious** than v0.x. The library replacements being recommended (sqlite-vec + maybe rig wrapper + maybe rig::Extractor) are a **modest delta** from v0.x — they don't move 自成长 forward. If the thesis is the actual goal, v0.0.1 BLs should be about reflection mechanism design (Phase 0 deferred questions: trigger model, meta-fact confidence, single-vs-split table), **not library swaps**.

#### Surprises

- The intelligence layer is the bulk of complexity and the most defensible code (empirical results show it works). The foundation slice has more "v0.x baggage" than the intelligence slice.
- watcher.rs is dead code from one perspective (never wired in production) and a careful library from another (3 unit tests, correct notify+debounce pattern). v0.0.1 ingestion model (push from AE plugin vs pull from filesystem) decides whether watcher.rs is KEEP or DELETE.
- decay.rs is **40 LoC of pure math** and is the cleanest module in the codebase. It survives any direction.

## Summary

mengdie v0.x src/ is roughly 8500 lines across 19 modules. The intelligence layer (embeddings + vector + search + clustering + synthesis + dreaming + decay) is the substantive 4972 lines and **demonstrably works at current scale** (two production synthesis runs, 13–14 clean syntheses each). The foundation slice has well-defined v0.x baggage — 15 cited assumptions, mostly fleet-sized observability and production-data-precious migration plumbing — but the schema and core CRUD are PURE.

**Module portability distribution:**

- **PURE / KEEP-AS-IS** (5): db, schema (table defs), config, mcp_server, decay
- **PORTABLE-WITH-CLEANUP** (10): project, parser, mcp_tools, cli, embeddings, search, ingest, synthesis, dreaming, contradiction
- **REPLACEABLE (qualified)** (2): vector (sqlite-vec, pending static-vs-dynamic verification), llm trait (rig CompletionModel — challenger says near-zero v0.0.1 value)
- **CONDITIONAL DELETE** (1): clustering (only if ANN replaces — challenger argues ANN retrieval ≠ batch clustering, conflation may not hold)
- **DELETE** (1+): metrics; synthesis-audit subcommand; FTS-only fallback dead path; watcher.rs IF v0.0.1 uses push pattern from AE plugin

**Three strategic directions surface from the cross-cutting analysis** — they are mutually exclusive:

1. **Surgical refactor** (arch-intelligence + standards-expert + standards-rag converge): replace vector.rs with sqlite-vec, optionally rig::Extractor for synthesis JSON. ~200–500 lines touched. **Modest delta from v0.x; doesn't address 自成长 thesis.**
2. **API-first pivot** (codex-proxy): delete RAG infrastructure, mengdie becomes catalog + policy engine in front of OpenAI File Search. **Rejected by challenger on credential-sovereignty grounds** (CLAUDE.md states "mengdie never touches secrets" — locked decision); also breaks single-binary offline operation.
3. **Do-nothing + AE plugin work** (challenger Phase 2): v0.8.0 stays; build daemon + Round-0 injection as AE plugin BLs. **Most directly serves the AE-的大脑 thesis. Unexamined in rebuild plan.**

Step B must pick one direction explicitly, not ship a middle ground.

## Possible Next Steps

→ `/ae:discuss docs/discussions/025-functional-inventory/` jointly with `docs/discussions/026-rust-oss-survey/` for Step B integration strategy. Recommended discussion topics:

1. **Direction selection** — Surgical / Do-nothing / hybrid? Make this an explicit decision point. Resolve before per-module verdicts.
2. **Per-module firm verdict table** — convert the 10 PORTABLE-WITH-CLEANUP entries to KEEP-or-CHANGE decisions; resolve the 2 REPLACEABLE-QUALIFIED entries by running the verifications below.
3. **Reflection / 自成长 design** — Phase 0 deferred items (trigger model, meta-fact confidence, single-vs-split table). gemini-proxy supplied a starting concrete pattern: hybrid trigger (event + cron + on-demand) + split-table schema + structured provenance. Decide v0.0.1 vs post-v0.0.1 scope.
4. **Rebuild-plan thesis coherence** — do the proposed library swaps actually serve "AE 的大脑 + 自成长 + reflection", or are they orthogonal? If orthogonal, what's the actual v0.0.1 motivation?

**Verification spikes** (decision-blocking, ~1 hour total):
- sqlite-vec: clone + `cargo add` to fresh workspace; check whether `cargo build --release` produces a self-contained binary or requires a runtime `.dylib`. Decides ADOPT vs DEFER.
- rig::Extractor: 50-line proof that `rig::Extractor<SynthesisDraft>` parses claude-CLI subprocess output correctly. Decides synthesis.rs replacement.
