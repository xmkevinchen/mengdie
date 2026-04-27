---
agent: software-architect
round: 2
topic: "01"
created: 2026-04-27
---

# software-architect — Round 2

In Round 1 I proposed a "conditional v0.8.5" gated on prod migration
running first, with FK pragma + module-split + v5 operator-docs as the
core set; cluster-hash NOT NULL conditional on bundling a v6 schema
bump. Round 2 evidence resolves all three of my open questions and
materially shifts my Round 1 position on cluster-hash NOT NULL —
upward, toward codex.

## Findings

### F1 — Resolved Q1: BL-009's only published design IS a 6-line stub, and it routes through the existing `memory_ingest` MCP tool

`docs/backlog/005-phase2-roadmap.md:66-71` is the entire BL-009 design
artifact. Verbatim:

> **What**: `memory_dream` MCP tool — runs decay + promote + cluster,
> returns clusters to Claude. **Claude synthesizes inline and calls
> `memory_ingest`.**

That last sentence is load-bearing. It says BL-009's synthesis-write
path is **not** a new direct-SQL path that bypasses the schema layer —
it's Claude calling the already-shipped `memory_ingest` MCP tool with
`source_type='synthesis'`.

I traced what `memory_ingest` actually does
(`src/core/mcp_tools.rs:282-405`):

- Calls `db.insert_memory()` (or `insert_memory_resolving` if
  `resolves` is non-empty) at `mcp_tools.rs:380-383`.
- Does NOT route through `db.insert_synthesis_with_links()`.

`db.insert_memory` (`src/core/db.rs:122-162`):

- Omits `synthesis_cluster_hash` from the INSERT column list
  (`db.rs:130-133`). Column defaults to NULL.
- Inserts NO rows into `memory_synthesis_links` — that table is only
  written by `insert_synthesis_with_links` (`db.rs:349-405`).
- ON CONFLICT clause is keyed on `content_hash` with
  `WHERE source_type != 'synthesis'` (`db.rs:135`) — so for a
  synthesis row, it inserts a fresh row every call with no dedup.

**Concrete failure mode under BL-009 stub-as-shipped**:

1. Claude calls `memory_ingest({source_type: "synthesis", ...})`.
2. A row lands in `memory_entries` with `source_type='synthesis'`,
   `synthesis_cluster_hash=NULL`, and zero rows in
   `memory_synthesis_links`. **This is precisely the production orphan
   row `529d3212-...` that already exists today.**
3. The partial unique index `idx_synthesis_cluster WHERE
   source_type='synthesis' AND synthesis_cluster_hash IS NOT NULL`
   silently excludes the row — a second Claude call for the same
   cluster lands a second sibling. Zombie-sibling reborn. Plan 017's
   work invalidated by an unrelated MCP path.

This is no longer speculative. The failure mode **already happened
once** in production, **before BL-009**. BL-009 just multiplies the
rate.

### F2 — Resolved Q2: CHANGELOG.md exists, has v0.8.0 entry + Unreleased section

`/Users/ckai/Workspace/Projects/mengdie/CHANGELOG.md` exists. Lines
1-9: full Keep-a-Changelog header, "Unreleased" section, and a v0.8.0
entry with theme + dated. v0.8.0's CHANGELOG payload includes 7
detailed entries.

