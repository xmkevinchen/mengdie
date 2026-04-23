---
round: 1
proxy: gemini-proxy
date: 2026-04-23
lens: scope & integration risks
---

# Gemini-Proxy Round 1 Findings

## Findings (per topic — scope & risks lens)

### Topic 1: Bundle boundary for the decay-cluster plan

**Scope risk**: Bundling 3 BLs into a single plan creates legitimate "all-or-nothing" coupling via the shared `src/bin/cli.rs` touch. If the plan must be rolled back post-merge, a surgical revert of format_structured_json could inadvertently break the JSON schema contract or the ops-doc references, cascading into test failures in the verify-decay script. **Mitigation**: The shared logic is localized (single function) and additive (new JSON schema enum + documentation), so the risk is containable — but requires reviewing the specific cli.rs changes to confirm no cross-BL type dependencies. 

**Bundling judgment**: Bundling *reduces* **plan/review overhead** (1 plan instead of 3) but *increases* **rollback granularity risk**. The tradeoff favors bundling for v0.8.0 because the BLs are small (M+M+S) and the shared file is low-complexity. However, this pattern should be revisited if cli.rs grows or if future decay work introduces type-level contracts (e.g., JSON schema enum enforcement in Rust types).

### Topic 2: Defer items & roadmap.remove

**Scope risk — destructiveness**: `/ae:roadmap remove` is a critical unknown here. If removal is *administrative* (BL stays in backlog with body intact), risk is **low** and re-filing later is cheap. If removal is *destructive* (deletes the BL file entirely), risk is **high** and the trigger context is permanently lost unless separately documented. **Recommendation**: Before acting, verify what `/ae:roadmap remove` actually does to the BL file — does it move to a `.removed/` archive or hard-delete?

**Contextual decay risk**: Even if removal preserves the BL file, the "when" of the trigger gets decoupled from the "why" of the sprint. If the scale trigger fires next week (corpus > 50k) or BL-010 lands, there's a multi-day window where the context is "rediscovered" rather than "always warm." For a solo-dev project this is acceptable, but team projects risk trigger loss. **Recommendation**: Document the trigger condition in the BL itself (already done) and audit removal semantics before acting.

### Topic 3: Hardening actions & hidden dependencies

**False unblock risk**: Action 4 (CI coverage) was gated on "BL-ci-full-clippy-test" landing. Plan 014 just completed this, so the gate is *technically* satisfied. **However**, action 4 is "pairs with" not "requires" BL-ci-full-clippy-test — they're independent. The three environmental hardening actions (binary preflight, DB path, RUST_LOG normalization) can ship independently of CI coverage. **Recommendation**: Treat actions 1-3 as "core robustness" and action 4 as "observability gate"; both can co-ship but neither blocks the other.

**Hidden dependency — action 2 (--db-path flag)**: This introduces a **test isolation risk** that isn't explicitly called out. If the hardening script or CI environment assumes serial DB access (only one test writes to a given path at a time), the `--db-path` flag could enable tests to run in parallel with unique paths, which is good. **But** if the test *suite* assumes a single global DB path for discovery or if there's pre-existing test isolation tooling, the flag could break isolation assumptions. **Recommendation**: Confirm in the BL that `--db-path` is *always* explicitly passed in CI (never relies on a default) and that all tests are written to tolerate parallel execution with unique paths.

### Topic 4: Project-level admission gate

**Friction risk**: Introducing a gate that filters "not now" BLs at plan-time sounds defensive but creates **operational debt**. If the gate exists and v0.9.0 planning happens, there's a process ceremony to debate each trigger-gated BL *again* (is the trigger met? should we wait longer?). On a solo-dev project, this overhead is likely unjustified. **Recommendation**: Topic 2's case-by-case handling (remove or close the 2 specific items) is simpler than a policy. If a pattern emerges across multiple sprints, revisit then.

**"Ghost planning" risk**: Gemini surfaced a real downstream issue — if a policy exists that says "don't admit trigger-gated BLs," but the roadmap skill's `/ae:roadmap plan` command doesn't have an enforcement hook, developers may work around it by manually adding them anyway. The policy becomes "advice, not enforcement," which erodes credibility. **Recommendation**: If the team wants to enforce this, the policy must be *automated* (a pre-check in `/ae:roadmap plan`) or *documented as exception-driven* (case-by-case, Topic 2 style).

**Invariant conflict**: The discussion's framing mentions Invariant 3 — "frozen initial_items" after a sprint is planned. An admission gate at plan-time doesn't conflict with this, but it does create a **chicken-egg tension**: the sprint is planned (items frozen), then someone discovers a trigger-gated item *was* included (ad-hoc discovery, not gate enforcement). Retroactively applying the gate (Topic 2's remove/close decision) is fine, but making the gate *prospective* (future sprints) requires explicit `/ae:roadmap plan` skill changes, which is out of this discussion's scope.

---

## Google-family specific observations

**Gemini's framing aligned with risk analysis**: Gemini raised the "all-or-nothing" rollback risk (Topic 1) and "ghost planning" (Topic 4) concerns independent of my analysis. These are real cross-family blind spots — Codex/Doodlestein may focus on "does the code change work" but miss "what happens if we undo it halfway."

**Concurrency as a differentiator**: Gemini flagged the `--db-path` test isolation angle (Topic 3) as a Google-lens contribution — Google infrastructure tends to run tests in parallel aggressively, so this visibility is on-brand. Other lenses may not surface it.

---

## Disagreements between proxy & Gemini

**On Topic 1 rollback risk**: Gemini said bundling "amplifies" the blast radius. I partially disagree. In Rust, bundling *keeps changes atomic*, which actually *reduces* the window for partial failures. A surgical rollback of one bundled change is one `git revert`, not a multi-step unstitch. The real risk is that reverting forces a **forward fix** in the next commit (can't leave the codebase in partial state), which increases friction — not blast radius. Gemini's framing conflates "difficulty of rollback" with "scope of rollback." They're different.

**On Topic 4 policy**: Gemini emphasized the "bottleneck of perfection" friction. I agree this is high-cost, but I'd add that *not having* a gate also has a cost — it allows the current situation to repeat (trigger-gated items in sprints, wasting planning cycles). The real insight is that this is a **policy design decision** that should be made consciously, not defaulted. Gemini's framing is "don't bother," which is reasonable for solo-dev.

---

## Open Questions

1. **Semantics of `/ae:roadmap remove`**: Is the operation destructive or archiving? Blocks all downstream decisions on Topic 2.

2. **Test isolation model for `--db-path`**: Does the test suite assume serial execution or support parallel? Affects action 2 scope.

3. **Threshold-mode (action 5) vs. BL-010**: Is action 5 truly v0.8.0-scoped, or is it design work that belongs with BL-010 planning? The BL body says "paves the way for" but doesn't say "required by." Suggest Topic 3 also clarify this in its decision.

4. **Policy as automation vs. convention**: If Topic 4 opts for a project-level gate, is it enforced by `/ae:roadmap plan` tooling, documented as a convention, or dropped entirely? Determines operational cost.
