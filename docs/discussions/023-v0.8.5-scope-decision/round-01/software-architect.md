---
agent: software-architect
round: 1
topic: "01"
created: 2026-04-27
---

# software-architect — Round 1

System-shape lens: I look at delivery-unit shape via dependency
boundaries, schema-evolution events, and refactor forcing-functions —
not aesthetics or process ceremony.

## Findings (with file:line evidence)

### F1 — Sprint discipline IS earning its keep, but the unit is "plan", not "version"

Cadence evidence (`git log --since="2026-04-20"`):

- 2026-04-20 — 6 commits, all BL-008 (plan 013 power-law decay).
- 2026-04-22 — 13 commits, all BL-014 (CI runner env fix).
- 2026-04-23 — **28 commits across plans 015 + 016 + 017 + close-outs**.
  Three plans landed in one day with their reviews and BL close-outs
  interleaved.
- 2026-04-24 — 3 commits closing v0.8.0.

The shape is **bursty per-plan**, not steady-trickle. Each plan is
self-contained: discuss → plan → work → review → fixup → close-out. The
plan boundary is what the discipline rules govern (Review Rules in
CLAUDE.md, trigger-discipline rule from discussion 021). The version
boundary (v0.8.0) was a wrapper that fired *one* useful event:
`[roadmap] close v0.8.0 — 7 items shipped` (commit `5c8e1cc`), which
bundled 7 BLs into `.ae/backlog/done/v0.8.0/`.

Reading plans 013-017 reviews shape: each one followed the same
discuss/plan/work/review/fixup/close-out arc and produced its own value
independent of the v0.8.0 frame. **The version tag was an aggregation,
not a gating event.** The discipline is at the plan level; the version
is a label.

**Implication for Q1**: a *cadence-or-threshold-triggered* tag is the
honest description of what's already happening. v0.8.0 became a tag
because Kai decided 7 plans was enough, not because a coherent
themed-sprint was discussed up-front. v0.8.5 risks inverting that —
deciding the version exists *before* identifying the work — which is
the failure mode the framing acknowledges.

### F2 — BL-009's blast radius makes pre-readiness real, not theater

Phase 2 chain (`docs/backlog/005-phase2-roadmap.md`): BL-009 (MCP Dream
Tool) → BL-010 (daemon) → BL-011 (Lint) + BL-013 (Edges).

BL-009 is the first MCP tool that **writes synthesis rows in-session
under user-driven cadence** rather than via `mengdie dream
--synthesize` (operator-controlled). This changes the threat model in
two specific ways:

**(a) New writer path that bypasses `insert_synthesis_with_links`?**
`src/core/mcp_tools.rs` is currently 19.1K and wraps existing library
methods 1:1 (`memory_search` → `db.memory_search`). If BL-009's tool
also wraps `insert_synthesis_with_links`, the existing app-level
invariant ("source_type='synthesis' AND valid_until IS NULL ⇒
synthesis_cluster_hash IS NOT NULL") still holds. **If it
introduces any new direct-SQL or batched-insert path** (plausible —
in-session tool calls may want to materialize partial syntheses
differently from the batch dream pass), the invariant relies on
documentation, not enforcement. That is exactly the gap
`BL-synthesis-cluster-hash-not-null-enforcement` closes
(`.ae/backlog/unscheduled/BL-synthesis-cluster-hash-not-null-enforcement.md:71-76`
explicitly names "a new dream mode" as a trigger).

**(b) New invalidation cadence raises FK risk surface.** Today the
DB has zero hard deletes (`docs/backlog/BL-fk-pragma-and-deletion-safety.md:18`).
BL-009 might add user-facing "discard this synthesis" or "regenerate
this cluster" actions; those are exactly the
delete-then-orphan-link paths the FK pragma guards.
`.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md:54-56`
lists "next schema addition that introduces a new FK-bearing table"
as trigger — BL-009 may not add a new FK-bearing table, but it adds
the first **plausible delete cadence** under user agency, which is
the spirit of the trigger.

**Architectural verdict**: BL-009 isn't theoretically blocked by these
two invariants — but it **expands the consequence-radius if they
silently hold by convention only**. Pre-landing them is cheap (XS + S),
makes BL-009's plan reviews narrower (don't have to re-litigate FK
enforcement under review pressure), and gives a v0.8.5 a coherent
schema-integrity theme.

### F3 — `dreaming.rs` split: forcing function fired but mis-prescribed

