---
id: blueprint
title: "mengdie — system blueprint"
type: blueprint
created: 2026-04-27
updated: 2026-05-08
status: draft
version: 0.2
revision_log:
  - "2026-05-08: F-004 increment: §5 milestone mapping, §7 expected trigger version, §13 per-milestone PRD convention, §14 doc stack roles"
---

# mengdie — System Blueprint

This document is **long-lived** (not version-bound). It defines what
mengdie is. Migration paths and sprint scopes live in
`docs/v0.0.1-rebuild-plan.md` and per-version roadmaps; this document
is the moving target they aim at.

---

## 1. What mengdie is

> **mengdie is the operator's archive of AI-assisted decision-making,
> with provenance and temporal validity, wired into whatever AI tools
> the operator uses.**

Concretely, mengdie:

- **Ingests** structured AE pipeline outputs (conclusion / plan /
  review / discussion / retrospect / analysis) as facts.
- **Stores** them locally with full provenance (where the fact came
  from, when, in which project) and temporal validity (when it was
  recorded, when it became invalid, what superseded it).
- **Returns** relevant prior facts on query, ranked by relevance and
  recency, so the AI tool calling mengdie sees the operator's prior
  conclusions before doing new work.
- **Reflects** in the background — promotes frequently-recalled facts
  to long-term memory, decays unused facts, and synthesizes meta-facts
  from clusters of related facts.
