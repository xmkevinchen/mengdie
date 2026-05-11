---
agent: minimal-change-engineer
round: 1
created: 2026-04-28
discussion: 028-v0.0.1-architecture-design
lens: YAGNI / refuse scope creep / 3 similar lines > premature abstraction
---

# Round 1 — minimal-change-engineer

Lens: every changed line must trace to a current need. Apply blueprint
§6: "do not introduce abstractions that are not earned by current need."
Apply YAGNI's ≥2-impls-or-in-sprint rule.

## Findings (with file:line evidence)

### Topic 1 — Storage trait + search-split refactor

**Recommendation: search-split refactor, no `Storage` trait. Minimum
change is the function relocation only.**

Two facts in the repo:

- `src/core/search.rs:80` — `impl Db { pub fn search_fts(...) }`. Search
  is grafted onto `Db`'s method surface.
- `src/core/mcp_tools.rs:306-331` — MCP `memory_ingest` reimplements
  the embed → contradiction-check → insert pipeline inline rather than
  calling `ingest::ingest_document` (`src/core/ingest.rs:17`). This is
  the two-ingest-paths defect.

Fixing the defect cleanly (call `ingest::ingest_document` from
mcp_tools) requires zero search-split. The defect is in the **ingest**
side, not the retrieval side. So the search-split is not actually
**induced** by the defect fix as analysis.md line 285 implies.

However, the search-split is independently cheap and obviously
correct: `impl Db { fn search_fts() }` is a layering lie at the type
level. Moving the four search functions (`search_fts`, `search_vector`,
`memory_search`, plus the helper `apply_boost_and_decay`) from
`impl Db` to module-level `search::*(&db, ...)` is mechanical, all
callers are in `mcp_tools.rs` and `cli.rs`, and the change clarifies
the layer model **without adding a single new abstraction**. This is
in scope as "free cleanup" on the same v0.0.1 PR.

The `Storage` trait is a different question. Apply YAGNI:

- Tier 1 SQLite: 1 impl (`Db` in `src/core/db.rs`).
- Tier 2 Kuzu: no commit date, blueprint §10 spike not yet completed.
- No second impl is committed in v0.0.1 sprint.

The trait fails the ≥2-impls rule and fails the in-sprint commitment
rule. **Define the trait when the second impl actually lands**, not
speculatively. Until then `Db`'s concrete methods are sufficient. The
search-split itself is what makes the eventual trait introduction
trivial — the API surface will be small and CRUD-shaped at that
point. Defining the trait now buys nothing; it just locks in a guess
about what the Kuzu impl needs.

The reviewers' "ACCEPT conditional on search-split" position
implicitly assumes "if we're touching this code we should define the
trait" — but YAGNI says no: touch only what's needed, define only what
has a current caller. The conditional reduces to: if we do the
search-split (which I support), then introduce the trait — and the
trait introduction is what's premature, not the search-split.

Mechanism question (trait / struct / free functions / nothing): **free
functions over `&Db`**. This is what the search-split produces
naturally and matches v0.0.1's actual need. No struct, no trait, no
ceremony. When Tier 2 arrives, lift to a trait then.

### Topic 2 — Bi-temporal `event_time` column

**Recommendation: reject (not "defer with trigger") for v0.0.1.**

Falsifiable evidence: `src/core/ingest.rs:17-69` is the single ingest
path. It reads `doc.content` from a parsed AE artifact and inserts
immediately. There is no mechanism to set `event_time ≠ ingested_at`
from the AE flow today, and no caller in `src/` produces such a
divergence. The 214-fact production corpus has the same constraint —
every fact was ingested seconds after artifact write.

The challenger's falsifiable demand (one artifact with > 60s gap) is
the right test. Without it, the column is borrowed schema with zero
observable payoff. Worse, blueprint §6 explicitly says: "do not borrow
patterns that serve another use case if the payoff for mengdie's actual
workflow is not demonstrable." Graphiti's bi-temporal model is borrowed
from chat-derived facts; the AE flow is artifact-derived. Different
shape.

I'd go further than "defer with trigger" and **reject permanently for
v0.0.1**, because:

1. The trigger ("first post-hoc-dated artifact ingested") is not even
   on the operator's roadmap — blueprint §3.1 lists `valid_from` /
   `valid_until` / `superseded_by` but not `event_time`. The current
   model is uni-temporal-with-supersession, which is sufficient for
   the §2 core promise.
2. Adding a column later is not free, but neither is carrying it now
   (migration, every INSERT touches it, every contradiction-check
   query has to consider it). v0.x's contradiction code at
   `src/core/contradiction.rs:48` reads `valid_from` / `valid_until`,
   not `event_time`. Adding the column changes nothing about how
   contradictions are detected today.
3. If post-hoc documentation later becomes a real workflow, that's
   itself a discussion-worthy change and the column can be added
   under a v0.x.y migration with full context.

"Defer with trigger" implies we expect this to fire. I see no
evidence we expect this to fire. **Reject permanently** is the
honest disposition, with the standard "if a real post-hoc workflow
emerges, re-open via a discussion" — same as any other rejected idea.

### Topic 3 — Reflection module collapse

