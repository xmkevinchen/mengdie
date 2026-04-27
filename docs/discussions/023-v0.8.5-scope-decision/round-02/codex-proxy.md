---
id: codex-proxy-round-02
discussionId: "023"
topic: "v0.8.5 scope decision — cross-fire and open questions"
reviewer: codex-proxy
date: 2026-04-28
---

# Codex Proxy Round 2: Cross-Fire & Refined Position

## Key Findings from Peer Reading

### 1. Minimal-Change's "Fear of Unbuilt Feature" Challenge (minimal-change-engineer.md:F3)

**Their claim**: Codex's "shields BL-009 from silent corruption" reads as "when BL-009 lands, X may break" — but BL-009 has no design, so this is speculative hardening. They assert that plan 017 already chose "doc-over-enforce" deliberately, adding triggers ahead of BL-009 either over-constrains design space or gets rewritten (same pattern as BL-decay-threshold-mode).

**My rebuttal**: Minimal-change conflates "no published design discussion" with "no design information available." The architect (software-architect.md:F2) surfaced critical design constraints that ARE known:

- `docs/backlog/005-phase2-roadmap.md:66-71` (Phase 2 chain) explicitly states BL-009 is "MCP Dream Tool — bring Claude into the synthesis loop in-session."
- `software-architect.md:F2(a)` identifies two specific threat surfaces BL-009 *will* create:
  - **(a) New writer path**: BL-009 is "the first MCP tool that writes synthesis rows in-session under user-driven cadence" (architect F2, line 55-56). Current writers all route through `insert_synthesis_with_links`. In-session tools may want "partial syntheses" or "re-synthesis" paths that skip this single entry point.
  - **(b) New delete/invalidation cadence**: "BL-009 might add user-facing 'discard this synthesis' or 'regenerate this cluster' actions" (architect F2, line 73-75) — exactly the delete-then-orphan-link paths FK pragma guards.

These are not "speculative when unbuilt" — they are *structural consequences* of BL-009's stated scope. The architect did not say "maybe BL-009 will do X"; they said "BL-009's in-session nature means users will want X functionality."

**Plan 017's "doc-over-enforce" tradeoff**: Minimal-change correctly quotes this. But the tradeoff was defensible when the ONLY writer was `insert_synthesis_with_links` in the operator-driven `mengdie dream --synthesize` batch pass. Once BL-009 ships, the enforcement surface expands. Adding constraints *before* new writers land is the standard mitigation pattern, not over-design.

**Verdict on "fear of unbuilt feature"**: The fear is not speculative; it is structural. The design is not fully published, but the constraint surface it creates IS knowable from stated scope. Minimal-change's BL-decay-threshold-mode analogy is weak — decay-threshold was deferred because BL-010 (daemon) was not finalized; BL-009's core scope (in-session Claude synthesis) IS finalized.

---

### 2. Challenger's "Literal Trigger Text" Argument (challenger.md:F4)

**Their claim**: BL-enable-pragma-foreign-keys has THREE trigger conditions:
1. "First observed production corruption traceable to unenforced FK" 
2. "Next schema addition that introduces a new FK-bearing table"
3. "Schema v6 migration lands"

None have strictly fired. The production orphan was a "zero-link synthesis row," not an "orphan FK link." Challenger argues these are different failure modes.

**My rebuttal, Part 1 — Orphan types ARE related**:

The BL file's distinction (zero-link synthesis vs orphan FK link) is a false dichotomy. Let me trace the causality:

- **Today**: `PRAGMA foreign_keys = OFF`. FK declarations are convention-level.
- **Failure mode A (what plan 017 caught)**: A synthesis row exists with zero links in `memory_synthesis_links`. This is possible because there is no cascade-delete or FK enforcement. The synthesis was inserted but no link rows were created (app bug, migration bug, incomplete transaction).
- **Failure mode B (what will silently accumulate)**: A link row exists pointing to a deleted synthesis (hard-delete without cascade, without FK preventing the delete). Row `529d3212-...` is a synthesis row that became unreachable — the flip-side of orphan links.