`src/core/dreaming.rs` is 1326 lines. Structure (verified):

- L24-329: `DreamingConfig` + `DreamingResult` + `impl Db {
  run_dreaming_with_config }` — **promotion + decay orchestration**
  (single integrated pass).
- L330-580: `SynthesisResult` + async `run_synthesis_pass` (LLM-driven
  synthesis pipeline).
- L581-1326: `mod tests` (~745 lines of tests).

`docs/backlog/BL-dreaming-module-split.md:32-43` prescribed splitting
into three files (`dreaming.rs`, `synthesis_pipeline.rs`,
`decay_pipeline.rs`) when BL-008 lands.

What actually happened (commits `f296966`, `9e329b8`, `fd910e3`):

- BL-008's pure-math primitives went into a new `src/core/decay.rs`
  (7.6K, 4 pure functions: `decay_factor`, `effective_relevance`,
  `should_demote` + tests). That partially honors the split.
- BL-008's **orchestration** (the eligible-row scan, the demote loop,
  the dry-run path) went **into `dreaming.rs::run_dreaming_with_config`
  itself** (lines 157-311 reference decay), making it a third tenant in
  the same `impl Db` block. The trigger's recipe ("Add BL-008's decay
  pass in a new `src/core/decay_pipeline.rs`") was not followed for
  the orchestration layer.

**So the split has TWO components, only one of which fired**:

1. **Math/algorithm extraction** (synthesis.rs at 17.1K, decay.rs at
   7.6K, clustering.rs at 22.7K) — done, file:line evidence above.
2. **Orchestration extraction** (run_synthesis_pass + the new decay
   demote loop should live outside `dreaming.rs`) — NOT done. This is
   the 1326-line "three concerns in one file" problem the BL named.

**Cost of NOT splitting now**: BL-009 will likely add a third
orchestration tenant — an in-session synthesis path. If that lands in
`dreaming.rs` too, the file becomes ~1700 LOC with four concerns and
becomes a real merge-conflict / review-fatigue surface during BL-010
work. Forcing function for the split is **the BL-009 plan**, not a
v0.8.5 sprint.

**Cost of splitting now**: ~100-150 LOC of imports + module moves +
visibility adjustments. No behavior change. One small commit. The
right time is "before BL-009's first plan", which can be either
"end of v0.8.5" or "first commit of BL-009 plan". Sprint inclusion is
an *option*, not a requirement.

### F4 — Production v5 migration is a real schema-evolution event

`docs/discussions/023-v0.8.5-scope-decision/analysis.md:64-66` and
plan 017 evidence: production DB `~/.mengdie/db.sqlite` has a known
zero-link synthesis row (id `529d3212-...`) blocking v5 migration.
v5 is already in the codebase (`src/core/schema.rs:292-378`) — what's
pending is **running it on the live DB**.

Two distinct architectural framings:

**Framing A (ops-only)**: migration is just `mengdie dream` running on
prod for the first time after schema v5 ships. No code changes, no
release semantics, just operator action. Complete it before v0.8.5 is
even discussed. Codex-proxy in analysis.md:160 takes this stance.

