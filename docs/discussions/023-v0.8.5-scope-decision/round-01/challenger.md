---
role: challenger
round: 1
discussion: "023"
topic: "01"
---

# Challenger Round 1 — v0.8.5 delivery shape + scope

## Findings (with file:line evidence)

### F1: "Skip v0.8.5" is NOT the stronger position — one trigger is structurally different from "schedule sentiment"

Discussion 022 conclusion line 74: "Move to v0.9.0 per the roadmap theme." That is the right baseline. BUT: the framing explicitly says "The argument for v0.8.5 has to come from real fired-trigger work, not from 'we should have a sprint between releases.'" — BL-dreaming-module-split IS real fired-trigger work, not sentiment. The trigger in `docs/backlog/BL-dreaming-module-split.md:32-36` says: "Fires when BL-008 plan lands. The first commit of BL-008 should preferentially [split]." BL-008 landed 2026-04-20 (`d053720`). That is 4 days before v0.8.0 closed. The split was NOT done. This satisfies the trigger definition. The challenger's own C1 argument ("v0.8.5 is theater — discussion 022 named v0.9.0 next") cannot hold against a factually fired trigger. Theater is "schedule-because-we-should-have-a-sprint." This is "a BL fired and was not consumed."

**The skip-v0.8.5 argument is weaker than the analysis credits on this specific item.**

### F2: BL-dreaming-module-split trigger status — the "ship has sailed" argument is wrong, but for a different reason

The analysis says the BL "CLEANLY FIRED." That is correct. The team-lead instructions ask me to test whether "the user accepted the bundled shape and the BL is now stale."

Evidence: BL-008 Step 1 (`7245994`, 2026-04-20) modified `{src/core/db.rs, src/core/decay.rs, src/core/mod.rs}` — it did NOT touch dreaming.rs. Step 2 (`fcb4f26`) modified `{src/bin/cli.rs, src/core/dreaming.rs}` — it extended dreaming.rs rather than splitting it. No commit in the plan 013 history created `synthesis_pipeline.rs` or `decay_pipeline.rs`.

The BL's trigger language says "first commit of BL-008 *should preferentially* split." "Should preferentially" is a soft directive, not a hard gate — the BL cannot be closed by the split failing to happen; it is still open and the trigger has fired. The user did NOT "accept the bundled shape" in a formal sense (no BL-close commit says the split was accepted). The BL is open in `docs/backlog/BL-dreaming-module-split.md`.

**However**: the "should preferentially" language means the split was guidance that the implementation overrode for scope-hygiene reasons. This is a legitimate override. The BL is not stale — it is legitimately open and fired. But the "ship has sailed" concern is real: plan 013 added BL-008 decay logic into dreaming.rs (Step 2), making dreaming.rs now 1326 lines instead of the original 641. The split is now MORE urgent than when the BL was filed — not stale.

Verdict: **BL is validly open and its trigger is validly fired. Not stale.**

### F3: Is the split enough to anchor v0.8.5, or should it ride in v0.9.0?

Counter-argument to Option A (skip): BL-009 (MCP Dream Tool) will call `run_synthesis_pass` from a new MCP context. That function lives in dreaming.rs at line 399 alongside `run_dreaming_with_config` at line 85. If they stay co-located, BL-009's diff will touch dreaming.rs to import and wire `run_synthesis_pass`, creating a merge surface across the same 1326-line file. The split REDUCES blast radius for BL-009 — it is a legitimate pre-BL-009 readiness item.

**Counter to that counter**: BL-009 will import `run_synthesis_pass` by module path. Whether that path is `dreaming::run_synthesis_pass` or `synthesis_pipeline::run_synthesis_pass` is a trivial rename — not a material blast-radius reduction. The "it cleans up BL-009's diff" argument is weak.

**Net assessment**: the split justifies v0.8.5 only if it is combined with at least one other meaningful fired-trigger item. Alone, it is a one-commit PR that does not need a version tag.

### F4: FK pragma — the trigger has NOT fired by the BL's own definition

The `.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md:55-60` lists three trigger conditions:
1. "First observed production corruption traceable to unenforced FK"
2. "Next schema addition that introduces a new FK-bearing table"
3. "Schema v6 migration lands"

Current schema version: v5 (`src/core/schema.rs:5`). Schema v6 does not exist. No new FK-bearing table has been added post-v5. Condition 1 is the strongest fire candidate — the plan 017 pre-check found the orphan synthesis row `529d3212-...`, but that was a zero-link synthesis row, not an orphan FK link in `memory_synthesis_links`. The analysis conflates "the pre-check caught a real issue" with "FK corruption traceable to unenforced FK" — these are different failure modes.

**Additionally**: there is a duplicate BL. `docs/backlog/BL-fk-pragma-and-deletion-safety.md` (origin: BL-007 review, 2026-04-18) covers the same fix with a DIFFERENT trigger: "fires when a plan adds a DELETE FROM memory_entries path, OR a plan adds an audit/provenance feature that depends on link integrity." No DELETE path exists. BL-009 might add link-integrity queries — but that is conditional. Neither trigger has fired.

**Verdict: BL-enable-pragma-foreign-keys trigger has NOT fired. The analysis labeled it "weak fire" but based on the actual BL text it is "not fired."**

### F5: BL-v5-migration-operator-docs — trigger is ambiguous but leans toward fired

