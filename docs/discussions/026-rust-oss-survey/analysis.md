---
id: "026"
title: "Analysis: v0.0.1 Step A2 — Rust open-source library survey"
type: analysis
created: 2026-04-27
tags: [v0.0.1, survey, rust-ecosystem, rag-libraries, vector-stores, embedding-libs]
---

# Analysis: v0.0.1 Step A2 — Rust open-source library survey

## Question

Survey mature Rust open-source libraries that mengdie v0.x reinvented. Per candidate library: scope, maturity, license, mengdie module overlap, solo-project adoption cost, abandonment risk, verdict (adopt / wrap / defer / skip). Step A2 of the v0.0.1 redesign migration outline; pairs with A1 at `docs/discussions/025-functional-inventory/analysis.md` to feed Step B integration discussion.

## Findings

### Prior Art from Project Knowledge Base

Prior context: unavailable (memory_search MCP tool not registered in this session).

### Relevant Code

Survey work split across two specialists by library lane:

- **Storage + search lane**: sqlite-vec, LanceDB, Qdrant Rust client, Tantivy + adjacents (arroy, duckdb-rs)
- **RAG framework + LLM client lane**: swiftide, rig, candle, async-openai, fastembed-rs (already in use) + adjacents (text-splitter, ollama-rs, mistral.rs, community Anthropic clients)

Source order per library: GitHub README, recent CHANGELOG / release date, latest docs.rs API surface, then comparison searches as needed. Several libraries cloned to `/Users/ckai/Projects/mengdie-oss-survey/` (gitignored scratch directory) for direct inspection of `Cargo.toml`, `examples/`, `src/lib.rs`, `tests/`, and `git log`.

### Architecture & Patterns

#### Library scorecard (compiled across both lanes)

| Library | Lane | Stars / last release | License | Mengdie overlap | Cost | Verdict |
|---|---|---|---|---|---|---|
| sqlite-vec | storage | 7.5k / v0.1.9 (Mar 2026) | MIT+Apache-2.0 | vector.rs (100%) | LOW (claimed); contested | **Qualified ADOPT** — pending static-vs-dynamic distribution check |
| rig | RAG/LLM | 7.1k / Apr 2026 | MIT | llm.rs trait, synthesis.rs Extractor | MED | **Contested** — see disagreement section |
| swiftide | RAG/LLM | 691 / v0.32.1 (Nov 2025) | MIT | ingest.rs + watcher.rs | HIGH (transitive deps) / MED (modular) | **Disagreed → SKIP (TL synthesis)** |
| LanceDB | storage | 10.1k / v0.27.2 (Mar 2026) | Apache-2.0 | vector + search + db (full layer) | HIGH | **DEFER** — trigger: corpus > 10k OR p95 vector latency > 50ms |
| Qdrant Rust client | storage | 401 / v1.17.0 (Feb 2026) | Apache-2.0 | vector + search | VERY HIGH (separate server) | **SKIP** — incompatible with single-binary, no-server constraint |
| Tantivy | storage | 15.1k / v0.26.1 (Apr 2026) | MIT | search.rs FTS path | HIGH (40+ deps, parallel index) | **SKIP for now** — FTS5 adequate at current scale |
| candle | RAG/LLM | 20.1k / active | Apache-2/MIT | embeddings.rs (alternative) | HIGH | **SKIP** — fastembed handles embedding; candle for local LLM only |
| async-openai | RAG/LLM | 1.9k / v0.36.1 (Apr 2026) | MIT | llm.rs HTTP path (oMLX endpoint) | LOW | **ADOPT as option** — 2nd LlmProvider impl |
| fastembed-rs | RAG/LLM | 873 / v5.13.4 (Apr 27 2026) | Apache-2.0 | embeddings.rs (already in use) | NONE | **KEEP** — confirmed best-in-class |
| arroy | storage adj | 302 / v0.6.2 (Aug 2025) | MIT | vector / clustering | MED | **SKIP** — index is build-once, incompatible with streaming ingest |
| duckdb-rs | storage adj | 901 / v1.10502 (Apr 2026) | MIT | db / schema | HIGH | **SKIP** — no native vector search, no FTS |
| Anthropic Rust HTTP client | RAG/LLM adj | community max 75★ | MIT | llm.rs | LOW-MED | **SKIP** — keep ClaudeCliProvider; rig fallback if needed |
| text-splitter | RAG/LLM adj | 594 / v0.30.1 (Apr 2026) | MIT | (future chunking) | LOW | **OPTIONAL** — v0.0.1 ingests pre-structured AE facts; not needed |
| ollama-rs | RAG/LLM adj | 1k / v0.3.4 (Feb 2026) | MIT | llm.rs (local LLM path) | LOW | **SKIP** — local LLM synthesis out of v0.0.1 scope |
| mistral.rs | RAG/LLM adj | 7.1k / v0.8.0 (Apr 2026) | MIT | embeddings (alternative) | HIGH | **SKIP** — fastembed sufficient |