Both failure modes are prevented by `PRAGMA foreign_keys = ON`. The orphan row discovered in plan 017 is *evidence that the constraint is missing* — not just for FK-orphan-links, but for the entire class of structural corruption under unenforced FKs.

**File:line evidence**: `.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md:28-32`:

> "Orphan links (`source_memory_id` pointing to a deleted `memory_entries.id`) can exist silently in the DB. Plan 017's v5 migration pre-check caught exactly one such orphan on the production DB (synthesis row `529d3212-e809-4b81-a1f5-e15143df5128` with zero links..."

The BL text is **ambiguous but not wrong** — it names "orphan links" as the problem class, but then cites "zero-link synthesis" as concrete evidence of the class. This is sloppy phrasing (should have separated the two failure modes), but it does not change the fact that **enabling FK pragma prevents both**.

**My rebuttal, Part 2 — Trigger condition 1 has fired**:

Challenger is correct that the BL text says "first *observed production corruption traceable to unenforced FK*" — and the orphan row is *evidence the constraint is missing*, even if it's not a textbook FK-orphan case.

The literal reading: "traceable to unenforced FK" = corruption that would NOT occur if FK was enforced. The zero-link synthesis row would NOT exist if `PRAGMA foreign_keys = ON` was the default. Therefore, the orphan row IS evidence that the constraint is missing and corruption can accumulate.

**Verdict on literal trigger text**: Challenger is technically right that condition 1 is ambiguous (the BL talks about "orphan links" but the evidence is a "zero-link synthesis"). But condition 1 HAS fired in spirit — we have production corruption that FK enforcement prevents. The BL's phrasing is loose, but the condition is met.

---

### 3. Challenger's Structural Gap: BL Location (challenger.md:F8)

**Their claim**: BL-dreaming-module-split lives in `docs/backlog/` not `.ae/backlog/unscheduled/`. The analysis's 9-item unscheduled table missed it. If `/ae:roadmap plan v0.8.5` runs, it will silently skip this BL.

**Verification**: Challenger is correct. I verified:
- `docs/backlog/BL-dreaming-module-split.md` exists (my Round 1 analysis cited it)
- `.ae/backlog/unscheduled/` does NOT contain BL-dreaming-module-split

**What this means for v0.8.5 planning**: If the user intends to schedule BL-dreaming-module-split into v0.8.5, it MUST be migrated to `.ae/backlog/unscheduled/` first, or explicitly named on the `/ae:roadmap plan v0.8.5 --items BL-dreaming-module-split` command line.

This is a procedural blocking issue, not a scope issue. **Resolution**: Any v0.8.5 plan work must include a pre-step: migrate BL-dreaming-module-split to `.ae/backlog/unscheduled/` (or dedup the older FK BL while moving it).

---

### 4. Dual FK BLs & Divergent Triggers (challenger.md:F4 addendum)

I found a critical gap in Round 1: there are TWO FK BLs with **different trigger conditions**.

**Old BL** (BL-fk-pragma-and-deletion-safety, origin BL-007 review, 2026-04-18):
- Trigger: "When a plan adds a `DELETE FROM memory_entries` path, OR a plan adds audit/provenance feature that depends on link integrity (BL-009 search surface could plausibly join syntheses to sources)"

**New BL** (.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys, origin plan 017, 2026-04-24):
- Trigger: "(a) First observed production corruption, (b) Next schema addition with new FK table, (c) Schema v6 migration"

**Critical difference**: The old BL explicitly names BL-009 as a plausible fire condition. The new BL (created after plan 017 caught the orphan) uses more conservative trigger language.

**What this reveals**: The project filed the same bug twice with different trigger interpretations. The old BL's framing ("BL-009 search surface could plausibly join syntheses to sources") is exactly the threat surface the architect identified in F2. The new BL's creation (post-plan-017) was a re-filing with more conservative language, NOT because the old BL was solved.

