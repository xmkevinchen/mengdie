---
id: "v1-rebuild-plan"
title: "mengdie v1.0 rebuild plan"
type: plan-doc
created: 2026-04-27
status: research-phase
supersedes: "v0.x (frozen at v0.8.0)"
---

# mengdie v1.0 rebuild plan

## Why rebuild

v0.x development surfaced a structural mismatch between mengdie's
implementation strategy and its actual goal. Reframe driven by user
chat 2026-04-27:

- **"重复造轮子"**: mengdie's ~150K LOC is ~70-80% reimplementation of
  what mature Rust libraries (swiftide RAG framework, rig LLM agent
  framework, Qdrant/LanceDB vector stores) and open-source memory
  tools (Letta, mem0, Zep) already provide. Vector search, RAG
  pipeline, ingestion, clustering, synthesis — all standard plays
  mengdie wrote from scratch.
- **"schema 全删都行"**: data is reproducible from `docs/discussions/`,
  `docs/plans/`, `docs/reviews/`. The whole "plan 017 v5 migration safety
  net" complexity (3 pre-checks + transactional + abort-loud + cluster-hash
  invariant + trigger-based CHECK fallback) was scaled for "production data
  is precious" assumption that doesn't apply.
- **"差 Karpathy LLM-wiki 好远"**: mengdie's heavy infra was scaled for
  "LLM is expensive primitive" assumption that's increasingly outdated
  (Claude 200K window, Gemini 1M+ window). Modern LLMs can read 30
  markdown files and reason directly; mengdie's pre-abstraction
  (clustering / dreaming / synthesis row pre-computation) is solving a
  problem that may not exist anymore.
- **"AE 加工的信号给 mengdie"**: correct division of labor. AE plugin
  uses Karpathy LLM-wiki style for in-session LLM-driven processing
  (read raw markdown, summarize, decide). mengdie receives AE-distilled
  propositional facts as ingest input — not raw markdown. This puts
  the "expensive abstraction" work in the right place (AE skill
  prompts, where host LLM has full context) and leaves mengdie as a
  retrieval engine + sediment store.
- **"RAG 要用，怎么用是问题"**: retrieval engine stays. What gets cut
  is mengdie-side LLM-driven generation (synthesis row pre-computation)
  and mengdie-side ingestion-time abstraction (clustering, semantic
  chunking, etc.). Generation moves to AE plugin (Karpathy-in-session).
- **"mengdie 能支持 LLM 调用没问题，代码已经在那"**: don't throw away
  `LlmProvider` trait + `ClaudeCliProvider` impl. They're clean
  building blocks; reuse them for the *much smaller* reflection
  capability v1 still needs.

## v1 thesis

Three goals (user 2026-04-27):

1. **mengdie = AE 的大脑**. Not a generic personal second brain. v1
   serves AE plugin first; other ingestion sources are post-v1.
2. **mengdie 能自成长**. As more projects accumulate, mengdie produces
   meta-knowledge (patterns observable only across multiple decisions
   / multiple projects). "Knowledge becomes systematic" via reflection,
   not just retrieval.
3. **开源 deferred**. v1 is a personal-use rebuild; not yet
   accountable to external users.

## v1 form (working understanding, not committed)

**Not** a from-scratch rewrite. Incremental simplification:

- **Keep building blocks**: `LlmProvider` trait + `ClaudeCliProvider`,
  `embeddings.rs` (fastembed-rs), `project.rs` (git-remote project_id
  inference), `parser.rs` (AE frontmatter parser, retained for AE
  files even though v1 ingest is push-mode), `mcp_tools.rs` (MCP server
  wrapping), `config.rs`, `metrics.rs`, simplified `contradiction.rs`.
- **Rewrite (clean schema)**: `db.rs` + `schema.rs` — drop the v0.x
  cluster_hash invariant, the migration history (v1→...→v5+), the
  partial-index trickery. Single fresh schema; data reproducible from
  docs/.