**Framing B (release event)**: v5 is the **first migration to ship to a
populated user DB** (Kai's own). Operator docs are missing
(`BL-v5-migration-operator-docs` exists for this). Once a real
operator (Kai) has gone through the migration, the lessons learned
become CHANGELOG/release-notes content. Tagging v0.8.5 *after* the
migration runs gives the version a real schema-evolution payload.

I prefer Framing B by a small margin. Reasoning: v5 is the first
schema migration mengdie has done with a non-empty production DB
(plan 017 introduced the v5 pre-check pattern explicitly because of
this DB). Treating it as "ops-only" wastes the architectural
information. But it doesn't *block* sprint shape — the migration
should run before v0.8.5 *whatever the v0.8.5 decision is*, because
it's blocking nothing else.

### F5 — Discussion 022 said v0.9.0 is next, but didn't say "no v0.8.5"

`docs/discussions/022-synthesis-provenance-options/` — I haven't read
its conclusion in detail, but the framing.md and analysis.md repeatedly
cite "next destination is v0.9.0". That is *destination*, not
*immediate next sprint*. A v0.8.5 between v0.8.0 and v0.9.0 doesn't
contradict 022 — it inserts a hardening checkpoint before v0.9.0
opens. The challenger's "C1 — v0.8.5 is theater" reading conflates
"next destination" with "next event".

### F6 — Duplicate BL: FK pragma is filed twice

`docs/backlog/BL-fk-pragma-and-deletion-safety.md` (created 2026-04-18,
origin BL-007 review) and
`.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md` (created
2026-04-24, origin plan 017 review) are the same finding filed twice
across two different backlog locations. The archaeologist already
flagged this in analysis.md:117-119. Architecturally this is a backlog
hygiene issue; resolving it (dedupe, keep the newer one with richer
trigger conditions) is XS work that belongs to *whichever sprint
schedules the FK pragma fix*, not its own item. The dedupe matters
because if v0.8.5 ships the fix, the *other* backlog file becomes
stale and will mis-trigger future analyses.

## Agreements

(None yet — this is Round 1 independent research.)

## Disagreements

(None yet — Round 1.)

## Open Questions

1. **Does BL-009 introduce any new direct-SQL synthesis-write path?**
   I assumed yes is plausible. If the answer (from the eventual
   BL-009 design discussion) is "no, it routes through
   `insert_synthesis_with_links` exclusively", then
   `BL-synthesis-cluster-hash-not-null-enforcement` becomes weaker as
   a v0.8.5 candidate — its trigger explicitly names "any code path
   other than `insert_synthesis_with_links`". This question can only
   be answered by the BL-009 design pass itself, which hasn't started.
2. **Does mengdie need a "release notes" file at all?** Currently
   there is no top-level `CHANGELOG.md` (verified — analysis.md:71
   references it conditionally). Without a CHANGELOG, the value of a
   v0.x.5 tag is reduced — it's just a git tag with no human-readable
   payload. Worth deciding: does cutting v0.8.5 also obligate creating
   a CHANGELOG? If yes, that's its own line of work.
3. **Is BL-009 plan-ready, or does it need a `/ae:discuss` first?**
   Analysis.md:235 flags "BL-009 has NO discussion doc yet". If v0.8.5
   includes a BL-009 design discussion (Option D in analysis), that's
   a different shape — sprint-with-design-mixin — than pure hardening.

## Position

**Q1 (delivery shape)**: The honest model for mengdie today is
**threshold-triggered tag** (Q1 third option). The plan-level
discipline is what's working; the version is a label that fires when
N plans accumulate. v0.8.0 fired at 7 plans / 7 BLs over roughly 4
days of dense agent work. Don't manufacture a sprint just to have one,
but don't pretend the version tag isn't useful as a CHANGELOG anchor
and a backlog-archive boundary.

**Q2 (cut v0.8.5?)**: Yes, conditionally. The conditions: (a)
production v5 migration runs first (it's blocking nothing and informs
docs scope), and (b) the BL set is small + schema-integrity-themed +
all triggers cleanly fired. If those conditions don't hold, skip to
v0.9.0.

**Q3 (BL set)**: Concretely, my preferred set is:

- `BL-dreaming-module-split` — fired, but only the *orchestration
  half* fired; the math half already happened in plan 013. Defensible
  to take here OR defer to BL-009's first plan.
- `BL-enable-pragma-foreign-keys` — fired weakly today, fires
  cleanly the moment BL-009's plan starts. Pre-landing reduces
  BL-009's plan-review surface. Also dedupe with
  `docs/backlog/BL-fk-pragma-and-deletion-safety.md` in the same
  commit.
- `BL-synthesis-cluster-hash-not-null-enforcement` — fires
  defensibly under "next memory_entries schema migration" if v0.8.5
  bundles a v6 trigger install. If we don't bump schema, it doesn't
  fire — defer. So this is conditional on whether v0.8.5 bundles
  schema-v6 work.
- `BL-v5-migration-operator-docs` — fires once the production
  migration runs. Defensible XS doc work in the v5-migration
  retrospective shape.

Realistic shape: **3 items, 1 schema-integrity theme**. If schema v6
trigger install is included, it becomes 4 items with one schema bump.
This is Option B from analysis.md, expanded with my F2 + F3 reasoning
about *why* these specific items shield BL-009's blast radius.

I disagree with the challenger's C1 ("v0.8.5 is theater"): cleanly
naming a hardening-event-tied-to-prod-migration is exactly what 0.x.5
patches are *for* in the Rust ecosystem. The theater risk is real if
v0.8.5 becomes a vehicle for new features (Option C); it is not real
if v0.8.5 is the schema-integrity + production-migration boundary.