- **Connects** to current AI tools via current standard transports
  (MCP today; whatever's standard tomorrow).

The stored facts are the operator's. They persist independently of
any specific AI tool. If the AI tool ecosystem changes — or if all
current AI tools disappear — the archive is still useful as a
queryable record of the operator's decision history.

The integration with AI tools is the path by which the archive
delivers ongoing value: the operator's prior thinking informs the
operator's next thinking. The integration is intentionally mechanical
and replaceable; the archive is the durable thing.

### What mengdie is not

- Not chat memory (ChatGPT Memory / Letta / mem0 — those track
  conversation state)
- Not generic document RAG (NotebookLM / "ask my notes")
- Not a code index (Cursor `@codebase` / Continue `@docs`)
- Not a general-purpose PKM "second brain" (Obsidian / Reflect)
- Not a multi-user / multi-tenant / SaaS system
- Not a generic ingestion endpoint (no arbitrary markdown / PDF / chat
  transcripts — only AE pipeline artifacts and similarly structured
  outputs from agentic-engineering-style workflows)

---

## 2. Core promise

When Claude (or whatever AI tool) runs the AE pipeline on a question,
the AI sees, before doing any new research:

> "You discussed this question in project P1, discussion 023 —
> concluded Y. You revisited it in project P2, plan 015 — concluded Z,
> superseding Y. Across projects you tend to prefer Z-shaped solutions
> for X-shaped problems."

Without mengdie, every AE invocation starts blank. Prior discussions,
superseded conclusions, and pattern preferences are invisible.

The "ship one thing" test: if v0.0.1 only delivers one thing, it is
this end-to-end loop:

> AE artifact lands → mengdie ingests → on the next AE invocation in
> the same area, the AI sees the relevant prior facts before
> researching → the AI's output reflects that context → the next
> artifact lands enriched → loop continues.

Everything else (deeper synthesis, contradiction handling, bidirectional
update, advanced reflection triggers) makes this loop better. Nothing
that doesn't make this loop better belongs in v0.0.1.

---

## 3. Conceptual model

The data model is implementation-independent. Any storage backend
(§7) must preserve all of:

### 3.1 Inputs

AE pipeline outputs:
- `conclusion.md`, `plan.md`, `review.md`, `retrospect.md`,
  `analysis.md`

Each ingested file produces one or more **facts** with metadata:
- `id` — stable identifier
- `content` — the fact text
- `entities` — concept tags (drives contradiction detection and
  meta-fact clustering)
- `source_file`, `source_type`, `project_id` — provenance
- `valid_from` — when the fact was recorded
- `valid_until` — when it was superseded (NULL = currently valid)
- `superseded_by` — pointer to the fact that invalidated this one
- `recall_count`, `avg_relevance`, `last_recalled` — usage signal
- `is_longterm` — promoted-to-long-term-memory flag

### 3.2 Operations

Four primary operations, exposed as MCP tools and CLI subcommands:

1. **Ingest** — accept an artifact, extract facts, run contradiction
   check, store with provenance.
2. **Search** — given a query and scope, return ranked facts with
   provenance and temporal validity. Hybrid retrieval (full-text +
   vector + reciprocal rank fusion + recency / longterm boost).
3. **Contradict** — when a new fact's entity set overlaps an existing
   fact's, compare them; if they conflict, set the older fact's
   `valid_until` and `superseded_by` (logical invalidation; history
   is retained).
4. **Reflect** — periodic background pass:
   - Promote frequently-recalled facts to long-term
   - Decay unused long-term facts toward demotion
   - Cluster related facts and synthesize meta-facts (one fact that
     summarizes a pattern across many)
   - When a new fact arrives, re-evaluate the related facts in its
     entity cluster (bidirectional update — A-MEM pattern)

### 3.3 Output

MCP tool surface, registered in Claude Code's `~/.claude/settings.json`:
- `memory_search` — primary query interface
- `memory_ingest` — primary write interface
- `memory_invalidate` — explicit supersession when needed manually

The AE plugin's `/ae:analyze` Round-0 injection is the canonical
caller of `memory_search`. Other AE skills (`/ae:plan`, `/ae:discuss`)
are secondary callers.

---

## 4. Differentiation

| Existing tool | Why it cannot replace mengdie |
|---|---|
| ChatGPT Memory | Black box, cross-tool dead-end, no provenance, no temporal validity |
| Letta core memory | Agent-self-editing model; mengdie is called from outside, not by an in-loop agent |
| mem0 | Generic chat memory; doesn't understand structured AE pipeline outputs |
| Graphiti | Closest in spirit (bi-temporal + provenance) but designed for entity extraction from conversation, not AE-pipeline-output ingestion; Python-only |
| Obsidian Smart Connections | For human-written notes; no concept of AE state-machine artifacts |
| NotebookLM | Notebook boundary isolates corpora; no autonomous reflection; cloud-only |
| CLAUDE.md / cursorrules | Static rules; cannot evolve, cannot be invalidated, cannot trace provenance |

The unoccupied space mengdie fills, in one line: **AE-aware ingestion
+ cross-project + self-evolving + locally inspectable**. No
commercial or OSS tool combines all four.

---

## 5. Function priority

**P0** — without these, the core promise (§2) does not hold:
- AE artifact ingest path (push from AE plugin, or pull via watcher
  daemon — see §8) → **v0.0.1** (shipped: parser.rs/watcher.rs/ingest.rs)
- `memory_search` returning provenance + temporal validity → **v0.0.1**
  (shipped; spec at docs/specs/memory_search.md)
- AE plugin `/ae:analyze` Round-0 injection (the operator's plugin
  needs to actually call mengdie — without this, the loop never
  closes) → **v0.0.1** (BL-008/BL-025 AE wiring; gates on v0.0.1.prd.md)
- Basic instrumentation: log every search call, what was returned,
  what was used (so the operator can verify the loop is closing,
  not just looks closed) → **v0.0.1** (F-002 `audit_returned_facts`
  substrate shipped)

**P1** — make the core promise robust:
- Bi-temporal contradiction handling (set `valid_until` on superseded
  facts; retain history) → **v0.0.1 partial** (single-temporal active
  in `contradiction.rs`); **v1.0 full** (bi-temporal `event_time` +
  `ingest_time` borrowed from Graphiti per docs/roadmap.md v1.0)
- Bidirectional update on ingest (when a fact arrives, re-evaluate
  related facts in its entity cluster — A-MEM pattern) → **v1.0+
  trigger** (per §12: corpus ≥1k AND ≥5 supersession-within-7-days
  events / 30-day window from persisted domain audit table)
- Cross-project search (default per-project; opt-in cross-project) →
  **v0.0.1** (shipped; `scope: "global"` MCP param)

**P2** — fulfill the self-evolving promise:
- Synthesis / dreaming (cluster related facts, LLM synthesizes a
  meta-fact, store with link table) → **v0.0.1** (BL-006/007 shipped;
  13-14 syntheses per production run empirically)
- Reflection trigger model (count threshold + entropy + on-demand,
  vs. cron — see §8) → **vNext+** (027 T2 settled on-demand default +
  `ReflectionTrigger` trait; trait implementation deferred to vNext)

**P3** — storage / implementation optimization:
- sqlite-vec adoption (replaces hand-rolled brute-force vector search) →
  **v0.0.1** (BL-026 spike pending; integrate after PASS)
- Storage layer migration along the §7 ladder when triggers fire →
  **per-tier triggers** (Tier 1 = v0.0.1 default; Tier 2 = v1.0+;
  Tier 3-4 = post-v1.0 trigger-fired only)

If a P3 item gets prioritized over a P0 item, something has gone
wrong in scope review.

---

## 6. Implementation principle

> **Unless no industry reference exists, do not build it ourselves.**
> mengdie is a glue layer over mature OSS, not a re-implementation of
> commodity infrastructure. Custom code is justified only for
> AE-specific semantics that no OSS framework addresses.

### Operational definition of "industry reference exists"

A library counts as an industry reference for mengdie's purposes when
ALL of:

1. OSS, public source (not SaaS, not closed-source API)
2. Commit in the last 90 days
3. ≥2 contributors with commits in the last 180 days (excludes
   single-maintainer hobby projects), OR ≥500 stars if pre-v0.1.0
4. Compatible with Rust — native crate OR stable C FFI callable from
   Rust without a runtime dependency
5. At least one production user other than the maintaining org

Plus three exclusions:

- **Academic papers without a maintained OSS implementation do not
  count.** Citing "Reflexion" as a reference for reflection is not
  valid unless an OSS library ships it as a callable primitive
  meeting the five criteria. Papers count as design inspiration only.
- **SaaS-only products do not count**, even if they have a self-hosted
  OSS tier, if the OSS tier is materially feature-incomplete vs the
  SaaS tier.
- **Python-only libraries do not count** for mengdie unless there is
  a stable FFI or subprocess interface that does not require shipping
  a Python runtime.

### Categories

**Adopt** (commodity layers with industry references):
- LLM provider abstraction → `rig` (handles HTTP-API providers like
  OpenAI / Anthropic / Cohere; ClaudeCliProvider impl stays as
  mengdie-side code that conforms to rig's `CompletionModel` trait)
- Indexing pipeline → `swiftide` (subject to verification of
  contributor count and version maturity, see §10)
  <!-- 2026-05-08 update: 026 OSS Rust survey verdict = SKIP swiftide
       (multi-contributor + active but stack-incompatible with
       mengdie's narrow scope; per docs/discussions/026-rust-oss-survey/
       analysis.md). Body retained for §11 audit-trail invariant
       ("revised when §8 question resolved" — §8 doesn't cover this);
       per F-004 review fixup. Replace with rig::Extractor narrow
       borrow in BL-027 spike (per docs/roadmap.md v0.0.1 oss_wheels). -->
- Vector storage Tier 1 → `sqlite-vec` (subject to bundled-rusqlite
  compatibility verification, see §10)
- Local embedding → `fastembed-rs` (already in use; confirmed)
- MCP transport → `rmcp` (already in use; confirmed)
- Graph storage when needed (§7 Tier 2+) → `Kuzu` (subject to
  verification, see §10)
  <!-- 2026-05-08 update: per docs/roadmap.md v1.0 oss_wheels, if KG
       is needed mengdie adopts Graphiti (which already wraps Kuzu/Neo4j)
       rather than Kuzu directly. Reverse-leverage rule from F-004
       roadmap §D: don't rebuild what Graphiti has done. Body retained
       per §11 audit-trail invariant; re-evaluate at v1.0 trigger. -->


**Build** (no adequate industry reference; custom code justified):
- AE pipeline artifact ingestion (no OSS treats AE-style structured
  decision artifacts as primary input)
- Cross-project meta-fact synthesis (frameworks cluster within a
  project; none synthesize across project boundaries)
- Provenance-weighted retrieval (AE conclusion ranked higher than
  raw prose)
- AE plugin Round-0 injection contract (the call-site protocol is
  AE-specific)
- Loop instrumentation (no OSS pattern for measuring "is the AI
  feedback loop spiraling up" at solo-operator scale)

**Borrow design, build in Rust** (OSS reference exists but
language-incompatible or architectural-fit issues):
- Bi-temporal contradiction model — design from Graphiti
  (Python-only); implementation in Rust against the §7 backend tier
- Reflection trigger (count + entropy + on-demand) — design from SCM
  paper + LangMem debounce pattern; Rust implementation
- Bidirectional fact update — design from A-MEM paper; Rust
  implementation

---

## 7. Scalability ladder

The conceptual model in §3 is implementation-independent. The
implementation evolves along this ladder; each tier is reached only
when its trigger fires.

| Tier | Storage | Deployment | Trigger to advance | Expected trigger version |
|---|---|---|---|---|
| **1** | SQLite + sqlite-vec; bi-temporal logic in SQL | Single binary | Cross-project graph traversal queries become a regular need; OR contradiction chains exceed ~3 hops and SQL recursion gets ugly | **v0.0.1 default** (current target) |
| **2** | + Kuzu (embedded graph DB, file-based) | Single binary, two stores | Graph data exceeds Kuzu's practical limit; OR graph algorithms in Kuzu lag specialised graph DBs | **v1.0+** (corpus + graph-query demand from operator dogfood) |
| **3** | + FalkorDB / Neo4j (separate process) | Multi-process locally | mengdie's hand-rolled bi-temporal logic falls behind Graphiti's evolution; OR community clustering / saga summaries become first-class needs | **post-v1.0 trigger** (only if hand-rolled bi-temporal lags Graphiti) |
| **4** | Delegate the graph layer entirely to Graphiti's MCP server | mengdie thin (~1k–2k LoC) + Graphiti MCP + graph DB | Graphiti API stabilises post-v1.0 and proves long-term commitment; mengdie's custom Rust graph code becomes a maintenance burden | **future trigger only** (not currently planned; reverse-leverage path per docs/roadmap.md §D) |

**Default for v0.0.1: Tier 1.** The data model is designed to migrate
forward without breaking changes.

---

## 8. Open questions

These do not block §1–§5 but must be resolved by `/ae:discuss` before
P1 or P2 work begins.

<!-- 2026-05-08 update (per F-004 review fixup): Q1 and Q2 settled
     by discussion 027. Q5 partially settled by F-002 audit substrate.
     Body retained per §11 audit-trail invariant. -->

1. **Ingest mechanism: push or pull?** AE plugin explicitly calls
   `memory_ingest` after each pipeline phase (push), OR mengdie runs
   a watcher daemon over the AE output directory (pull). Push is
   simpler, mirrors how MCP tools are normally driven, and decouples
   from filesystem. Pull was the v0.x default but the daemon was
   never wired. **Recommendation: push as v0.0.1 default; watcher
   library remains as opt-in fallback.**
   <!-- ✅ RESOLVED 2026-05-05 by 027 T1: push-primary, watcher.rs
        as opt-in library, cmd_import for cold-start. See
        docs/discussions/027-industry-state-2026/conclusion.md. -->

2. **Reflection trigger.** Cron / salience-threshold (Generative
   Agents) / composite (count + entropy + elapsed time, SCM) /
   debounced submit-dedupe (LangMem) / on-demand. Pick one as v0.0.1
   default; others remain triggers for evolution.
   <!-- ✅ RESOLVED 2026-05-05 by 027 T2: on-demand default +
        ReflectionTrigger trait for swap-in alternatives. Filed
        BL-024 for salience/composite/debounced as deferred BLs.
        See docs/discussions/027-industry-state-2026/conclusion.md. -->


3. **Cross-project default scope.** Currently per-project default,
   cross-project opt-in. Some queries may want global default. Is
   the default a policy decision or a user-config option per call?

4. **Ingest sources beyond AE.** §3.1 restricts inputs to AE pipeline
   artifacts. Does the blueprint allow chat summaries, commit
   messages, issue / PR content as future ingest sources? Or does
   "AE-only" stay as a permanent identity boundary? If permitted,
   what discipline keeps mengdie from becoming a generic memex?

5. **What "loop is closing" looks like measurably.** §5 P0 includes
   basic instrumentation. What's the minimum viable signal that
   confirms the loop is actually delivering value (not just being
   called)?

---

## 9. Out of scope

Locked. Do not revisit without rebuilding the blueprint:

- Multi-user / multi-tenant / SaaS
- Cloud-only operation (local-first is identity, not preference)
- Carrying secrets or API credentials (delegated to the calling
  AI-tool's CLI / SDK; never proxied through mengdie)
- Generic document RAG (no arbitrary markdown / PDF ingestion)
- Code indexing (Cursor / Continue / Aider already do this)
- Real-time collaboration / multi-writer
- Mobile / browser-only deployment
- Enterprise features (RBAC, audit beyond local provenance,
  compliance certifications)

---

## 10. Pre-blueprint verifications

Three small spikes are required before the §6 "Adopt" categories can
be committed to. Each is a few hours of focused work:

1. **sqlite-vec + bundled-rusqlite compatibility.** Cargo workspace
   with `rusqlite = { features = ["bundled"] }` plus `sqlite-vec`,
   verify `sqlite3_auto_extension` registration works against the
   bundled SQLite, run a `vec0` virtual-table query, confirm KNN
   returns expected results. Decides whether sqlite-vec adoption
   ships in v0.0.1 or defers.
2. **swiftide + rig adoption fitness.** Concrete numbers: stars,
   contributor count (last 180 days), commit frequency, version,
   production users. Apply §6 industry-reference criteria. Verify
   rig has subprocess-LLM-dispatch trait support OR confirm
   ClaudeCliProvider stays as a mengdie-side impl of rig's
   `CompletionModel` trait. Decides what to actually adopt.
   <!-- 2026-05-08 update: 026 OSS Rust survey resolved this spike —
        swiftide = SKIP; rig = qualified ADOPT but only via narrow
        rig::Extractor (BL-027 spike); ClaudeCliProvider stays as
        mengdie-side primitive (BL-005 shipped). Spike is closed in
        spirit; line retained per §11 audit-trail invariant. -->
3. **Kuzu / kuzu-rs maturity.** Same fitness check. Decides whether
   §7 Tier 2 is a viable next step or needs revisiting.
   <!-- 2026-05-08 update: per docs/roadmap.md v1.0 oss_wheels,
        Tier 2 KG path adopts Graphiti (not Kuzu directly). This
        spike is deferred to v1.0 trigger or removed entirely
        depending on dogfood. Line retained per §11 audit-trail. -->


These spikes are filed as v0.0.1 BLs, scheduled before any P0 / P1
implementation work that depends on the outcome.

---

## 11. Relationship to v0.x and the v0.0.1 rebuild plan

- **v0.8.0** (current production state): partially implements the
  blueprint. The §3.2 operations exist and work. The §5 P0 ingest
  side works. The §5 P0 Round-0 injection on the AE plugin side is
  **not yet wired** — this is the single most consequential gap.
- **`docs/v0.0.1-rebuild-plan.md`**: the migration outline from v0.x
  reality to blueprint reality. Steps A (inventory) + 0 (industry
  survey) feed this blueprint; Steps B (integration discuss) + C/D
  (interface + reflection) consume it.
- **v0.0.1 sprint commits**: each BL filed against this blueprint
  must reference a specific §1–§7 item it advances. BLs that don't
  trace to a blueprint section are scope creep.

This blueprint is revised when:
- §1 changes (rare; major direction shift)
- A §8 open question is resolved (revise the open question section,
  fold the resolution into §1–§7)
- A §7 ladder tier is reached and its successor's design needs
  specification before migration begins
- A locked §9 out-of-scope item is genuinely re-opened (rare;
  requires a discussion documenting why)

## 12. v0.0.1 architecture decisions (concluded 2026-04-28)

See `docs/discussions/028-v0.0.1-architecture-design/conclusion.md`
for the full Decision Summary. Headline decisions:

- **Storage layer (Tier 1)**: free functions over `&Db`, no
  `Storage` trait. Trait deferred to Tier 2 (Kuzu) trigger.
- **Bi-temporal `event_time` column**: NOT in v0.0.1 schema.
  Alternative: optional `valid_from` parameter on `memory_ingest`
  for bulk import. Re-open path = new discussion when batch-import
  workflow ships.
- **Reflection module consolidation**: deferred pending sqlite-vec
  compatibility spike outcome.
- **`Reflector` trait**: NOT in v0.0.1, regardless of sqlite-vec
  outcome. ANN is similarity-primitive swap, not 2nd reflection
  strategy.
- **A-MEM bidirectional update**: deferred from v0.0.1. Trigger =
  corpus ≥1k AND ≥5 supersession-within-7-days events / 30-day
  window from persisted domain audit table.
- **MCP `memory_search` ACK feedback**: NOT in v0.0.1 contract.
  Triggers must be server-side observable.
- **v0.0.1 P0 instrumentation requirement**: persisted domain audit
  table with separate `audit_returned_facts` link table (FK to
  `memory_entries.id`) — derived from A-MEM trigger needs.
- **Search-split refactor**: IN v0.0.1 sprint (alongside
  `mcp_tools.rs` two-ingest-paths defect fix). search.rs functions
  + `search_vector` move from `impl Db` to module-level.

v0.0.1 sprint structure (per 028 Doodlestein-strategic finding):
two-wave BL ordering with BL B (sqlite-vec spike) requiring an
explicit PASS/FAIL outcome record.

---

## 13. Per-milestone PRD convention (added 2026-05-08, F-004)

Each milestone (v0.0.1, vNext, vN+1, …) ships with a milestone PRD
written **before** milestone implementation begins. PRD location:
`docs/milestones/v<N>.prd.md`. The PRD is the **engineering ship gate**
for that milestone — when AC items in the PRD pass, the milestone
ships.

**Hard cap**: ≤500 words per PRD section (Goal / AC / Out-of-scope /
Ship gate / Roadmap delta). Single-section overflow → split into
follow-up BL, do not inline-expand.

PRD scope (each milestone):
- **Goal**: 1-paragraph statement of what this milestone delivers,
  traceable to a vision pillar in this blueprint or a multi-version
  goal in `docs/roadmap.md`.
- **Acceptance criteria** (AC): 3-7 verifiable conditions. Bash
  spot-checks where mechanical; operator sign-off where judgmental.
- **Out-of-scope**: explicit list of what this milestone is NOT
  delivering. Prevents scope creep mid-implementation.
- **Ship gate**: when all AC pass + out-of-scope list holds + operator
  sign-off → milestone ships, version tag bumped, post-ship retrospect
  filed.
- **Roadmap delta**: what changed in `docs/roadmap.md` due to this
  milestone, or "no change". Binds roadmap update cadence to milestone
  PRD writing — without this section, roadmap can stay perpetually
  advisory (per F-004 codex review fixup).

PRD writing trigger: at the start of milestone implementation. The
v0.0.1 PRD (`docs/milestones/v0.0.1.prd.md`) is the **first PRD
instance** under this convention; it is filed by F-004's follow-up
work, not in F-004 itself.

Rationale (from F-004 council 2026-05-08): the v0.x stack shipped
without per-milestone PRDs, and operator's "完全没头绪" sensation
traced to missing top-down anchors. PRDs are the milestone-level
anchor. Roadmap is multi-version trajectory; per-milestone PRD is the
specific milestone's work definition. Different layers, different
update cadence (roadmap ~quarterly; PRD per milestone start; PRD
becomes immutable after milestone ships, serves as audit trail).

---

## 14. Doc stack roles (added 2026-05-08, F-004)

The mengdie project doc stack consists of 4 layers, each with a
specific role and update cadence:

- **`docs/blueprint.md`** (this document) — long-lived technical
  identity. What mengdie is, conceptual model, scalability ladder,
  function priority. Update cadence: when system identity itself
  shifts (rare; ~yearly).
- **`docs/roadmap.md`** — multi-version implementation trajectory.
  v0.0.1 / vNext / v1.0 milestones with leverage list, self-built
  parts, API surface. Update cadence: ~quarterly OR per dogfood-driven
  trigger.
- **`docs/milestones/v<N>.prd.md`** (per active milestone) — that
  milestone's engineering ship gate. AC, out-of-scope, ship gate.
  Update cadence: written at milestone start; immutable after ship
  (becomes audit trail).
- **`docs/specs/*.md`** (per user surface) — API contract for each
  user-facing surface (MCP tool / CLI subcommand). Signature, params,
  returns, errors, examples, notes. Update cadence: when source code
  changes signature (`as_of_commit` field bumped + `stability`
  managed).

Cross-references:
- BLs (`docs/backlog/`) serve specs serve PRDs serve roadmap serve
  blueprint.
- Acceptance test for a new BL: "this BL serves vision pillar X via
  milestone Y via spec criteria Z." If not answerable → deprioritize.

Single-source-of-truth (SoT):
- Code wins for current signature; specs reflect code (not vice versa).
- PRDs win for milestone ship criteria; roadmap reflects PRD outcomes
  retrospectively (binding mechanism: per-PRD "Roadmap delta" section,
  see §13).
- Blueprint vision elements win for cross-cutting decisions; if
  roadmap or PRD conflicts, blueprint is canonical until explicitly
  superseded.

Update direction (cycle prevention rule):
- API changes flow bottom-up: specs → roadmap → blueprint
  (e.g., adding new MCP tool → write spec → update roadmap api_surface
  → reference in blueprint §5 if cross-cutting).
- Strategy changes flow top-down: blueprint → roadmap → specs
  (e.g., new vision pillar → blueprint update → roadmap milestone
  re-plan → spec stability transitions).
- Following the wrong direction (e.g., roadmap rewrite forcing
  blueprint update) is a signal the change is mis-classified.

Supersession protocol — two patterns supported:

1. **`status: superseded`** — used when a document is replaced by a
   successor that fulfills the same role. Required frontmatter:
   `status: superseded` + `superseded_by: <path>` + `superseded_on: <YYYY-MM-DD>`
   + `superseded_reason: "<rationale>"`. Body retained (audit trail).
   Top blockquote with successor link is **strongly recommended** for
   reader clarity. Example: `docs/discussions/001-product-vision/`,
   `docs/backlog/005-phase2-roadmap.md` (both superseded by
   `docs/roadmap.md` on 2026-05-08).

2. **`status: archived`** — used when a document is **out-of-scope**
   under current direction (no direct successor, but body preserved
   as historical context). Required frontmatter: `status: archived` +
   `archived_on: <YYYY-MM-DD>` + `archived_reason: "<rationale>"` +
   optional `superseded_by:` if a successor exists conceptually. Body
   retained (audit trail). Top blockquote recommended. Example:
   `docs/prd-ae-integration.md` (archived 2026-05-08, conceptual
   successor = blueprint §13 per-milestone PRD convention).

Pattern selection rule: if successor exists and fulfills same role →
`superseded`. If document is out-of-scope without direct replacement →
`archived`. Both retain body; both require dated reason. Future
formalization (template + checklist) tracked as follow-up BL post-F-004.