This resolves my Round 1 Open Q2 ("does v0.8.5 oblige creating
CHANGELOG.md?") to **no obligation creates new infrastructure** — the
file is already there with an Unreleased section ready to receive
entries. Cutting v0.8.5 is just adding a dated section header above
Unreleased. The "ceremony cost" minimal-change cites
(`minimal-change-engineer.md:172-174`) drops by one task.

### F3 — Resolved Q3: BL-009 needs design before /ae:plan can land cleanly, but the "design" can be inline in the v0.8.5 framing

Repo evidence: there is no BL-009 file, no BL-009 discussion, no plan
008 / 009 / 010 / 011. The 6-line stub at
`005-phase2-roadmap.md:66-71` is the entire design artifact.
Minimal-change is correct that this is thin (`minimal-change-engineer.md:69-73`).

But the stub *does* commit one architectural decision: BL-009 calls
**existing `memory_ingest`**, not a new write path. Whether or not
that decision survives a `/ae:discuss` pass, **the schema-integrity
case for FK pragma + cluster-hash NOT NULL holds even if BL-009 ends
up adding a fancier path** — the existing `memory_ingest` path is
already a corruption surface, BL-009 or no BL-009.

So minimal-change's "fear of unbuilt feature" critique
(`minimal-change-engineer.md:57-90`) is partially refuted by F1: it's
not fear of an unbuilt feature, it's an unaddressed corruption
surface in **already-shipped code** that BL-009 expands the use of.

### F4 — The full backlog candidate set is 9 + 6, not 9. Migration is a real prerequisite.

Per challenger F8 (`challenger.md:85-89`) and the TL synthesis
verification (`synthesis.md:90`), 6 BLs in `docs/backlog/` are not in
`.ae/backlog/unscheduled/` and would be silently skipped by
`/ae:roadmap plan`:

- `BL-clustering-validation.md`
- `BL-dreaming-module-split.md` (the cleanly-fired one)
- `BL-fk-pragma-and-deletion-safety.md` (duplicate-with-divergent-trigger)
- `BL-synthesis-cli-skip-metric.md`
- `BL-synthesis-result-struct-promotion.md`
- `BL-valid-until-boundary.md`

Verified locally (Round 2):
`ls /Users/ckai/Workspace/Projects/mengdie/docs/backlog/`. Result
matches.

This is not just a tooling annoyance — it's a **trigger-discipline
hazard**. The discussion 021 rule (CLAUDE.md Review Rules section,
embedded into `/ae:roadmap plan` workflow) "skim candidate BL bodies
for explicit 'not now' / 'filed for trigger' language" implicitly
defines the candidate set as `.ae/backlog/unscheduled/`. Any
`/ae:roadmap plan v0.8.5` run today will fail to consider 6 BLs — one
of which (BL-dreaming-module-split) is the *single most-cited
fired-trigger item in this discussion*.

### F5 — Reconciling F1 + F4: the production orphan is also caused by the docs/backlog split

The v0.8.5 question is now nested. To answer it correctly:

1. Migrate `docs/backlog/BL-*.md` candidates into
   `.ae/backlog/unscheduled/` (pure bookkeeping, no code change), AND
   dedupe `BL-fk-pragma-and-deletion-safety` ↔
   `BL-enable-pragma-foreign-keys` (challenger F4 +
   `challenger.md:104`).
2. THEN the v0.8.5 candidate set is well-defined and `/ae:roadmap
   plan` can produce a sound plan.
3. THEN the corruption-surface argument from F1 is the strongest case
   for including FK pragma + cluster-hash NOT NULL.

Step 1 is a single small commit. It's not v0.8.5 work; it's
prerequisite hygiene to make the v0.8.5 decision answerable at all.
This converges with challenger F8 and the TL synthesis disagreement
#5 (`synthesis.md:144-148`).

### F6 — Module-split timing: split NOW is correct, but for codex's reason (smaller surface for BL-009 review), not the BL's stated reason

My Round 1 finding (`software-architect.md:120-152`) was that the
split's math half landed in `decay.rs` but the orchestration half
stayed in `dreaming.rs`. Round 2 reading of peers:

- Codex (`codex-proxy.md:108-112`) calls module-split P2 hygiene; OK
  to defer.
- Minimal-change (`minimal-change-engineer.md:151-157`) treats it as a
  ride-along PR on `main`, not sprint material.
- Gemini (`gemini-proxy.md:23-29`) calls it process integrity (fired
  trigger ignored).
- Challenger (`challenger.md:30-37`) credits it as a real fire but
  notes the BL-009 import-path argument is weak (a rename).

I now think the split-now-vs-with-BL-009 question turns on F1, not
on aesthetics. With F1's evidence: BL-009's first plan will need to
change `memory_ingest` (or `db.insert_memory`) to enforce the
cluster-hash invariant, or change the dreaming-module orchestration
to call out to a new in-session synthesis path. Either way, BL-009's
plan will touch `dreaming.rs`. Splitting NOW means BL-009's plan is
reviewed against a 3-file structure (clean diffs); splitting INSIDE
BL-009's plan means a 1326+1700-line file gets refactored mid-feature
review, with merge conflicts against the feature work itself.

**Verdict**: split now, in v0.8.5. Codex's "P2 hygiene, can defer"
under-rates the cost; minimal-change's "ride-along on main" route is
fine in isolation but loses the bundling benefit with the FK + cluster-hash
hardening (one v0.8.5 sprint vs three loose commits + one sprint).

### F7 — Production migration as architectural event, not ops task

Minimal-change (`minimal-change-engineer.md:135-142`) says the
production v5 migration is an off-tree ops task, not v0.8.5 work. I
disagreed in Round 1 (`software-architect.md:F4`); Round 2 sharpens
the disagreement.

The orphan synthesis row `529d3212-...` is **the empirical
manifestation of the F1 corruption surface**. The migration's
pre-check at `src/core/schema.rs:292-378` will detect it and refuse
to migrate. Resolving it requires choosing between:

- (a) Hard-delete the orphan (`DELETE FROM memory_entries WHERE id =
  '529d3212-...'`). FK pragma OFF allows this, but FK pragma ON +
  the new triggers from cluster-hash NOT NULL would gate this
  differently. Migration order matters.
- (b) Backfill links retroactively. Requires inferring the source
  cluster from the synthesis content. Unsound for old syntheses.
- (c) Invalidate (set `valid_until=now`). Preserves history but
  leaves the row in the DB.

Each option has a different relationship to the proposed v0.8.5 BLs.
If FK pragma + cluster-hash NOT NULL ship in v0.8.5, the migration
playbook (and the `BL-v5-migration-operator-docs` content) is a
**different document** than if they don't ship. So the migration is
NOT an off-tree ops task — it is a **schema-evolution event whose
runbook depends on v0.8.5 scope decisions**.

This converges with codex's "production orphan first informs scope"
(`codex-proxy.md:111` + `analysis.md:160`) and disagrees with
minimal-change's "off-tree, run tomorrow" framing.

## Agreements

### A1 — With codex (`codex-proxy.md:140-148`) on the integrity case for FK pragma + cluster-hash NOT NULL

My Round 1 was conditional ("only if v0.8.5 bundles a v6 schema
bump"). F1 changes this. The cluster-hash NOT NULL trigger fires the
moment Claude calls `memory_ingest({source_type:'synthesis', ...})`
through BL-009 — and the FAILURE MODE has already happened once
without BL-009. Codex's "imminent fire" framing
(`codex-proxy.md:121`) is correct.

I now agree with codex on **landing both items in v0.8.5**, with one
caveat: the dedup of BL-fk-pragma-and-deletion-safety vs
BL-enable-pragma-foreign-keys needs to happen first (F4).

### A2 — With challenger (`challenger.md:85-89`) and gemini (`gemini-proxy.md:23-29`) on dreaming-module-split being a fired trigger

This is unanimous in Round 1 (4/5; minimal-change agrees on the fire
but routes it differently). Confirmed in Round 2 reading. The
disagreement is about delivery shape, not whether the trigger fired.

### A3 — With minimal-change (`minimal-change-engineer.md:206-215`) that BL-009 needs `/ae:discuss` before `/ae:plan`

I read this in Round 1 as an open question; Round 2 evidence (F1 + F3)
makes me agree. The 6-line stub is enough to *justify* schema-integrity
hardening in v0.8.5 (the corruption surface is real and pre-existing).
But it is NOT enough to start `/ae:plan` on BL-009 itself —
`/ae:discuss` is needed for BL-009 to define its actual write paths,
its accept/reject UX, and its interaction with the existing
`memory_ingest`.

This becomes part of v0.8.5's outcome: **v0.8.5 closes with
schema-integrity hardening + module-split + a queued `/ae:discuss
BL-009` as the first event in v0.9.0**. Not a v0.8.5 deliverable
itself, but a v0.8.5 close-out item.

### A4 — With challenger (`challenger.md:38-49`) that BL-enable-pragma-foreign-keys's literal trigger has not fired

Per the BL's stated triggers
(`.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md:54-62`):
none of the three literal conditions has fired. The orphan row was
zero-link, not orphan-FK-link.

I still recommend including the FK pragma in v0.8.5 — but the
**justification has to be different than "the trigger fired"**.
Honest justifications:

1. F1's corruption surface is real for `memory_ingest(source_type:
   synthesis)` — orphan FK risk is the same class even if the
   production row has zero links rather than dangling links.
2. The duplicate `BL-fk-pragma-and-deletion-safety.md`'s trigger
   (DELETE path or audit/provenance feature) is closer to fire under
   BL-009 if BL-009 introduces "discard this synthesis" UX.
3. Bundling with cluster-hash NOT NULL is a single coherent
   schema-integrity theme that makes v0.8.5 an idiomatic 0.x.5
   patch.

If the v0.8.5 framing is honest about #1 (the production orphan IS
schema-integrity evidence even if the BL's literal trigger language
doesn't quite match), the trigger-discipline rule is honored in
spirit — the rule was written to forbid scheduling on sentiment, not
to forbid scheduling on **observed corruption**.

## Disagreements

### D1 — With minimal-change (`minimal-change-engineer.md:11-15`): "skip v0.8.5 entirely" understates F1

Minimal-change's recommended path is: 1 ride-along refactor PR + 1
CLAUDE.md fix + open `/ae:discuss BL-009`. Total cost ≈ 1 day.

This is the right shape **if F1 didn't exist**. But F1 says: the
already-shipped `memory_ingest` MCP tool **can already** create the
corruption pattern. An MCP client that mistakenly passes
`source_type:'synthesis'` (or, BL-009 design notwithstanding, a
future client that does) creates the orphan. The bug exists today;
it just hasn't been triggered at scale because no LLM client has
been calling `memory_ingest` with synthesis source_type yet.

Minimal-change's "fear of unbuilt feature"
(`minimal-change-engineer.md:57-90`) is precise as a critique of
*speculative* hardening, but the corruption surface I documented in
F1 is not speculative — it's a property of *already-shipped* code.

### D2 — With gemini (`gemini-proxy.md:107-115`) on residuals-clarity CLI as v0.8.5 scope

Gemini's recommendation includes a residuals-clarity CLI subcommand
(`mengdie audit explain` or `mengdie audit summary`) as v0.8.5 scope.

This violates two constraints simultaneously:

1. **0.x.5 patch convention** (analysis.md:88-90): new user-visible
   CLI subcommands are not patch-shape work.
2. **Trigger-discipline**: there is no BL for "residuals clarity
   CLI". Gemini acknowledges this implicitly by listing three
   alternative implementation shapes
   (`gemini-proxy.md:84-88`). Choosing among them is a design
   discussion, not a sprint plan input.

I agree with gemini's *diagnosis* (transparency need before BL-009 —
`gemini-proxy.md:14-22`) but disagree with v0.8.5 as the venue. The
right venue is `/ae:discuss residuals-transparency` parallel to or
following BL-009 design discussion. Same conclusion as
minimal-change (`minimal-change-engineer.md:191-195`), reached via a
different path.

### D3 — With minimal-change (`minimal-change-engineer.md:135-142`) on production v5 migration being off-tree

Per F7. Migration runbook depends on v0.8.5 scope; cannot run before
scope is locked.

### D4 — With codex (`codex-proxy.md:151-153`) deferring BL-v5-migration-operator-docs

Codex calls this "doc polish, not critical path; can ride v0.9.0's
wave". I disagree under F7: if v0.8.5 ships FK pragma + cluster-hash
NOT NULL, the migration order changes ("set FK pragma BEFORE running
v5 migration" vs "run v5 migration first") and the operator doc
needs to capture this. The doc IS critical path because it documents
the new scope's interaction with the production migration.

Including `BL-v5-migration-operator-docs` in v0.8.5 is the right
move; deferring to v0.9.0 means writing the doc against v0.8.5
behavior in v0.9.0, which is awkward.

## Open Questions

1. **(Cross-cut #4 — minimal-change's "fear of unbuilt feature")**
   Round 2 evidence (F1) refutes this for **already-shipped**
   `memory_ingest`. But there's a subtler version of minimal-change's
   argument I haven't refuted: should the FK pragma + cluster-hash
   NOT NULL be filed against `memory_ingest` directly (a `BL-ingest-
   synthesis-source-type-restrict` type item), rather than against
   schema-layer enforcement? The schema-layer fix is broader and
   more conservative; the
   `memory_ingest`-layer fix is narrower and might be the
   minimum-machinery solution. **My read**: schema-layer is
   correct because BL-009 may add additional writers in v0.9.0,
   and trigger-based enforcement at the schema layer catches all of
   them. But this deserves explicit decision in the v0.8.5 plan
   review.

2. **What is the upgrade path for the production orphan
   `529d3212-...`?** The three options in F7 (delete / backfill /
   invalidate) need a single canonical answer. This is a v0.8.5
   plan-time question, not a discussion-time question, but it
   should be flagged in the v0.8.5 framing so the plan author
   doesn't punt on it.

3. **Should the dedup of FK BLs be its own commit or part of v0.8.5
   sprint commit?** Per cross-cut #3 (backlog migration). My read:
   single commit "v0.8.5 sprint kickoff: backlog hygiene + FK
   dedup" preceding the actual sprint plan. Pure bookkeeping, zero
   code change, makes `/ae:roadmap plan v0.8.5` answerable.

## Position (Round 2 — revised from Round 1)

**Q1 (delivery shape)**: unchanged. Threshold-triggered tag is the
honest model for mengdie. v0.8.0 fired the threshold at 7 plans /
~4-day burst. v0.8.5 firing now is consistent with that model
**because** F1 documents a real corruption surface that wants to
land before BL-009 expands its blast radius.

**Q2 (cut v0.8.5?)**: Yes, more confidently than Round 1. The
conditions are now:
- (a) Backlog migration commit lands first (challenger F8 +
  cross-cut #3). Pure bookkeeping.
- (b) FK BL dedupe lands in same commit.
- The "production migration must run first" condition I gave in
  Round 1 is replaced by "production migration runs **as part of
  v0.8.5 close-out**, with operator doc shaped to match
  v0.8.5 invariants" (F7).

**Q3 (BL set)**: Revised set:

- `BL-dreaming-module-split` — fired, also reduces BL-009 plan-review
  surface (F6).
- `BL-enable-pragma-foreign-keys` — literal trigger has not fired
  (A4), but F1 + plan 017 production orphan are sufficient
  schema-integrity evidence to schedule it. Justification must be
  honest about this distinction.
- `BL-synthesis-cluster-hash-not-null-enforcement` — F1's
  trigger-#a ("any code path other than `insert_synthesis_with_links`
  writes synthesis rows") **fires today**, not in some unbuilt
  future, because `memory_ingest` IS such a code path and is
  already shipped.
- `BL-v5-migration-operator-docs` — bundled to capture the
  migration-runbook delta from F7.

Total: 4 items. Schema-integrity + module-split themed. Plus 1
prerequisite bookkeeping commit (backlog migration + FK BL dedup)
that is NOT part of v0.8.5 itself but lands first.

This is Option B from analysis.md, with **F1 as new evidence
upgrading codex's two items from "weak fire" to "fire-via-shipped-
ingest-path"**, and challenger F8's backlog migration as
prerequisite.
