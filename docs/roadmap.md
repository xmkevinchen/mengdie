---
title: "mengdie multi-version implementation roadmap"
type: roadmap
created: 2026-05-08
status: advisory                  # advisory until per-milestone PRD approves; each milestone may upgrade specific paragraphs to normative
review_date: 2026-08-08            # 90 days; refresh assumptions if any are violated by then
assumptions:
  - "MCP protocol retains de-facto-standard status through 2026 H2 (97M monthly SDK downloads, 67% CTO 12-month adoption per survey §2)"
  - "OSS coding agent ecosystem converges to a Claude-Code-baseline feature set (survey §1)"
  - "mem0 / Letta / Graphiti remain on three distinct technical paths (entity-linking / stateful runtime / bi-temporal KG; survey §3)"
  - "Single-operator development; mengdie does not compete on feature breadth with funded teams ($32M Cline / $10M Letta / Zep YC)"
  - "Karpathy minimum-viable-AE-brain thesis holds (CLAUDE.md FINAL section, 2026-05-05)"
supersedes:
  - docs/discussions/001-product-vision/   # early product vision (4-layer arch, team features) — scope shifted to 1-person personal AE-brain
  - docs/backlog/005-phase2-roadmap.md     # Phase 2.2-2.5 BL-009..014 partly absorbed into vNext/v1.0, partly deleted as over-engineering
superseded_by: ""
source_survey: docs/surveys/2026-05-oss-coding-tooling.md
---

# mengdie — multi-version implementation roadmap

This document is **engineering-blueprint** (not product narrative). For each milestone:
what to build, what OSS to leverage (specific surface, not project name only), what to
self-build, the API surface, and a coarse estimate. Per-milestone PRDs (the next layer
down) are written as each milestone starts (per `blueprint.md` §13 Per-milestone PRD
convention).

The `status: advisory` stance is deliberate: this roadmap is dogfood-driven. Operator
experience with v0.0.1 (and later vNext) revises later-milestone content. v0.0.1 itself
is locked in CLAUDE.md "v0.0.1 thesis" and `docs/v0.0.1-rebuild-plan.md`.

---

## v0.0.1 — Minimum-viable AE-brain (current focus)

