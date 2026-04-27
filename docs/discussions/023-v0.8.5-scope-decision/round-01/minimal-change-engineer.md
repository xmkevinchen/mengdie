---
agent: minimal-change-engineer
round: 1
discussion: "023"
created: 2026-04-27
---

# Round 1 — minimal-change-engineer

## Position (one line)

**Skip v0.8.5. Take BL-dreaming-module-split as a single ride-along
commit on `main` (no version tag). Open `/ae:discuss` for BL-009 next.
Everything else is not trigger-fired and stays in unscheduled.**

## Findings (with file:line evidence)

### F1 — Discussion 022 already decided "next is v0.9.0"

`docs/discussions/022-synthesis-provenance-options/conclusion.md:74`:

> "After v0.8.0 synthesis cluster ships: v0.8.0 complete. Move to v0.9.0
> per the roadmap theme (BL-009 MCP Dream Tool)."

That conclusion is concluded, not pending. Carving v0.8.5 out of the
gap between v0.8.0 (closed 2026-04-24) and v0.9.0 is **net new scope**
the user is now proposing — and the framing.md attempt-3 override
(`framing.md:6-8`) shows two reviewers (codex + minimal-change-engineer)
already pushed back hardest, the same lens this round should hold. The
default answer here is "do nothing more than 022 already specified."

### F2 — Only ONE BL has a cleanly-fired trigger, and it's a one-commit refactor

I read all 9 unscheduled BL frontmatters and bodies (plus the duplicate
`docs/backlog/BL-fk-pragma-and-deletion-safety.md`). Trigger-status
verdict — **strict reading of the BL body, not analysis 023's
interpretation**:

| BL | My verdict | Evidence |
|----|-----------|----------|
| **BL-dreaming-module-split** | **FIRED** | `docs/backlog/BL-dreaming-module-split.md:33-37` says trigger fires "when BL-008 plan lands. The first commit of BL-008 should preferentially…". BL-008 shipped as plan 013 on 2026-04-20. Trigger has been fired 7 days. `src/core/dreaming.rs` is 1326 LOC, confirmed. Body estimates "< 100 lines of imports + module moves, no new tests needed." This is a single-commit ride-along. |
| BL-enable-pragma-foreign-keys | NOT FIRED (strictly) | Triggers (`.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md:54-62`): (a) "first observed production corruption traceable to unenforced FK" — none observed; (b) "next schema addition that introduces a new FK-bearing table" — none scheduled; (c) "Schema v6 migration lands" — not landed. Plan 017 caught a zero-link orphan, NOT an FK-orphan corruption. Codex's "shields BL-009 from silent corruption" argument is **fear of an unbuilt feature**, not a fired trigger. |
| BL-v5-migration-operator-docs | WEAKLY FIRED (XS only) | Trigger (`BL-v5-migration-operator-docs.md:73-77`): "first production migration to v5 on a DB with `coalesced_duplicate` rows" — production migration **hasn't run yet** (orphan synthesis row blocks it). The third trigger ("next doc polish sprint") IS the self-referential one analysis 023 flagged. So strictly: not fired until the production migration runs. |
| BL-synthesis-cluster-hash-not-null-enforcement | NOT FIRED | Triggers (`BL-synthesis-cluster-hash-not-null-enforcement.md:71-80`): all 4 (other-code-path writer, observed live-NULL row, next migration, audit-all lands) are unobserved or unbuilt. Codex's "before BL-009 adds new writers" rephrases this as a forward trigger we do not yet have — same fear-of-unbuilt-feature pattern as the FK BL. |
| BL-audit-collection-discipline | NOT FIRED | Triggers (`BL-audit-collection-discipline.md:62-68`): 50% audited (currently ~37%), Option 2/3 plan in flight, or corpus > 100 syntheses (currently 27). Body itself acknowledges "latent-never-fires problem" — analysis 023 saying this is a `Trigger status: NOT FIRED`. Confirmed. |
| BL-decay-dreaming-pass-optim | NOT FIRED | Trigger gated on 50k corpus / 1s p95 / BL-010 (`BL-decay-dreaming-pass-optim.md:56-65`). All unfired. |
| BL-decay-threshold-mode | NOT FIRED | Frontmatter trigger explicit: "BL-010 daemon plan approved / work starts" (`BL-decay-threshold-mode.md:8`). Plan 015 + discussion 021 Topic 3 already established this is intentionally first-caller-deferred. |
| BL-get-synthesis-with-sources-n-plus-1 | NOT FIRED | `max_cluster_size` unchanged, profiler unrun (`BL-get-synthesis-with-sources-n-plus-1.md:62-72`). |
| BL-release-yml-ci-gate | NOT FIRED | Triggers all "first time something ships broken" (`BL-release-yml-ci-gate.md:34-42`). No release tag exists yet. |
| BL-synthesis-preload-db-miss-edge | NOT FIRED | Already removed via `/ae:roadmap remove` per discussion 021 Topic 2 (`docs/discussions/021-v0.8.0-bl-dependencies/conclusion.md:18`). It's in unscheduled because that's where /ae:roadmap remove parks descoped items. **It does not belong in any v0.8.5 candidate set** — the project already decided this. |