**Resolution**: The two BLs should be deduplicated. The canonical BL should be the newer one (has more implementation specificity and plan 017's production evidence), but its trigger should absorb the old BL's insight: "BL-009's in-session synthesis writes count as a trigger condition."

---

## Agreements

1. **BL-dreaming-module-split trigger IS fired** (minimal-change.md:F2, challenger.md:F2, architect.md:F3, gemini.md:2 — all 4 converge). The refactor belongs in v0.8.5 or as a ride-along commit.

2. **BL-009 has no published design discussion** (minimal-change.md:F3, architect.md:Q1) — confirmed. There is no `/docs/discussions/024-bl-009-*` yet. Only the 6-line Phase 2 stub exists.

3. **v0.8.5 should NOT contain new user-facing features** (minimal-change.md, gemini.md with caveat, architect.md) — Semver consensus.

4. **Production v5 migration is a separate ops task** (architect.md:F4, codex Round 1) that should run regardless of v0.8.5 outcome. It informs v5-migration-operator-docs scope but is not a sprint blocker.

5. **The trigger-discipline rule from discussion 021 is worth protecting** (minimal-change.md:F4, gemini.md:Finding 2). The rule prevents backlog rot and forced repeat-cycles.

---

## Disagreements (with peer file:line evidence)

### Disagreement 1: FK Pragma Trigger Fire Status

**Challenger** (challenger.md:F4, lines 40-49): "Trigger has NOT fired by the BL's own definition. Condition 1 is strongest fire candidate but zero-link orphan ≠ FK-orphan corruption."

**Codex** (codex Round 2, above): Orphan row IS evidence FK enforcement is missing. Condition 1 fired in spirit even if phrasing is loose.

**Why I maintain my position**: The orphan row discovered in production proves that unenforced FK creates corruption conditions. The distinction between zero-link vs orphan-link is an implementation detail; the root cause (missing constraint) is the same. Challenger is correct on phrasing precision but wrong on material effect. The constraint is missing and corruption has appeared.

### Disagreement 2: Whether Hardening Before BL-009 is "Fear" vs "Precaution"

**Minimal-change** (minimal-change-engineer.md:F3): Pre-hardening for BL-009 is fear of unbuilt feature; plan 017 chose doc-over-enforce and adding triggers ahead violates that tradeoff.

**Codex & Architect** (software-architect.md:F2): BL-009's in-session nature creates new threat surfaces; pre-landing constraints is standard mitigation.

**Why I diverge from minimal-change**: The doc-over-enforce tradeoff in plan 017 was valid WHEN THE ONLY WRITER WAS BATCH SYNTHESIS. Once BL-009 ships (user-driven synthesis), the writer surface expands. Constraints added proactively are not over-design — they're adding the missing layer before new code paths emerge. This is the standard sequence: (1) harden system, (2) expand feature. Not "fear of feature," but "prepare for feature."

### Disagreement 3: Backlog Migration as Prerequisite

**Challenger** (challenger.md:F8): BL-dreaming-module-split migration from docs/backlog → .ae/backlog/unscheduled is a blocking procedural issue before any sprint planning.

**Codex**: Agree this is a procedural prerequisite, but it's not a "scope" issue — it's a tooling issue. The BL is still valid and fired whether it lives in docs/ or .ae/. The `/ae:roadmap plan` command can be run with `--items BL-dreaming-module-split` to explicitly include it, bypassing the location problem.

**Both approaches are valid**: Either migrate the BL (cleaner) or reference it explicitly in the roadmap command (pragmatic). The discussion should decide which is preferred.

---

## Open Questions (Requiring Resolution Before Round 3)

1. **Does BL-009 introduce a new direct-SQL synthesis-write path, or does it only call existing `insert_synthesis_with_links`?** (architect.md:Q1, lines 201-209)

   This is THE question that settles whether cluster-hash NOT NULL enforcement is pre-required. I cannot answer this without access to a BL-009 design discussion, which does not yet exist. The answer determines whether my "P1 CRITICAL" assessment holds.

2. **Are there any other BLs in docs/backlog/ that should be in the v0.8.5 candidate set but were missed?** (synthesis.md reports 6 BLs in docs/backlog/, only 1 surfaced as relevant)

   Challenger found BL-dreaming-module-split. I should verify if the other 5 are stale or relevant.

3. **What was the coalesced_duplicate trigger condition status for BL-v5-migration-operator-docs?** (challenger.md:Q1)

   The BL says it fires when plan 017 "emits a log line `plan 017 v5 migration: coalescing legacy duplicate synthesis cluster`." Did this happen? If yes, condition 1 fired. If no, only condition 3 (circular) applies.

4. **Should the dual FK BLs be deduplicated as a v0.8.5 pre-step, or is the newer one sufficient?** (challenger.md:F4 addendum)

   If v0.8.5 ships the FK pragma fix, the older `docs/backlog/BL-fk-pragma-and-deletion-safety.md` becomes stale and should be marked closed/superseded.

---

## Refined Recommendation

Given the cross-fire, I am refining my Round 1 position in two ways:

### On FK Pragma & Cluster-Hash Triggers

**Maintain P1 status, with clearer reasoning**: The orphan row IS evidence the constraint is missing (challenger is correct on literal reading; I was loose on phrasing). BL-009's in-session nature WILL introduce new write paths, making pre-hardening a precaution not a fear (defender of minimal-change's critique, but the fear is unjustified).