The BL's trigger conditions (`BL-v5-migration-operator-docs.md:74-77`): (1) "First production migration to v5 on a DB with coalesced_duplicate rows (log line...)"; (2) "Operator reports wrong row kept"; (3) "Next doc polish sprint."

The user's production DB has the orphan row. Plan 017 ran on production. If the log line was emitted, condition 1 has fired. However, the BL says it fires on a "coalesced_duplicate" row — the orphan was a "zero-link" row. The distinction matters: the coalesce heuristic fires when two synthesis rows exist for the same cluster; the orphan is a different failure mode. Condition 1 may NOT have fired.

Condition 3 ("Next doc polish sprint") is self-referential — it means "fires whenever we do a doc sprint," which is circular. That is not a real trigger.

**Verdict: marginally fired at best (condition 1 uncertain, condition 3 circular). The XS effort means the cost of scheduling it is low. But it does not anchor a sprint.**

### F6: The cost asymmetry on skip vs ship

The analysis asks: what's the cost of being wrong?

- **Wrong to skip v0.8.5**: dreaming.rs stays at 1326 lines. BL-009 starts with a large file. The split happens anyway inside v0.9.0 where it competes for review attention with BL-009. BL-fk-pragma-and-deletion-safety (old) stays unfixed until v0.9.0. Docs gap persists for the operator who already ran v5 migration. Cost: low-medium. No data corruption risk — the FK trigger hasn't fired (no DELETE paths).

- **Wrong to ship v0.8.5 (Option B)**: we spend ~0.5-1 day of agent work on a pure refactor sprint that produces a smaller dreaming.rs and a docs page. No user-visible behavior change. The sprint machinery (plan, review, commit, tag) costs time. If the BLs don't materially reduce risk before BL-009, that time is overhead. Cost: low. The biggest loss is sprint bookkeeping for a marginal gain.

- **Wrong to ship v0.8.5 (Option C, transparency features)**: these are new features, not patch-shape. They do NOT exist as BLs. They require their own discuss→plan→work→review cycle. Scheduling them into v0.8.5 would violate the trigger-discipline rule AND the 0.x.5 semver convention. Cost if wrong: wasted sprint + awkward version history.

**Conclusion on cost asymmetry**: the skip-v0.8.5 cost is low. The hardening-sprint cost is also low but positive. There is no corruption risk on either path. The decision is not between safe and dangerous — it is between "ship a cleanup sprint" and "fold cleanup into v0.9.0."

### F7: Is the user's desire for v0.8.5 evidence-driven or sentiment?

The framing says discussion 022 named v0.9.0 as next. The user is proposing a v0.8.5. That is the user re-opening a closed question. Two scenarios:

A) The user is acting on BL-dreaming-module-split's fired trigger — legitimate. The protocol says "argument for v0.8.5 has to come from real fired-trigger work." BL-dreaming-module-split qualifies.

B) The user wants a cleanup moment before the big feature. That is schedule sentiment. Protocol says reject.

The fact that this discussion exists suggests the user believes there IS real fired-trigger work. The evidence supports scenario A for at least one item (dreaming split). The challenger cannot honestly call it "pure theater" given that fired trigger.

**Verdict: not pure theater. One legitimate fired trigger. The question is whether one S-size refactor justifies a version tag.**

### F8: BL location and discovery problem — docs/backlog vs .ae/backlog divergence

BL-dreaming-module-split lives in `docs/backlog/` not `.ae/backlog/unscheduled/`. The analysis's unscheduled BL table lists 9 items from `.ae/backlog/unscheduled/` — BL-dreaming-module-split is NOT in that list. This is a structural problem: the BL is "open" but invisible to `/ae:roadmap` tooling that scans `.ae/backlog/unscheduled/`. If the team ran `/ae:roadmap plan v0.8.5` without first migrating this BL into `.ae/backlog/unscheduled/`, it would be silently skipped.

**This is the real blocking issue for any v0.8.5 planning run: BL-dreaming-module-split must be migrated to `.ae/backlog/unscheduled/` before `/ae:roadmap plan v0.8.5` can pick it up.**

## Agreements (none in Round 1)

None — Round 1 is independent research.

## Disagreements (none in Round 1)

None — Round 1 is independent research.

## Open Questions

1. **Was the `coalesced_duplicate` log line actually emitted** during plan 017's production run? If yes, BL-v5-migration-operator-docs condition 1 has fired definitively. If no, only condition 3 (circular) applies.
2. **BL-dreaming-module-split migration**: before any sprint planning, does the BL need to move from `docs/backlog/` to `.ae/backlog/unscheduled/`, or does the roadmap tool scan both locations?
3. **Does BL-009 import `run_synthesis_pass` directly, or does it go through a higher-level abstraction?** If BL-009 would add a new MCP tool that calls through `mcp_tools.rs` → `db` → `dreaming::run_synthesis_pass`, the split produces a rename that BL-009's author would need to track. Low cost but real coordination.
4. **Old vs new FK BL reconciliation**: the two FK BL files (`docs/backlog/BL-fk-pragma-and-deletion-safety.md` and `.ae/backlog/unscheduled/BL-enable-pragma-foreign-keys.md`) have divergent triggers. Which is the authoritative trigger? The newer one (plan 017 origin) is more conservative (waits for v6); the older one fires when a DELETE path appears. This gap should be resolved before either is scheduled.