**Score: 1 firmly fired, 1 weakly fired (XS, blocked on a separate ops
task), 7 not fired.**

That is not a sprint. That is a ride-along.

### F3 — The "BL-009 readiness" argument is fear of an unbuilt feature

Codex's recommendation (analysis.md:150-160) reads as:

- BL-enable-pragma-foreign-keys "shields BL-009's new synthesis paths
  from silent corruption."
- BL-synthesis-cluster-hash-not-null-enforcement "closes
  doc-over-enforcement gap before BL-009 adds new writers."

Both claims share a shape: "**when BL-009 lands, X may break, so harden
X now**." The problem with this shape:

1. **BL-009 has no design.** No discussion file
   (`docs/discussions/` ends at 023; no BL-009 entry exists). No plan.
   `docs/backlog/005-phase2-roadmap.md:66-71` is a 6-line stub. We do
   not yet know if BL-009 introduces a new synthesis writer, modifies
   `insert_synthesis_with_links`, or just calls existing code paths.
2. **The hardening shape is wrong without the design.** Plan 017
   already chose "doc-over-enforce" deliberately
   (`BL-synthesis-cluster-hash-not-null-enforcement.md:42-48`). Adding
   triggers ahead of BL-009 either over-constrains the design space
   ("any writer must route through `insert_synthesis_with_links`") or
   gets rewritten when BL-009's actual shape forces a different
   invariant.
3. **The risk model is "production may have orphans."** The actual
   evidence: plan 017 found ONE zero-link synthesis row in 27. Zero
   FK-orphan rows. The corruption codex protects against is
   theoretical, not observed.

This is the BL-decay-threshold-mode pattern (discussion 021 Topic 3):
build for an unknown caller and you ship a stub that gets rewritten.
**The minimum-machinery move is to do the BL-009 design discussion
FIRST** and let *that* discussion decide whether FK pragma + cluster-hash
NOT NULL are real preconditions or fear.

### F4 — The trigger-discipline rule from discussion 021 binds this decision

`docs/discussions/021-v0.8.0-bl-dependencies/conclusion.md:18` Topic 2:

> "/ae:roadmap remove both `BL-decay-dreaming-pass-optim` and
> `BL-synthesis-preload-db-miss-edge`. ... `--reason` cites
> trigger-not-fired"

CLAUDE.md `Review Rules` section codified this:

> "before running `/ae:roadmap plan v<ver>`, skim candidate BL bodies
> for explicit 'not now' / 'filed for trigger' language. /ae:roadmap
> remove such items before sprint-commit."

