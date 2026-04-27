---
agent: gemini-proxy
round: 1
timestamp: 2026-04-28T14:32:18Z
model: gemini-2.5-flash
---

# gemini-proxy — Round 1: v0.8.5 scope decision

## Findings

### 1. BL-009 Success Depends on Foundation Clarity

The "residuals anxiety" Kai feels is not a math failure (plan 011 validated the algorithm). It's a **transparency failure**: Kai cannot see *why* memories are residual or the overall topology that creates disconnection. When BL-009 ships, it adds Claude's generative layer on top of this invisible foundation. Result: *more* decision-making (now Claude's choices are added) without addressing the foundational opacity.

**Critical insight**: A "residuals context CLI command" shipping in v0.8.5 becomes a **prerequisite for BL-009 success**. Without it, Kai will experience:
- Existing confusion about system-level residuals (topology, pre-Claude filtering)
- New confusion about Claude's choices (why *this* cluster was picked for dreaming, why *this* subset of memories was used)
This dual opacity would make BL-009 much harder to trust, debug, and use effectively.

**Evidence**: BL-009's stated scope (~100-150 LOC) focuses narrowly on "cluster→synthesize→ingest" + "explain Claude's generation logic." It does NOT explain why memories become residual at the system level or why topology creates disconnection. That gap must be closed first.

### 2. BL-dreaming-module-split Trigger is Definitively Fired

BL-008 shipped 2026-04-20 as plan 013. The trigger explicitly says "first commit of BL-008 should preferentially split." The trigger was not acted on. This is a breach of Kai's own trigger-discipline rule (discussion 021), which he established precisely to avoid backlog rot.

The current state: 1326-line `dreaming.rs` mixes three concerns (promotion/demotion + LLM orchestration + test stubs). This is both a **DX problem** (Kai has to hold three separate mental models when reading/modifying the code) and a **risk problem** (any change to promotion logic touches orchestration code and vice versa, increasing regression risk for BL-009).

**Signal**: Kai's proposal for v0.8.5 is fundamentally a request to honor his own discipline rule and restore closure on a triggered item. This is not "theater"; it's **process integrity**.

### 3. Kai's Implicit Psychology: Seeking a Forcing Function

Kai's decision to propose v0.8.5 despite discussion 022 explicitly naming v0.9.0 as next sends a clear signal: **he is not deferring; he is seeking structured permission to address foundational problems.**

Evidence of this psychology:
- He explicitly identifies BL-008 split as a fired trigger (not a stretch)
- He names personal UX pain ("residuals anxiety," "audit fatigue") that blocks his confidence in moving forward
- He brought this to discussion format rather than just scheduling work (seeking validation that the concern is legitimate)
- He has NOT proposed rushing into new features; he is asking whether the foundation is solid enough for BL-009

Interpretation: After intense Phase 2.1 work, Kai is recognizing that pushing directly into BL-009 on an unstable foundation (un-split BL-008) + with unresolved UX anxiety = **risk of friction, debugging burden, and reduced effectiveness during the exciting new feature work.** A focused v0.8.5 is a **strategic pause to sharpen tools before the complex carving**, not a detour.

### 4. Industry Convention Strongly Supports Patch-Only Scope

Across Rust ecosystem (tokio, tantivy, PyO3): 0.x.5 releases are hardening-shaped — bugfixes, refactoring for clarity, docs, DX improvements. NOT new user-facing features. Using 0.x.5 for new features is the awkward case the community explicitly avoids.

This means v0.8.5 can legitimately include:
- **Module refactoring** (BL-dreaming-module-split) — pure DX, no API change
- **CLI subcommand enhancements for existing data** (e.g., `mengdie audit explain <memory_id>`, summary views, enhanced error messages) — reorganization of existing internal state, not new features
- **Schema invariant enforcement** (FK pragma, not-null checks) — hardening, not expansion

But NOT: Dashboard, Batch Audit UI, edge export (those are v0.9.0+ scope).

### 5. "Skip v0.8.5" Option Violates Kai's Own Discipline

Discussion 021 established trigger-discipline as critical after v0.8.0 shipped with 2 committed-but-not-fired items that had to be retroactively removed. Kai knows this pattern is bad. By proposing v0.8.5, he is **preventing that mistake from repeating**: acting on the cleanly-fired BL-dreaming-module-split trigger rather than letting it fester into v0.9.0.

Skipping v0.8.5 means:
- Violating trigger-discipline (fired trigger is ignored)
- Building BL-009 on unstable codebase (un-split BL-008)
- Attempting to work on new synthesis logic while feeling anxious about residuals (mental friction)

This is a **setup for the exact problem discussion 021 identified**: scheduled work that should have been done first, now causing friction and rework.

## Agreements

1. **New features out of scope for v0.8.5**: Dashboard, Batch Audit, edge export are unequivocally v0.9.0+ scope. These violate 0.x.5 patch convention and require full discuss→plan cycle before `/ae:roadmap` can schedule them.

2. **BL-008 split is essential for BL-009 readiness**: Attempting to implement new synthesis logic while the foundational dreaming module is tangled is asking for unnecessary cognitive load and risk.

3. **Residuals transparency is prerequisite for BL-009 success**: If Kai can't *see* why residuals exist at the system level, adding Claude's layer of decisions on top will amplify confusion, not resolve it.

4. **Trigger-discipline matters**: Firing a trigger and then ignoring it (as happened with BL-dreaming-module-split) degrades the value of the discipline rule. Honoring fired triggers is how solo devs prevent backlog rot.

## Disagreements

None at this time. Codex's technical-debt lens (P1: FK pragma + not-null enforcement) and this UX/momentum lens both converge on the same core recommendation: ship v0.8.5 with foundational work rather than skip it.

(Minor: whether residuals transparency work is best as a *CLI subcommand* vs. *improved logging/error messages* vs. *lightweight documentation* — implementation shape differs, but the need is aligned.)

## Open Questions

1. **Residuals transparency: how to implement without adding new features?**
   - Simple option: `mengdie audit explain <memory_id>` — show why a single memory is classified as residual (missing sources, failed clustering threshold, etc.)
   - Summary option: `mengdie audit summary` — group residuals by category, show counts, explain what "residual" means in this corpus
   - Documentation option: update CLAUDE.md "Project Status" to explain the 67% residuals finding + what it means for Kai's workflow
   - Combinations of the above?
   The constraint: no *new calculations*, just presentation of existing system state.

2. **Scope of BL-dreaming-module-split: API-surface breaking?**
   - If the split is a pure refactor (extract `promote()` + `demote()` + `synthesize()` + tests into separate modules, same public API), it's 100% safe.
   - If the split requires minor adjustments to `DreamingResult` or how callers interact with promotion logic, that's still patch-safe (internal adjustment).
   - Any signature changes to `src/core/dreaming.rs` public functions would need validation.

3. **Timeline: does v0.8.5 delay v0.9.0 materially?**
   - BL-dreaming-module-split: ~100 LOC pure refactor, should be ~4 hours agent work
   - Residuals transparency: ~50-100 LOC (new CLI subcommand or enhanced logging), ~2-3 hours
   - Total: ~1 day of agent work, review cleanup in one `/ae:review` pass
   - BL-009: ~100-150 LOC, starts fresh after v0.8.5 closes
   - Realistic v0.8.5 delivery: 48 hours from go-ahead to closed, minimal delay to v0.9.0 runway. However, the *code quality* and *cognitive clarity* benefit to v0.9.0 work far outweighs the 1-day delay.

4. **Is this analysis reflecting Kai's *actual* psychology, or a projection?**
   - This is inferred from Kai's decision to open the discussion + the fired-trigger state. Direct confirmation from Kai would validate whether the "seeking a forcing function" interpretation is accurate.

## Recommendation

**Ship v0.8.5 with tightly-scoped, high-impact work:**

### Core v0.8.5 Scope
1. **BL-dreaming-module-split** (S, ~100 LOC) — extract promotion/demotion + orchestration + tests into cleanly separated modules. Pure refactor, same public API. Restores code clarity and enables easier BL-009 development.
2. **Residuals clarity work** (XS–S, ~75 LOC) — implement one or more of:
   - `mengdie audit explain <memory_id>` CLI subcommand showing why a memory is residual
   - Update CLAUDE.md "Project Status" to explain the 67% residuals finding for Kai's own reference
   - Enhanced CLI output on `mengdie dream` showing residuals summary
3. **Optional hardening** (XS, if bandwidth) — `BL-enable-pragma-foreign-keys` or `BL-synthesis-cluster-hash-not-null-enforcement` (per Codex's P1 items). But not required for v0.8.5 to be valuable.

### Why This Shapes BL-009 Success
- **Cleaner codebase** for BL-009 logic to live in
- **Kai understands residuals** before adding Claude's layer of decisions
- **Trigger-discipline restored** — fired work is honored, preventing backlog rot
- **Psychological momentum** — Kai closes technical debt and UX friction before tackling the exciting new feature

### Timeline and Risk
- **Duration**: ~1 day agent work + review cleanup
- **Risk**: Very low (pure refactor + CLI enhancement, no schema changes in core scope)
- **Delay to v0.9.0**: Negligible (~1 day); benefit to v0.9.0 code quality and Kai's confidence is substantial

### Alignment
- Honors discussion 021 trigger-discipline rule
- Respects 0.x.5 patch convention (no new features)
- Sets up BL-009 for cleaner, faster development
- Addresses Kai's real UX pain (residuals anxiety) through foundational transparency, not new UI