### Industry Practice Comparison

#### sqlite-vec (highest-leverage proposed swap)

**Scope.** SQLite extension (pure C, zero deps) providing `vec0` virtual tables for KNN over float32 / int8 / binary vectors. Loads via `sqlite3_auto_extension` into existing rusqlite connection. No separate process, no separate file. Distance functions: `vec_distance_L2`, `vec_distance_cosine`, `vec_distance_hamming`.

**Mengdie overlap.** Replaces vector.rs (264 lines) entirely. The current implementation does a full table scan, deserializes every BLOB to `Vec<f32>` in Rust, computes cosine in a loop, sorts, truncates. sqlite-vec moves this to one SQL query against an indexed virtual table. Even at N=214 the round-trip avoidance is meaningful — not just a "wait for N>10k" win.

**Standards-expert verdict.** ADOPT at LOW cost. `cargo add sqlite-vec` plus a one-time `unsafe { sqlite3_auto_extension(...) }` registration, plus a `CREATE VIRTUAL TABLE vec_memories USING vec0(embedding float[384])` schema migration, plus a `1.0 - distance/2.0` conversion in the RRF merge. ~50 lines net change.

**Challenger pushback.** "LOW adoption cost" needs concrete qualifier: does sqlite-vec ship as a Rust crate that statically links the C via a `cc` build script, OR as a shared library (`.dylib`/`.so`) that operators must install separately? The two have very different operator stories. Same-binary-no-runtime-extension is LOW; "operator installs `.dylib`" is real friction. **Decision blocked on this verification before ADOPT commits.**

**Distance metric.** sqlite-vec returns distance (cosine: 0 = identical, 2 = opposite); existing RRF expects similarity (1 = identical). Conversion `score = 1.0 - distance/2.0` maintains [0,1]. Verify against fastembed-rs all-MiniLM-L6-v2 normalization (it normalizes to unit vectors by default).

#### rig — three separable pieces, three verdicts

**Scope.** LLM agent framework with `CompletionModel` and `EmbeddingModel` traits, 20+ providers, `Extractor<T>` for structured JSON (T: Deserialize + JsonSchema), `Agent` with dynamic context. 7.1k stars, 568+ releases (rapid iteration), pre-1.0.

**Piece 1 — wrap ClaudeCliProvider in rig's CompletionModel trait.** Standards-rag proposes this as a stable interface contract. **Challenger: near-zero value for v0.0.1.** ClaudeCliProvider already works; wrapping it in a trait it doesn't need adds indirection without benefit. The trait churn risk (rig is pre-1.0, "future updates will introduce breaking changes" warning) is real for a solo project. **TL synthesis: skip the trait wrap unless a concrete second provider materializes that justifies the abstraction.**

**Piece 2 — replace synthesis.rs hand-rolled brace-depth JSON parser with `rig::Extractor<SynthesisDraft>`.** Standards-rag proposes this saves ~100 lines. **Challenger: unverified.** rig::Extractor is designed for REST-API-response structured outputs. mengdie's synthesis path uses subprocess-streamed text from `claude -p`. It is unverified that rig::Extractor handles streaming subprocess output. **TL synthesis: requires a concrete spike — write a 50-line proof that rig::Extractor parses claude-CLI subprocess output correctly. If yes, adopt for synthesis.rs only. If no, keep brace-depth parser.**

**Piece 3 — adopt rig Agent with dynamic_context for v0.0.1 reflection / 自成长 mechanism.** Premature. The reflection design hasn't been settled (Phase 0 deferred items). **TL synthesis: revisit after Step D.**

#### swiftide — the active disagreement, resolved SKIP

**standards-rag case for ADOPT.** swiftide's `Pipeline::from_loader().then().then_chunk().then_in_batch().then_store_with()` is exactly what mengdie's ingest.rs + watcher.rs hand-rolled. Built-in exponential backoff. Native fastembed integration. text-splitter comes free. Saves the code that arch-pipeline flagged as "ingest pipeline mirrors swiftide indexing."

**standards-expert case for SKIP.** swiftide's primary storage adapters are Qdrant / LanceDB / Redis / Postgres — none compatible with mengdie's single-binary SQLite constraint. Adopting swiftide buys framework overhead with no first-class storage reuse. The ingest pipeline is ~220 lines total (watcher 75 + ingest 70 + parser 280); not complex enough to justify framework dependency. Pre-1.0 churn cost.