**The rule explicitly forbids what analysis 023's Option B and codex's
recommendation are about to do**: schedule `BL-enable-pragma-foreign-keys`
("filed for later" — `BL-enable-pragma-foreign-keys.md:64-69`) and
`BL-synthesis-cluster-hash-not-null-enforcement` ("filed here rather
than bundled" — same body line 42-48) into a sprint when their explicit
trigger conditions have not fired.

If we follow Option B, we will repeat exactly the pattern v0.8.0 had to
retroactively undo. The trigger-discipline rule was written for this
moment.

### F5 — Continuous-no-tag is the right delivery shape for one ride-along commit

framing.md:23-30 lists three delivery shapes. The choice is governed by
what the inventory actually contains:

- **9 unscheduled BLs, 1 fired** is not "a sprint." It's "a single
  small refactor that should land on main."
- **A version tag (v0.8.5)** is shipping ceremony — sprint plan, work,
  review, close-out, CHANGELOG, gate-text. ~~10x~~ overhead vs the
  refactor itself (BL says "< 100 LOC, no new tests needed").
- **Cadence/threshold-triggered** is a forward-looking decision about
  delivery process that should not be made on n=1 evidence. Defer it.

The minimum-machinery answer is: **continuous flow on main, no tag,
land BL-dreaming-module-split as a single PR. v0.9.0 is the next
named anchor; nothing changes that.**

### F6 — Two independent ops tasks should not be confused with a sprint

Out of scope per framing.md:62-64 but worth naming:

1. **Production v5 migration** — orphan synthesis row
   `529d3212-...` blocks `~/.mengdie/db.sqlite` v5 migration. This is
   one operator action: invalidate the row OR restore its links, then
   re-run migration. It is NOT v0.8.5 work.
2. **CLAUDE.md "Next step (current)" stale** — analysis 023 confirmed
   plan 011 closed, "67% residuals" line is stale. One-line edit. NOT
   v0.8.5 work.

These get conflated with "v0.8.5 has real work" only if we let scope
creep. They are owed regardless of v0.8.5 outcome.

## My recommended path

```
[NOW]
  ├─ git commit: bump BL-dreaming-module-split (~80 LOC refactor) on main, no tag
  ├─ git commit: CLAUDE.md "Next step" line updated (one-line ops)
  ├─ user runs: production v5 migration (separate ops task, off-tree)
  └─ /ae:discuss on BL-009 design + scope (THIS is the next pipeline step)

[v0.9.0]
  ├─ BL-009 plan + work + review (with whatever readiness BLs the
  │   design discussion proves are ACTUAL preconditions — not codex's
  │   speculative pre-hardening)
  └─ Ride-along: any of the 8 not-fired BLs whose trigger fires DURING
      v0.9.0 (e.g., if BL-009's design adds a new synthesis writer,
      THEN BL-synthesis-cluster-hash-not-null-enforcement's trigger #1
      fires and it bundles in.)
```

Cost of this path: 1 small refactor PR + 1 one-line CLAUDE.md fix +
opening one discussion.

Cost of Option B (codex's hardening sprint): plan + 4 BLs + work +
review + close-out + version tag + CHANGELOG + retroactive scope
cleanup if any of the 3 weak-fired items prove not-actually-precondition
to BL-009.

The minimum-machinery framing: **the right answer to "should we have
a sprint?" when the inventory yields one fired-trigger XS-to-S item is
"no, just commit it."**

## Disagreements (with analysis 023)

- **Disagree with Option B framing as "trigger-fired hardening
  sprint."** It contains 1 fired item + 2 weak-fired/not-fired items
  promoted under "BL-009 readiness" — the trigger-discipline rule
  forbids this.
- **Disagree with codex's "run v5 migration before locking scope."**
  v5 migration is an ops task. It does not need scope-locked sprint
  framing to run; the user can run it tomorrow and discover whatever
  it discovers, independent of any v0.8.5 question.
- **Disagree with gemini's "v0.8.5 is prerequisite for BL-009 success"
  Transparency Pivot.** Those items don't exist as BLs — analysis 023
  noted this. The minimum-machinery answer to "we don't have BLs for
  this" is "open `/ae:discuss` on the UX problem", not "smuggle
  features into v0.8.5 that violate 0.x.5 patch convention."

## Agreements (with analysis 023)

- **Agree** with archaeologist that BL-dreaming-module-split is the
  single cleanly-fired trigger.
- **Agree** with challenger C1/C5/C6: v0.8.5 is theater; Phase 2 chain
  is BL-009 → BL-010 → BL-011/BL-013; correct answer is probably skip.
- **Agree** with challenger that CLAUDE.md "67% residuals" line is
  stale (validated independently).

## Open Questions

1. **Is the user proposing v0.8.5 because they have unstated
   work/intent that hasn't surfaced in this analysis?** Challenger
   C6 names this directly. If yes, the user should name it. If no,
   the answer above is the minimum.
2. **Does BL-009 design discussion need to happen *before* the
   module-split refactor, or in parallel?** I believe parallel — the
   refactor is pure mechanical move-rename, no API/contract surface
   touched, safe to land while design discusses. But this is a
   judgment call.
3. **If the project genuinely wants version-tag cadence (the
   threshold-triggered model from framing.md:30-37), is that a
   delivery-process decision worth its own discussion** rather than
   piggy-backing on a thin v0.8.5 question? I lean yes — the question
   "should we tag versions on N-PR threshold" deserves its own
   `/ae:discuss` not a sprint-scope debate.