- **milestone_goal**: "minimum-viable AE-brain that avoids re-inventing wheels in future"
  (verbatim from operator's CLAUDE.md FINAL section, 2026-05-05). Narrow OSS-adoption
  scope: keep all working in-house code, swap the 2-3 specific surfaces where OSS
  prevents reinvention. Empirically validated: v0.x produced 13-14 syntheses per
  production run; the main pipeline works.

- **oss_wheels** (leverage list — specific surface, with survey citation):
  - **sqlite-vec** — replace `src/core/vector.rs` (264-LoC full-table-scan brute-force
    cosine) with proper ANN. Already a 026-survey ADOPT verdict. Spike to confirm
    static-vs-dynamic linking is BL-026. (per 026 OSS Rust survey)
  - **rig::Extractor** — replace `src/core/synthesis.rs` ~100-LoC brace-depth JSON
    parser with schema-driven LLM extraction. 026-survey CONTINGENT verdict; spike to
    verify subprocess-streaming works for ClaudeCliProvider integration is BL-027.
    (per 026 OSS Rust survey)
  - **fastembed** — already adopted; 384-d local embeddings, all-MiniLM-L6-v2.
    (per 026 OSS Rust survey)
  - **rmcp v1.3** — already adopted; MCP server transport (stdio).
    (per 026 OSS Rust survey; survey §2 "MCP protocol" leverage)
  - **rusqlite + FTS5** — already adopted; hybrid full-text + vector backbone.
    Chinese trigram tokenizer (Plan 004) is the differentiator vs mem0's
    multilingual-generic FTS. (per 026 OSS Rust survey)
  - **claude-CLI subprocess** — already adopted (BL-005 shipped); inherits user
    Claude credentials, no external telemetry. Same "local-first + inherits host
    credentials" pattern as Aider / Cline. (survey §2)
  - **mem0 9-tool MCP description style** — borrow description copy + parameter
    schema conventions when writing `docs/specs/{memory_search, memory_ingest,
    memory_invalidate}.md` in F-004 Step 3. mem0 has settled the industry-standard
    naming and description grammar; mengdie matches the conventions to maximize
    cross-host compatibility. (survey §3; F-004 Step 3 will use this)

- **self_built** (no OSS substitute, or OSS-substitute would erase the differentiator):
  - **AE-pipeline-coupled ingestion** (`parser.rs` / `watcher.rs` / `ingest.rs`) —
    parses `<plan-id>/conclusion.md`, `analysis.md`, `review.md`, `retrospect.md`
    structure. AE-plugin specific; no OSS framework can target this format because
    the format is internal to the AE plugin.
  - **Chinese FTS5 trigram tokenizer** (Plan 004 shipped) — mem0/Letta/Graphiti are
    multilingual-generic; none invest in Chinese-specific tokenization. This is one
    of the three load-bearing v0.0.1 differentiators.
  - **`audit_returned_facts` substrate** (F-002 shipped) — tracks which memories
    were returned by which AE-plugin call, enables retrospect-time evaluation. AE
    plugin specific; no OSS analog.
  - **dreaming / synthesis main pipeline** (`clustering.rs` / `synthesis.rs` /
    `dreaming.rs`) — 13-14 syntheses produced empirically per run; this is the core
    of mengdie's IP. Karpathy "don't refactor what isn't broken" applies.
  - **`contradiction.rs`** (single-temporal validity) — current implementation is
    simple but works. Bi-temporal upgrade is v1.0 candidate (see below).

- **api_surface**:
  - MCP tools: `memory_search` / `memory_ingest` / `memory_invalidate`
  - CLI: `mengdie dream [--synthesize | --decay-dry-run]` / `mengdie import` /
    `mengdie search` / `mengdie stats`
  - Configuration: `~/.mengdie/config.toml` (`[llm]` + `[llm.claude_cli]` sections);
    MCP host registers via `~/.claude/settings.json` for Claude Code.

- **estimate**: 1-2 weeks of focused work after F-004 closes. Path:
  1. F-004 close → unblock BL-026 (sqlite-vec spike, ~2-3 days) — **pre-PRD validation work** (spike's PASS/FAIL goes into v0.0.1 PRD's `oss_wheels` decisions; not gated on PRD existence)
  2. F-004 close → unblock BL-027 (rig::Extractor spike, ~2-3 days) — **pre-PRD validation work** (same rationale)
  3. v0.0.1 PRD (the first per-milestone PRD per blueprint.md §13) → ship gate. Per blueprint §13 "PRD before milestone implementation": **spike outcomes feed PRD; integration + ship work gated on PRD existence**.
  4. BL-008 / BL-025 (AE-plugin wiring) follow PRD landing — these are integration work, gated on v0.0.1.prd.md being written.

---

## vNext — Dogfood-driven refinement + multi-host verification

- **milestone_goal**: Fix the pain-points operator hits during ~1 month of v0.0.1
  dogfood. Verify (not develop) that mengdie's MCP description works in additional
  hosts (Continue / Cline / Cursor). Multi-host expansion is **a testing task, not a
  development task** — same MCP protocol, zero code change required (survey I2).

- **oss_wheels** (leverage list):
  - **mem0 entity-linking prompt** — if v0.0.1 dogfood reveals propositional-fact
    extraction quality is insufficient (e.g., AE-pipeline `analysis.md` topics aren't
    cleanly mapped to memory entities), borrow mem0's LLM extraction prompt template
    (Apache 2.0; portable conceptually, not Python directly). (survey §3)
  - **MCP `resources` primitive** — already in spec but mengdie currently uses only
    `tools`. Exposing memory entries as read-only `resources` is a low-cost expansion
    that increases mengdie's surface in MCP-aware hosts. Implementation contingent on
    `rmcp` v1.x supporting resources (verify before scheduling). (survey §2)
  - **Continue / Cline / Cursor MCP host configurations** — borrow each host's
    MCP-server registration syntax/examples. This is testing scaffolding (verify
    mengdie works as-is in N hosts), not development. (survey §2)
  - **BL-009 absorbed: `memory_dream` MCP tool** (from `005-phase2-roadmap.md`) —
    inline dream during Claude Code session (cluster + return clusters + Claude
    synthesizes inline + Claude calls `memory_ingest`). Promote to vNext only if
    operator dogfood reveals the current `mengdie dream --synthesize` CLI mode is
    inconvenient. (Otherwise stays deferred.)

- **self_built**:
  - **host adaptation test harness** — zero-code-change verification per host:
    a script that spins up a configured host (Claude Code, Continue, Cline, Cursor)
    pointed at mengdie's MCP server, runs `memory_search` + `memory_ingest`, and
    asserts response shape.
  - **operator-dogfood feedback adjustments** — search-ranking tweaks, Chinese-query
    optimization, etc. Specifics determined by what dogfood reveals; don't pre-design.

- **api_surface**:
  - **New**: MCP `resources` (read-only memory entries; if rmcp supports)
  - **New (conditional)**: `memory_dream` MCP tool (from BL-009; only if operator
    needs in-session dream)
  - **Unchanged**: `memory_search` / `memory_ingest` / `memory_invalidate` / CLI
    subcommands

- **estimate**: 1-2 weeks. **Trigger to start**: at least 1 month of v0.0.1 dogfood
  with operator-recorded pain-points, OR MCP-host adoption demand from operator's
  workflow change.

---

## v1.0 — Long-term schema evolution (trigger-driven, not pre-scheduled)

This milestone is intentionally vague at the moment. v1.0 content is dogfood-driven:
operator's experience with v0.0.1 + vNext determines which of the candidates below
fire. Karpathy "don't fix what isn't broken" — none of these fire automatically.

- **milestone_goal**: System upgrade to address pain-points that emerged during
  v0.0.1 + vNext. Specific scope determined by operator-observed failures, not
  pre-decided.

- **oss_wheels** (candidates with explicit trigger conditions):
  - **Graphiti bi-temporal SQL DDL** — borrow `event_time` + `ingest_time` +
    validity-intervals-on-edges schema; upgrade `src/core/schema.rs` from
    single-temporal (`valid_from` / `valid_until`) to bi-temporal. **Trigger**:
    operator empirically observes `contradiction.rs` false-positives that the single
    temporal model can't disambiguate. (survey §3 + I3)
  - **Letta context compaction strategies** — borrow design ideas (not code; Python)
    for `dreaming.rs` synthesis trigger logic. **Trigger**: synthesis cadence
    consistently mismatches operator workflow rhythm in dogfood.
  - **Graphiti as a whole** (knowledge graph backend) — adopt rather than rebuild
    if mengdie genuinely needs multi-entity graph queries. **Trigger**: operator
    explicitly needs query patterns that flat fact retrieval can't serve (e.g.,
    "all decisions made by entity X across project Y in timeframe Z"). (survey §3 (Graphiti intersection) —
    explicitly do not self-build a KG; Graphiti owns this lane.)
  - **mem0 entity-linking redesign** — port the prompt to Rust + adapt to AE-pipeline
    structure. **Trigger**: vNext-borrowed prompt iteration plateaus and a deeper
    entity model is needed.