- **Replace (orchestration)**: `dreaming.rs` (1326 LOC) + `synthesis.rs`
  (449) + `clustering.rs` (625) + `decay.rs` (219) ≈ 2400 LOC →
  `reflection.rs` ~500 LOC. Same goals, simpler shape.
- **Delete**: `watcher.rs` + `ingest.rs` (push mode replaces watcher);
  `vector.rs` if a Rust crate covers it (TBD by Phase 0 research).

Estimated LOC: v0.x ~150K → v1 ~80K (not the ~500 LOC ascetic version
considered earlier — that one was over-correcting after the
"重复造轮子" reframe; it threw away healthy building blocks too).

## Ingest model (working understanding)

Push mode. AE plugin actively pushes propositional facts to mengdie
after `ae:discuss` / `ae:plan` / `ae:review` / `ae:retrospect`
completes. Facts are LLM-distilled propositions (e.g.,
`[plan-017 review]: cluster-hash invariant must be enforced at DB layer`),
not raw markdown.

External materials (web articles Kai reads) flow in via AE pipeline
first: `ae:analyze` an article → conclusion → AE skill extracts facts
→ push to mengdie. No direct external ingest path in v1.

## Retrieval model

Hybrid: FTS5 (lexical) + vector (semantic) + RRF merge. Same as v0.x
in spirit, simpler in implementation. Per-project + global scope
flags.

Returns ranked facts to AE skill. Host LLM (in AE skill context)
reasons over returned facts to produce verdicts ("this was tried in
project Y, didn't work because Z"). mengdie itself does not produce
verdicts; it produces high-quality candidate facts.

## Reflection model (the 自成长 capability)

mengdie has its own LlmProvider access (reuse from v0.x).
Reflection trigger options (deferred to research phase):

- **on-demand** via MCP tool `memory_reflect(topic?, scope?)`
- **cron / scheduled** via config
- **threshold-triggered** (e.g., every N new facts in a project)

Reflection action: retrieve relevant facts → call LLM via
LlmProvider to abstract a meta-pattern → write back to facts table
(marked `source_type='meta'` or via separate metadata).

This is the core difference from "pure passive retrieval" (which
v0.x dreaming was supposed to do but became schema-heavy and
mis-scaled). v1 reflection is lightweight: just "summarize the
group, store the summary."

---

## Phase 0 — research items (do these BEFORE filing v1 BLs)

User-prescribed order (chat 2026-04-27):

### Item 1: Survey open-source Rust libraries

What well-trodden infrastructure mengdie can adopt instead of
reimplementing. Specific candidates already on the radar:

- **swiftide** (`/bosun-ai/swiftide`) — Rust RAG framework with
  indexing + query pipelines, transformer chain, multiple vector
  store integrations
- **rig** (`/0xplaygrounds/rig`) — Rust LLM agent framework with
  completion + embedding abstractions over multiple providers
- **Qdrant** — Rust-native vector DB
- **LanceDB** — Rust embedded columnar/vector store
- **sqlite-vec** — SQLite extension, callable via rusqlite
- **Tantivy** — Rust full-text search (alternative to SQLite FTS5)

For each: summarize what it does, what mengdie currently reimplements
that it would replace, integration cost, what mengdie still has to
build itself.

### Item 2: Per-library role + mengdie integration strategy

Given Item 1's survey, decide which libraries mengdie adopts and how
it composes them. Output: a clear architecture diagram showing:
- which library handles ingestion / chunking / embedding / storage / retrieval
- where mengdie's own code sits (the integration glue + AE-specific
  parts: project_id inference, AE frontmatter parser, MCP tool
  surface)
- what's deleted from v0.x

### Item 3: mengdie ↔ AE integration design

How does the AE plugin push facts to mengdie? Two patterns surfaced
in chat (user-flagged open question, item 2):

- **Pattern A**: host LLM mediates. AE skill prompts host LLM to call
  `memory_ingest_fact` MCP tool. Token cost per ingest; non-deterministic
  (host may skip).