**Challenger sides with SKIP, three additional grounds.**
1. v0.0.1-rebuild-plan explicitly says *"generic ingestion from other sources is post-v0.0.1."* There is no future document type in v0.0.1 scope. Adopting a framework for a deferred use case is premature abstraction.
2. swiftide replaces only embedding-and-store, the simplest part of the pipeline. The AE-specific frontmatter parsing (parser.rs) requires a custom loader/transformer anyway — the saving is partial.
3. swiftide pre-1.0 has breaking changes per minor version (v0.31 → v0.32 changed toolspec API and node typing). Solo-project upgrade churn cost.

**TL synthesis: SKIP swiftide for v0.0.1.** The 2-of-3 weight (standards-expert + challenger) is grounded in explicit "deferred" language in the rebuild plan. standards-rag's framework-for-future-extensibility argument is the kind of premature abstraction that karpathy.md (loaded via CLAUDE.md global) explicitly warns against: *"No 'flexibility' or 'configurability' that wasn't requested."*

#### Other libraries — short verdicts

- **fastembed-rs**: KEEP. Confirmed best-in-class for local Rust embedding. v5.13.4 released same day as this survey (2026-04-27); active maintenance by Anush008/Qdrant org.
- **LanceDB**: DEFER with trigger. Right long-term embedded vector DB if mengdie scales to 10k+ memories. Not now; migration cost ~1500 LoC across db / schema / vector / search.
- **Qdrant**: SKIP. Single-binary constraint hard rule.
- **Tantivy**: SKIP for now. FTS5 unicode61 adequate. Revisit if multilingual recall becomes measurable bottleneck or corpus > 500k tokens. v0.26.1 released 2026-04-21 (recent); active.
- **arroy**: SKIP. Build-once index incompatible with streaming ingest.
- **candle / mistral.rs / ollama-rs**: SKIP. Local LLM synthesis is post-v0.0.1.
- **async-openai**: ADOPT as second LlmProvider impl alongside ClaudeCliProvider, primarily for the local oMLX endpoint (OpenAI-compat). rig wraps async-openai internally so this comes naturally if rig::Extractor lands.
- **Anthropic community Rust clients**: SKIP. Too immature (75 stars max). ClaudeCliProvider remains primary Anthropic path; rig's built-in Anthropic provider is the safer fallback if HTTP is ever needed.
- **text-splitter**: OPTIONAL. v0.0.1 ingests pre-structured AE-distilled facts; chunking not needed. Add only if raw document ingestion is added post-v0.0.1.
- **duckdb-rs**: SKIP. No native vector search, no FTS — wrong shape for mengdie's data.

### Challenges & Disagreements

The cross-cutting lenses produced **three mutually exclusive strategic directions**:

#### Direction 1: Surgical refactor

Replace vector.rs with sqlite-vec; optionally replace synthesis.rs's JSON parser with rig::Extractor (pending verification). Total touched: ~200–500 lines. RAG architecture preserved.

Backed by: arch-intelligence + standards-expert + standards-rag (partially).

#### Direction 2: API-first pivot

Delete embeddings / vector / search / synthesis / clustering modules. mengdie becomes a catalog + policy engine in front of OpenAI File Search + Responses API + Batch API. Cost ~$1/month at solo scale. ~1500 lines of mengdie src/ deleted.

Proposed by: codex-proxy.

**Challenger rejection on 4 grounds:**
1. **Credential sovereignty violation.** CLAUDE.md explicitly states *"mengdie never touches secrets — credentials delegated to claude CLI."* OpenAI File Search requires mengdie to hold or proxy an OpenAI API key. **Direct reversal of a locked decision**, not a tradeoff.
2. **Vendor dependency.** Every MCP tool call becomes an OpenAI API roundtrip. Worse operational story than local SQLite + fastembed.
3. **Offline failure.** mengdie runs as stdio MCP server invoked synchronously during Claude Code sessions. API-dependent design fails silently when network is unavailable. Current design has no such failure mode.
4. **Structural bias.** codex-proxy's recommendation is structurally predictable from its OpenAI-ecosystem role, not from project-specific evidence.

**TL synthesis verdict: Direction 2 is rejected.**

#### Direction 3: Do-nothing on src/ + wire AE plugin

v0.8.0 runs. 214 memories in production. Search, ingest, synthesis all work. The genuine gap toward "AE 的大脑" thesis is daemon integration + ae:analyze Round-0 injection — both **AE-plugin** changes, not mengdie src/ changes. Library swaps don't move the needle on the actual missing piece.

Proposed by: challenger Phase 2 (unaddressed in rebuild plan).

**What this option delivers**: The AE feedback loop the v0.0.1 thesis declares as priority #1. May ship faster than any library refactor.

**What this option doesn't deliver**: 自成长 / reflection (Phase 0 deferred items). But neither does Direction 1 — the proposed library swaps are *orthogonal* to 自成长.

#### Coherence concern that cuts across all directions (challenger)

