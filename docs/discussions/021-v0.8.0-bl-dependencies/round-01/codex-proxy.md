---
id: "021"
round: 1
agent: codex-proxy
created: 2026-04-23
reasoning_effort: high
---

# Round 1 — Codex Proxy Analysis

## Plan-Quality Cross-Family Lens

Codex (OpenAI family) reviewed all four topics through the lens of acceptance criteria verifiability, step decomposition, and one-pass review depth. High reasoning effort applied to foundational plan-policy decisions.

---

## Topic 1: Bundle boundary for decay-cluster plan

**Assessment**: Bundling the 3 active decay BLs into one plan is reviewable and correct—**but only if decomposed by dependency edges, not by vague "operator hardening" grouping**. The shape must preserve internal structure: schema contract first (with AC for field + schema file + integration test), script hardening second (preflight/db-path/RUST_LOG/CI test each verifiable), ops/doc third (rollback + threshold semantics).

If bundled ACs collapse to "all 3 BLs done," review granularity is lost and reviewers cannot verify closure independently. With explicit step-to-AC mapping (as in plan 013 and plan 014), the bundle stays smaller and comfortably within one-pass review scope.

**Sequencing**: Bundling improves downstream verifiability because internal dependencies (schema precedes both its consumers) remain internal to one review unit. Splitting creates artificial cross-plan coordination around `cli.rs` and the schema consumer path—exactly the drift that makes `/ae:review` less reliable. Prefer the bundle.

**Policy**: Existing review mechanisms suffice. Only guardrail needed: scope discipline. Do NOT pull in unrelated decay-adjacent cleanup like `release.yml` or BL-010 daemon prep. The bundle is about *review-originated* polish, not design-ahead.

---

## Topic 2: Fate of 2 defer-until-trigger items in v0.8.0

**Assessment**: Leaving those 2 trigger-gated items open breaks the sprint's AC as written. The roadmap gate is absolute: "All 7 review-originated follow-ups closed" and "All 4 `BL-decay-*` items closed" + "All 3 open `BL-synthesis-*` items closed." "Open but unmet" ≠ "closed."

If the items remain in-sprint, the gate is not verifiably met. The AC is binary, not trigger-aware, so the close decision becomes ambiguous.

**Sequencing**: `/ae:roadmap remove` is the sequencing-clean option. It restores an honest active set without erasing trigger logic. Closing as "superseded-by-trigger" would be semantically wrong—they are unresolved, not replaced.

**Policy**: Existing mechanisms are not sufficient if items stay. The gate text is absolute. For v0.8.0: remove is required. Without removal, later close semantics are ad hoc and damage downstream plan verifiability.

---

## Topic 3: Which hardening actions ship in v0.8.0

**Assessment**: Action 5 (threshold-mode daemon flag) should **not ship** in v0.8.0. An AC that references BL-010 when BL-010 has no plan and no settled design is not verifiable; it is a placeholder, not an acceptance condition.

Actions 1–4 are verifiable now because they improve current robustness and CI coverage. Action 4's trigger (cargo clippy + cargo test in CI) was satisfied by plan 014. Action 5 would add dead code and speculative daemon semantics that BL-010 may later change.

**Sequencing**: Including action 5 weakens downstream verifiability. It bakes a pre-BL-010 interface that BL-010 may break. Excluding it keeps the hardening plan reviewable in one pass and allows BL-010 to settle its own interface independently.

**Policy**: Action 5 also violates the sprint's theme: "review-originated follow-ups only, no new design." Threshold mode is design-ahead daemon prep, not closure of a current review defect. Existing review mechanisms are enough to reject it. The BL should be split: ship 1–4 now, re-file threshold as its own BL when BL-010 design lands.

---

## Topic 4: Sprint-commitment policy for unresolved pre-conditions

**Assessment**: The current `gate:` frontmatter is insufficient. It defines the desired end state, but does not specify close-state semantics when committed items have unresolved preconditions. From a plan-quality lens: the gate is not fully reviewable in one pass because reviewers can detect the ambiguity, but the roadmap does not tell them how to resolve it.

**Sequencing**: This matters beyond v0.8.0. Without explicit policy, future sprints can again admit "not now" items, and every close decision becomes ad hoc. That directly harms dependency accuracy and sprint-level AC verifiability.

**Policy**: A separate lightweight rule is needed. Preferred: add an admission marker (e.g., `admission_status: defer-until-trigger`) and a rule that such items are excluded from sprint commitment unless the trigger is expected within the sprint. If they slip in, they must be removed before sprint close, not counted as open. This keeps sprint gates binary and reviewable—much better than "open-but-deferred counts as closed," which introduces conditional logic into what should be a simple state check.

---

## OpenAI-family Specific Observations

1. **Scope-discipline patterns**: Codex favors explicit scope lines and clear rejection criteria. On Topics 1 and 3, the OpenAI reasoning strongly preferred "what stays out" language over "what gets bundled." This aligns with Codex's general bias toward narrower, more reviewable plans.

2. **Verifiability-first thinking**: All four topics are framed through AC verifiability and one-pass review depth. Codex did not trade off "shipping this in v0.8.0" vs. "deferring cleanly"—the decision pivoted on "can reviewers verify closure in one pass?" That is foundational to OpenAI-family planning discipline.

3. **Placeholder-detection**: On Topic 3, Codex explicitly flagged AC-by-reference-to-nonexistent-plan as unverifiable. This is a strong signal for review-gate design: forward references to unsettled designs are review smell.

---

## Disagreements Between Proxy & Codex

None. The proxy's own reading of plan 013, plan 014, and the topic summaries aligned with Codex's findings on all four points. The analysis is coherent.

---

## Open Questions

1. **Topic 1 detail**: Does the bundled decay plan require a separate AC for `cli.rs` string contract, or is the string contract verifiable as part of the schema BL's AC? (Minor decomposition question.)

2. **Topic 4 operationalization**: If the admission marker is adopted, does it live in frontmatter (alongside `status:`, `priority:`, etc.), or as a tag in the BL body? Frontmatter is easier for tooling; body is less ceremony.

3. **Downstream timing**: Topic 3's recommendation (defer action 5) implies v0.8.0 has one fewer BL to complete (BL-verify-decay-script-hardening ships 4 of 5 actions). Does that change v0.8.0's size/velocity math, or is the plan already sized to absorb the smaller scope?

