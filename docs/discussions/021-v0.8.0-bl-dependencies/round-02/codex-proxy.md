---
id: "021"
round: 2
agent: codex-proxy
created: 2026-04-23
reasoning_effort: high
updated_findings: true
---

# Round 2 — Codex Proxy Findings

## Updated Evidence Integration

Round 1 peer reviews surfaced critical factual corrections:
- **Archaeologist**: actions 1 + 3 are already LIVE in `scripts/verify-decay.sh`; effective bundle is ~120–150 LOC, not 150–200.
- **Archaeologist**: ops-doc-polish edits `format_dreaming_line` (cli.rs:226), not `format_structured_json` (cli.rs:207). Two separate adjacent functions 19 lines apart.
- **Minimal-change-engineer**: argues 2+1 split (json-schema-contract + verify-decay-hardening together; ops-doc separate) preserves hard-dep coupling while improving review clarity per-plan.
- **Gemini**: flags rollback-granularity risk—bundled plans need clear rollback strategy if partial undo is needed post-merge.

---

## Q1: Plan-quality AC verifiability at actual scope

**Finding**: Bundling all 3 is still reviewable at ~120–150 LOC; it does not break AC verifiability. **But the updated evidence removes most of the argument for bundling everything.**

The better plan-quality boundary is 2+1 split:

- **Plan 015**: json-schema-contract + verify-decay-hardening (M+M, ~100 LOC)
- **Plan 016**: ops-doc-polish (S, ~20–30 LOC)

**Why the split yields better clarity**:
- The hard seam is real: `format_structured_json` feeds `scripts/verify-decay.sh:62`. These BLs must co-test.
- The ops-doc BL lives on a different surface: mostly `format_dreaming_line` (separate function) plus documentation. No shared test harness with the schema contract.
- Four-agent review is clearer when each plan has one dominant AC family. 2+1 prevents cross-contamination between machine-contract findings (schema correctness, test coverage) and operator-doc findings (rollback clarity, docs accuracy).

**Trade-off resolved**: 2 plans with cleaner per-plan ACs beats 1 bundle with less overhead. The "plan overhead = 1 S-class item" argument from Round 1 breaks when ops-doc is pure documentation (20 min review, no CI drift churn like code plans).

---

## Q2: Sequencing with confirmed dependencies

**Finding**: Yes, strong case for 2-plan sequence.

**Plan 015 internalizes** (M+M, hard-dep inside):
- JSON schema contract stabilization (cli.rs `format_structured_json`)
- stderr integration test (consumes schema version from verify-decay.sh output)
- script hardening actions 1–3 (already live; action 1 is clarification, action 3 is normalization)
- Action 4 CI coverage (shell test + CI job hook)
- Minimal schema-related doc update if needed (e.g., JSON event format docstring in cli.rs)

**Plan 016 holds** (S, soft-dep):
- Ops-doc rollback procedure (references breaches[] array, which exists post-contract regardless of schema_version)
- `format_dreaming_line` cleanup + threshold-mode discussion sections
- Arrow-contract/format decision (no code touch, pure docs)

**Sequence advantage**: Plan 015 ships the contract + consumer lock + tests in one atomic unit. Plan 016 can slip one sprint without breaking the hard dep. If rollback is needed later, reverting Plan 016 alone is trivial (docs-only revert). Reverting Plan 015 requires care but stays atomic (schema + script + test all reverse together).

**Comparison to bundling all 3**:
- Bundling: 1 plan, lower plan overhead, higher review scope, partial rollback risk if any BL needs undo.
- 2+1: 2 plans, clear AC boundaries, easier partial rollback, sequencing flexibility.

**Verdict**: 2-plan sequence is higher plan-quality shape.

---

## Q3: Admission policy implementability

**Finding**: `admission_status: defer-until-trigger` is implementable as a minimal `/ae:roadmap plan` skill change.

The AE roadmap skill already works by reading BL frontmatter and defining optional BL fields in the skill spec itself (no separate schema engine). Implementation path:

- Add optional BL frontmatter field: `admission_status: [enum: defer-until-trigger | standard]`
- Skill reads during `plan` phase and visibly marks deferred items in selection UI
- Non-interactive `plan --items <list>` refuses if any selected BL has `admission_status: defer-until-trigger` (unless `--force-deferred` flag is passed)

**Estimate for upstream AE BL**:
- Skill spec change: ~25–40 LOC (field definition + interactive filtering)
- Tests: ~40–70 LOC (1 assertion test for marking, 1 execution test for refusal)
- Docs/changelog: ~5–10 LOC
- **Total**: ~70–120 LOC, low test burden, no parser redesign needed.

**If you also want** validation via `--gaps` or enum strictness: +M follow-up, out of scope for v0.8.0.

**Cost-benefit for mengdie**:
- One-time skill change: ~1 hour work upstream, ~30 min integration downstream.
- Recurring benefit: `plan` command prevents admission error at least once per major sprint cycle (low-frequency issue).
- Challenger & minimal-change both argue this is over-automation for a 2-item problem. **Fair point**. But if you expect to use mengdie long-term and run ~4–6 sprints/year, the one-time cost amortizes.

---

## Q4: Rollback-granularity risk mitigation

**Finding**: `git revert` is not a complete mitigation if all 3 BLs land as one bundled/squashed commit.

**Specific case modeled** (ops-doc-only rollback):
1. Schema contract lands
2. Script hardening lands and passes tests
3. Ops-doc polish lands
4. Later, only ops-doc needs rollback

**If all 3 are squashed into 1 commit**:
- Atomic revert via `git revert` reverts all 3 (too coarse; loses good schema/script work)
- Partial revert requires manual surgical commits:
  - Revert `docs/operations/dreaming-decay.md` changes
  - Revert `format_dreaming_line` hunk in cli.rs
  - Update unit tests tied to that function's output
  - Re-run CI on the partial-revert commit
  - This is not catastrophic but is not clean one-command undo either

**If split into 2+1**:
- Plan 015 is one commit or squashed unit (good: schema + script + test are atomic)
- Plan 016 is separate commit/PR (good: revert ops-doc cleanly without touching schema/script)
- Partial rollback cost → minimal (single docs PR revert + maybe one test file update)

**Mitigation if you choose bundling**:
- Require separate logical commits per BL *within* the bundle (e.g., `git show <commit> | grep -E "^@@.*cli.rs.*" | wc -l` tracks hunks)
- Or split into Plan 015 + Plan 016 (removes risk outright)

**Verdict**: Gemini's rollback concern is real but bounded. The 2+1 split largely removes it.

---

## Net Recommendation: Revise Round 1

**Codex Round 1 said**: Bundle all 3 with explicit step/AC mapping.

**Codex Round 2 (updated)**: Use 2+1 split.

**Reasoning chain**:
- Updated evidence shows ~120–150 LOC effective scope (smaller than estimated).
- Archaeologist confirmed ops-doc is separate code surface (different cli.rs function).
- Minimal-change and Gemini both flagged 2+1 as higher-quality shape.
- Hard dep (schema → script) stays internal to Plan 015 (M+M).
- Soft dep (ops-doc references schema) defers to Plan 016 (S).
- Rollback risk disappears with split.
- Four-agent review gets clearer per-plan ACs.

**Open question from Round 2**: minimal-change asked whether `/ae:roadmap close` refuses to close v0.8.0 with open items. If yes, Topic 2's "remove both defer-items" becomes forced. If no, minimal-change's "do nothing" position (leave them in-sprint without action) becomes viable.

