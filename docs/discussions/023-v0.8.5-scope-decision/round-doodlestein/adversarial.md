---
id: "023-doodlestein-adversarial"
title: "v0.8.5 scope decision — Doodlestein adversarial review"
date: 2026-04-27
reviewer: doodlestein-adversarial
---

# Doodlestein Adversarial Review

## Finding 1 — BL-dreaming-module-split: trigger window already closed, scope estimate wrong

**Where the conclusion fails**: The conclusion includes BL-dreaming-module-split as a v0.8.5 item,
citing "BL-008 shipped 2026-04-20" as the trigger. This is correct — but the BL body says:

> "Fires when BL-008 plan lands. **The first commit of BL-008 should preferentially** [do the split]."

"First commit of BL-008" is a prescriptive ordering constraint, not a standing trigger. BL-008 has
already shipped. The prescribed split structure (move `run_synthesis_pass` + `SynthesisResult` to
`synthesis_pipeline.rs`; add BL-008 decay pass to `decay_pipeline.rs`) was never executed.
`dreaming.rs` is now **1326 lines** with all three concerns — promotion, synthesis orchestration, and
decay — fully merged. The window for "first commit of BL-008" is closed.

The conclusion's scope estimate compounds the problem: it cites "dreaming.rs:157-311 ~100-150 LOC"
as the refactor target. Lines 157-311 are the **decay pass** (BL-008 demotion logic). The synthesis
orchestration function `run_synthesis_pass` starts at line 399 and `SynthesisResult` is at line 330.
The conclusion is pointing at the wrong half of the file. Whoever executes this plan step will
encounter a mismatch between the cited line range and the actual split target.

**Severity**: The item is still worth doing — the module is genuinely large and the three-concern
separation is sound. But the BL body's prescribed structure needs to be re-evaluated against the
current file layout before plan execution. The split is no longer "the first commit of BL-008";
it is a standalone refactor with an already-merged-in decay module that has its own `decay.rs`
peer file. The conclusion treats the BL as though it arrived fresh when its trigger window expired
months ago.

---

## Finding 2 — v6 schema migration blocks on production v5 orphan resolution; conclusion treats them as independent

**Where the conclusion fails**: The conclusion lists two items as independent parallel work:

- "Production v5 migration" (out-of-scope ops task, item 2 of the out-of-scope section)
- BL-synthesis-cluster-hash-not-null-enforcement (in-scope v0.8.5 item, v6 migration)

These are **sequentially dependent**, not independent. The v6 migration adds BEFORE INSERT/UPDATE
triggers on `memory_entries` that abort on `source_type = 'synthesis' AND valid_until IS NULL AND
synthesis_cluster_hash IS NULL`. Before those triggers can be installed, the pre-check in the BL
proposal must find zero violations — or the migration aborts.

The production orphan `529d3212-e809-4b81-a1f5-e15143df5128` is a synthesis row with zero entries
in `memory_synthesis_links`. This row already blocks v5 migration (schema.rs Pre-check 2, line
294-310, aborts on zero-link synthesis rows). Until v5 completes and the orphan is resolved, v6
cannot run. The conclusion's next-steps say:

> "Independently: write the CLAUDE.md fix + run the production v5 migration as separate
> non-v0.8.5 commits."

but does not flag that v6 (the in-scope BL item) cannot be merged-to-production until v5 is done
and the orphan is cleared. The gate condition in the roadmap command is:

> `--gate "Production v5 migration runnable; BL-009 design not blocked by schema invariants"`

"Runnable" is not "complete". The gate as written lets `/ae:roadmap plan` proceed while the orphan
is still live. A plan executor can implement and merge v6 locally (in-memory tests pass, no
production DB involved), then ship v0.8.5 — and then find that the production upgrade sequence is:
(1) resolve orphan, (2) run v5, (3) run v6. None of that sequencing appears in the conclusion or
the roadmap gate. The operator docs BL (`BL-v5-migration-operator-docs`) covers v5 recovery but
says nothing about the mandatory ordering constraint relative to v6.

**Severity**: This is the first failure point in real production use. The plan will be implemented
and reviewed cleanly in CI (in-memory DB, no orphan). The failure surfaces the first time someone
runs `mengdie` against `~/.mengdie/db.sqlite` after upgrading to v0.8.5 with v6 triggers installed
but v5 not yet complete. They get a v5 pre-check abort referencing the orphan, which the user must
then manually resolve before v5 can proceed, before v6 runs. The conclusion provides no runbook for
this sequence, and the gate condition does not prevent the plan from closing without it.

**Concrete failure scenario**: operator upgrades binary, starts `mengdie-mcp` or runs `mengdie
search`, `run_migrations` fires, v5 pre-check 2 aborts with "synthesis row(s) have zero entries in
memory_synthesis_links". User does not know to consult `BL-v5-migration-operator-docs` (which is an
unshipped doc-only item). v6 triggers are installed in the binary but can never run until the user
finds and resolves the orphan manually. BL-v5-migration-operator-docs should explicitly document the
orphan→v5→v6 ordering, not just the v5 coalesce heuristic. As written, it does not.