- **Pattern B**: AE plugin → mengdie direct (server-to-server, bypass
  host LLM). Token-free; deterministic; but mengdie now exposes both
  MCP server (for retrieval queries) and HTTP/IPC server (for ingest).

Decide which (or hybrid). Define the AE skill changes needed:
- Where in `ae:discuss` / `ae:plan` / `ae:review` / `ae:retrospect`
  flows does fact extraction + push happen?
- Fact format contract (what fields are required: title / content /
  knowledge_type / entities / source_path / project_id).

### Item 4: Reflection design (depends on Item 2)

Once Item 2 settles which libraries mengdie uses, design the
reflection mechanism. Specifically:
- Trigger model (on-demand / cron / threshold / hybrid)
- Reflection prompt construction (input: retrieved facts; output:
  meta-fact propositions)
- Storage shape: meta-facts in same table as facts vs separate?
- Confidence handling: meta-facts are mengdie-LLM-derived, may be
  wrong. Tag with confidence? Allow easy invalidation?

---

## Deferred open questions (for Phase 0 → Phase 1 transition)

Three questions surfaced in chat 2026-04-27 that don't block Phase 0
research but must be settled before Phase 1 implementation BLs are
filed:

1. **Reflection trigger** — cron / on-demand / threshold / hybrid?
   (Item 4 above; explicit subquestion of it.)
2. **Meta-fact confidence model** — meta-facts may be incorrect (LLM
   reflection over partial corpus). Single-table boolean flag?
   confidence score? structured provenance trail?
3. **Single-table vs separate-tables for fact + meta-fact** —
   single-table is simpler queries but harder filter "I only want
   meta-facts derived from ≥3 projects." Separate-tables is more
   structure but more SQL. Trade-off TBD.

---

## BL archive note

All v0.x backlog items have been archived to
`.ae/backlog/closed/v0.x-superseded-by-v1/` as of 2026-04-27. 13 BLs
total:

- 9 from `unscheduled/` (audit-collection-discipline,
  clustering-validation, decay-dreaming-pass-optim, decay-threshold-mode,
  enable-pragma-foreign-keys, get-synthesis-with-sources-n-plus-1,
  release-yml-ci-gate, synthesis-preload-db-miss-edge,
  valid-until-boundary)
- 4 from cancelled v0.8.5 sprint (dreaming-module-split,
  memory-ingest-synthesis-source-type-removal,
  synthesis-cluster-hash-not-null-enforcement,
  v5-migration-operator-docs)

These items remain on disk for historical reference. They will not
be re-scheduled in v1; v1 architecture changes obsolete most of
them. Items v1 may revisit (clustering-validation, decay
parameters) will be re-filed as fresh BLs against v1 design once
Phase 0 + 1 settle.

## Branch state

- `main`: v0.8.0 closed; v0.8.5 cancelled. Frozen for v1 work.
- `feature/v1-rebuild`: empty (created 2026-04-27 then user paused
  before any commits). Will resume when Phase 0 research completes
  and Phase 1 BLs are filed.

## Resume checklist

When ready to leave Phase 0 (research) and enter Phase 1 (BLs +
implementation):

1. Items 1-4 above each have a written summary committed.
2. The 3 deferred open questions are answered (in answers may be
   "use this trigger model" / "tag meta-facts with simple
   high/medium/low confidence" / "single table with `source_type`
   discriminator").
3. v1 architecture + integration diagram lands in `docs/`.
4. BLs filed in `.ae/backlog/unscheduled/` for v1 implementation
   work (estimated 4-8 BLs covering: schema, retrieval,
   reflection, AE integration, MCP surface, migration from v0.x
   data if any preserved).
5. `/ae:roadmap plan v1.0.0` to commit BLs into a sprint.
6. `/ae:plan` per BL.

Until that point: no code changes to `src/`. `feature/v1-rebuild`
branch stays empty. v0.x continues to run (operator can still use
`mengdie dream --synthesize` etc.) but receives no new development.