- **self_built**:
  - **schema migration scripts** (v0.x → v0.0.1 → vNext → v1.0 data movement)
  - **`memory_contradictions` MCP tool** — expose validity intervals to host LLMs.
    Aligns with Graphiti's API shape (survey §3). This is the partial implementation
    of original BL-013 (Knowledge Graph Schema), but constrained to contradictions
    (the v0.0.1-already-implemented concept) rather than a full graph.

- **api_surface**:
  - **New**: `memory_contradictions` MCP tool
  - **Possibly modified**: existing tool descriptions may shift to reflect bi-temporal
    schema if that fires
  - **Unchanged unless triggered**: CLI subcommands

- **estimate**: 2-4 weeks **once triggered**. No calendar commitment; this is a
  trigger-fired milestone, not a pre-scheduled one.

---

## Disposition of `docs/backlog/005-phase2-roadmap.md`

005-phase2-roadmap.md (created 2026-04-16) listed BL-005 through BL-014 across Phase
2.1-2.5. BL-005, BL-006, BL-007, BL-008 already shipped in v0.x. The remaining 6 BLs
are disposed as follows under this roadmap:

| BL | 005's plan | Disposition under new roadmap | Reason |
|---|---|---|---|
| BL-009 | MCP Dream Tool (`memory_dream`) | **Absorbed into vNext** (conditional) | Useful enhancement if operator dogfood reveals CLI dream is inconvenient. Defer until trigger fires; do not pre-build. |
| BL-010 | Daemon + Job Queue | **Deleted** | Over-engineering for 1-person scale. `resources/com.mengdie.dream.plist` (launchd) already covers periodic dreaming. Daemon-class infrastructure is mem0/Letta-class scope; mengdie does not compete on this lane. |
| BL-011 | Async Entity Extraction on Ingest | **Deleted** | AE-pipeline ingestion already produces structured entities (frontmatter `tags:` field per AE plugin convention). LLM-driven async extraction would re-extract what's already structured upstream. If extraction quality becomes insufficient, vNext borrows mem0's prompt synchronously, not async. |
| BL-012 | LLM-Based RAG Search (`memory_query`) | **Deleted** | Host LLM (Claude Code, Continue, etc.) is itself a RAG layer; mengdie returning top-N results to the host LLM is the cleaner separation. Adding a `memory_query` that does internal RAG inside mengdie duplicates the host's job. |
| BL-013 | Knowledge Graph Schema + Typed Edges | **Partly absorbed into v1.0** as `memory_contradictions` | Full KG schema is Graphiti's territory (survey §3 (Graphiti intersection) — adopt Graphiti, don't rebuild). The contradictions-only subset is in scope for v1.0 because mengdie already tracks `valid_from`/`valid_until` and exposing them is incremental. |
| BL-014 | Feedback Signal + RL-Like Tuning | **Deleted** | 1-operator dogfood produces feedback through observed search-ranking quality + manual roadmap adjustments. Full RL-tuning infrastructure is a funded-team-class scope (mem0/Letta have it because they have many users; mengdie has 1). |