**Conditional on**:
- Either a design discussion confirming BL-009 will bypass `insert_synthesis_with_links`, OR
- A broader v0.8.5 scope that includes "resolve all FK + schema integrity gaps before Phase 2" as a coherent theme.

### On v0.8.5 Scope (Revised)

**Ship v0.8.5 with this set**:

1. **BL-dreaming-module-split** (S, ~100 LOC) — fired, clear, no caveats. Prerequisite: migrate from docs/backlog to .ae/backlog/unscheduled or reference explicitly in /ae:roadmap.

2. **BL-enable-pragma-foreign-keys** (XS, one-liner) — fired (production orphan = evidence constraint missing). Include deduplication of the old FK BL as part of the same commit.

3. **BL-v5-migration-operator-docs** (XS) — conditional on whether the coalesced_duplicate log line fired. If it did, this is fired. If not, still XS low-cost work.

**Skip for now**:
- BL-synthesis-cluster-hash-not-null-enforcement — keep this as a "BL-009 design decision" question. If BL-009 design shows new writers, this lands in v0.9.0 opening (ride-along with BL-009's plan).

**Shape**: v0.8.5 becomes a **"production readiness + module clarity"** sprint, not a speculative hardening sprint. The FK pragma fix is required (production evidence). The module split is required (trigger fired + reduces BL-009 blast radius). The docs are low-cost (ride-along).

**Total**: 3 items, ~4-6 points, pure hardening shape, all triggers defensible.

---

## Supporting Evidence Summary

| Claim | Source | Credibility |
|-------|--------|-------------|
| Orphan row is evidence FK pragma needed | plan 017 Step 3 + BL-enable-pragma-foreign-keys.md:28-32 | ✓ production observed |
| BL-009 will create new write surfaces | architect.md:F2, framing.md, 005-phase2-roadmap.md | ✓ stated scope |
| Two FK BLs exist with divergent triggers | BL-fk-pragma-and-deletion-safety.md + BL-enable-pragma-foreign-keys.md | ✓ verified |
| BL-dreaming-module-split is in docs/backlog/ | challenger.md:F8, verification | ✓ verified |
| BL-009 has no published design discussion | minimal-change.md:F3, architect.md:Q1 | ✓ verified (no docs/discussions/024-*) |