**Recommendation: defer the collapse decision (not the Reflector
trait — that's a separate "still no" by YAGNI).**

The empirical situation from the codebase:

- `src/core/dreaming.rs:1-10` imports `clustering::cluster_memories`
  and `synthesis::{build_synthesis_prompt, parse_synthesis_response,
  ...}`.
- No other module in `src/core/` imports `clustering` or `synthesis`.
- `clustering.rs` = 625 lines; `synthesis.rs` = 449 lines;
  `dreaming.rs` = 1326 lines.

So challenger is empirically right: clustering and synthesis are
single-caller helpers of dreaming. Collapsing them into one file is
defensible.

But: **collapse is cosmetic refactoring, not a v0.0.1 user-facing
goal**. The current 3-file split is not broken. It compiles, it works,
it shipped. The blueprint §5 P0 list does not include "tidy module
boundaries." Spending v0.0.1 effort on this trades a clean diff for
zero new behavior.

Deferring until the sqlite-vec spike resolves is correct because the
spike has a real chance of deleting `clustering.rs` entirely (per 025
CONDITIONAL-DELETE). Deciding the collapse before knowing whether
the file survives is wasted decision energy.

**On the Reflector trait specifically: still no**, even if the
sqlite-vec spike succeeds. Two reasons:

1. The "≥2 impls" reading is shallow. ANN-similarity-via-sqlite-vec
   isn't a *reflection strategy*; it's a *similarity primitive*.
   Reflection is one operation: cluster → synthesize → store. There's
   one reflection algorithm; only the similarity step underneath
   changes.
2. Even if the operator did want both seed-cosine AND ANN as separate
   reflection strategies (unclear why), the swap point is
   `clustering::cluster_memories` — a single function call inside
   `dreaming.rs`. A function pointer or `enum SimilarityBackend` is
   the YAGNI shape, not a `Reflector` trait wrapping the whole pass.

The Reflector trait is solving "what if we want pluggable reflection
policies?" — a question nobody is asking. Refuse.

### Topic 4 — A-MEM bidirectional update deferral trigger

**Recommendation: composite trigger, with the **observable** clauses
load-bearing and the size clause as a sanity floor.**

Concrete proposal:

> Re-open A-MEM bidirectional update when ALL of:
> 1. **Corpus ≥ 1k facts in a single project** (sanity floor; below
>    this, cluster-reevaluation cost is negligible and the question is
>    moot).
> 2. **The persisted domain audit (blueprint §5 P0) shows ≥ 5
>    instances in 30 days where a `memory_search` returned a fact
>    that was later superseded within 7 days of the search** — i.e.,
>    we measurably injected stale context into AI work and a human
>    had to invalidate it shortly after. This is the "retrieval
>    quality degraded" signal made concrete and measurable.
> 3. **One independent third-party replication of A-MEM** appears in
>    OSS or peer-reviewed literature (not a self-citation, not a
>    blog post). Currently: zero. (Optional safety check; can be
>    waived if clauses 1+2 hit hard.)

Why composite, not single-clause:

- **Size-only is wrong**: 1k facts can be perfectly well-served
  without bidirectional update if no fact-evolution is happening.
- **Quality-only is wrong** without the size floor: at 200 facts a
  single bad supersession looks dramatic in percentage terms.
- **External-only is wrong**: the operator's own production
  experience matters more than literature.

The clause-2 metric is **directly measurable from the v0.0.1
instrumentation** — blueprint §5 P0 already requires logging every
search call + what was returned + what was used. Adding a join on
"was that fact later superseded within 7 days" is a SQL query, not
new instrumentation. This satisfies the framing's "measurable using
v0.0.1's instrumentation" constraint exactly.

The 7-day window is a guess — first measurement in production tells
us if it's right. The 5-instances/30-days threshold is a guess too,
but it's calibrated to the operator's actual scale (~214 facts, low
ingest rate); at higher scale the threshold should rise proportionally.

I'd reject the "corpus > 1k facts" single-clause trigger from
analysis.md line 309 as too permissive — it'd fire automatically
just from steady ingestion regardless of whether bidirectional
update is actually needed.

## Agreements

- Two-ingest-paths defect (`mcp_tools.rs:306-331` vs `ingest.rs:17`)
  must be fixed in v0.0.1. Convergent across all reviewers.
- `Transport`, `EventEmitter`, `Reflector` traits all premature.
  Convergent.
- `LlmProvider`, `EmbeddingProvider` traits earn themselves via real
  second impls.
- Wire AE Round-0 first; everything else follows.
- A-MEM defer from v0.0.1 (4-of-4 already converged).

## Disagreements

- **vs reviewers / codex on Topic 1**: I reject the "ACCEPT conditional"
  position on `Storage` trait. Search-split alone is the minimum
  change; the trait should wait for an actual second impl. The
  conditional buys speculative future flexibility for current cost.

- **vs framing on Topic 2**: I propose **reject permanently** rather
  than "defer with trigger." No evidence the trigger will fire; no
  workflow on the operator's roadmap requires it; standard "re-open
  via new discussion if circumstances change" applies.

- **vs framing on Topic 3 sub-question (Reflector trait re-open)**: I
  reject re-opening the Reflector trait even if sqlite-vec spike
  succeeds. ANN is a similarity primitive swap inside one function,
  not a reflection-policy trait.

- **vs analysis.md on Topic 4 trigger**: single-clause "corpus > 1k
  facts" is too permissive. Composite with measurable retrieval
  degradation is the load-bearing test.

## Open Questions

1. Does the operator agree the search-split is "free cleanup" rather
   than scope creep? It touches `mcp_tools.rs` + `cli.rs` callers,
   but doesn't add new code paths — just relocates existing ones.
   If even this feels like scope creep, drop it and accept the
   `impl Db { fn search }` shape stays in v0.0.1.

2. For Topic 4 clause 2 (supersession-within-7-days metric): does
   v0.0.1's domain audit table already have the schema to support
   this query, or does adding the metric require schema changes?
   If it requires schema changes, the trigger must be defined in
   terms of what we'll actually have, not what we wish we had.

3. If Topic 2 is rejected permanently rather than deferred, do
   downstream BLs (e.g., contradiction-handling refactors) need to
   acknowledge the rejection, or is the discussion outcome enough?