005-phase2-roadmap.md frontmatter will be updated to `status: superseded` with
`superseded_by: docs/roadmap.md`. The file is preserved as historical context (don't
delete the file body).

---

## Disposition of `docs/discussions/001-product-vision/`

001-product-vision (2026-04-05) was the founding product-vision document. It
described a 4-layer architecture, team features (Layer 4, "3-15 person teams" target
users), GitHub webhooks, Slack integration, etc. The project pivoted to 1-person
solo scope by 2026-04-27 (v0.0.1 rebuild start) without ever formally superseding
this document. F-004 closes that loop:

- `docs/discussions/001-product-vision/index.md` frontmatter: `status: superseded`,
  add `superseded_by: docs/roadmap.md`.
- `docs/discussions/001-product-vision/product-vision.md`: prepend a supersession
  blockquote at the top pointing to this roadmap. The body is preserved as
  historical context (the early scope ideas may have value if mengdie ever pivots
  back to multi-user — but that's an explicit non-trigger here).

---

## Cross-references

- Survey (factual evidence): [`docs/surveys/2026-05-oss-coding-tooling.md`](surveys/2026-05-oss-coding-tooling.md)
- System blueprint (long-lived identity): [`docs/blueprint.md`](blueprint.md)
- v0.0.1 thesis (operator clarification, 2026-05-05): `CLAUDE.md` "Project Status" section
- v0.0.1 sprint outline: [`docs/v0.0.1-rebuild-plan.md`](v0.0.1-rebuild-plan.md)
- Per-milestone PRD convention: blueprint.md §13 (added by F-004 Step 4)
- Doc-stack roles: blueprint.md §14 (added by F-004 Step 4)