The rebuild-plan thesis (AE 的大脑 + 自成长 + reflection) is **more ambitious** than v0.x. The library replacements being recommended are a **modest delta**. They don't move 自成长 forward. **If 自成长 is the goal, v0.0.1 BLs should be about reflection design** (Phase 0 deferred questions: trigger model, meta-fact confidence, single-vs-split table) — not library swaps.

#### gemini-proxy — concrete reflection design starting point

Independently of the library question, gemini-proxy supplied concrete recommendations for the 自成长 mechanism that Step D will need:

- **Trigger**: hybrid (event-driven primary [ingest threshold, entity hot-spots, temporal boundaries] + cron secondary + on-demand)
- **Schema**: split-table (facts + meta_facts + relationships) for hierarchical distillation
- **Provenance**: structured (`originating_fact_ids`, `generator_llm_version`, `prompt_used`, `valid_from`/`to`, `contradiction_history`) — not a single boolean
- **Self-correction**: two-pass synthesis (generate → validate against source facts) using structured output / JSON schema enforcement
- **Long-context vs RAG**: keep RAG as primary; long-context Gemini augments retrieval, doesn't replace it

This is directly usable as input to Step D regardless of which Direction (1/3) is chosen for v0.0.1 src/ work.

## Summary

**Library verdicts** (compiled, in adoption-priority order):

- **ADOPT (qualified)**: sqlite-vec — pending static-vs-dynamic distribution verification (replaces vector.rs)
- **ADOPT optional**: async-openai — second LlmProvider impl for oMLX endpoint (only if a 2nd provider is needed)
- **KEEP**: fastembed-rs (already in use, confirmed)
- **CONTINGENT-ADOPT**: rig::Extractor for synthesis.rs — IF subprocess streaming verification passes (~50-line spike)
- **SKIP**: swiftide, rig CompletionModel trait wrap, Qdrant, candle, mistral.rs, ollama-rs, arroy, duckdb-rs, community Anthropic clients
- **DEFER (with trigger)**: LanceDB (corpus > 10k OR p95 latency > 50ms), Tantivy (multilingual recall poor OR > 500k tokens)

**Net library footprint change for v0.0.1 (if Direction 1 chosen and verifications pass)**: +sqlite-vec, +async-openai (optional), partial rig dep for Extractor only. ~200–500 LoC touched in mengdie src/. The Cargo.toml grows by 1–3 lines.

**Strategic divergence — Step B must pick one, not ship a middle ground:**

| Direction | What it does | TL synthesis assessment |
|---|---|---|
| **Surgical refactor** | sqlite-vec + maybe rig::Extractor for JSON parsing | Modest delta from v0.x. Doesn't address 自成长 thesis. Defensible but orthogonal to rebuild-plan's stated motivation. |
| **API-first pivot** | Delete RAG infra, OpenAI File Search front-end | **Rejected**. Violates credential-sovereignty (CLAUDE.md locked decision); breaks single-binary offline operation. |
| **Do-nothing on src/ + wire AE** | v0.8.0 stays; build daemon + Round-0 injection in AE plugin | Most directly serves the AE 的大脑 thesis. Library swaps deferred to triggers. **Should be on Step B's table explicitly.** |

**Open question for Step B**: which thesis is v0.0.1 actually committed to?
- If "AE 的大脑" → Direction 3.
- If "自成长 / reflection" → none of the proposed libraries help; need fresh design BLs (gemini's hybrid trigger + split-table is starting point).
- If "fix v0.x reinventions" → Direction 1, but the karpathy guideline ("don't refactor things that aren't broken") rejects this motivation. The v0.x intelligence layer empirically works (13–14 clean syntheses per production run).

Step B should pick the thesis FIRST, then pick the library subset that serves it.

## Possible Next Steps

→ `/ae:discuss docs/discussions/026-rust-oss-survey/` jointly with `docs/discussions/025-functional-inventory/` for Step B integration strategy. Recommended discussion topics:

1. **Direction selection** (gating decision) — Surgical / Do-nothing / hybrid?
2. **sqlite-vec verification spike** — clone + `cargo add` to fresh workspace; check `cargo build --release` produces self-contained binary or requires runtime `.dylib`. ~15 minutes. Decides ADOPT vs DEFER.
3. **rig::Extractor verification spike** — 50-line proof: does `rig::Extractor<SynthesisDraft>` parse claude-CLI subprocess output correctly? Decides synthesis.rs replacement.
4. **swiftide** — Closed: SKIP (unless someone champions a counter-case).
5. **Reflection / 自成长 design** — Phase 0 deferred items. Pick up gemini-proxy's hybrid trigger + split-table + structured provenance recommendation as starting point. Decide v0.0.1 vs post-v0.0.1 scope.
6. **AE plugin daemon + injection** — file as v0.0.1 BLs against the AE plugin (not against mengdie src/). Independent of all library decisions.
