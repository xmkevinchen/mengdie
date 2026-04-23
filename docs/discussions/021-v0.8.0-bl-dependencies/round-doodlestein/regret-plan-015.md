---
author: doodlestein
type: regret
plan: "015"
discussion: "021"
created: 2026-04-23
---

# Regret Analysis — Plan 015, Most Likely Reversed Decision

## Verdict: Step 1 — `schema_version: 1` bump rules (the "no bump for additive optional fields" rule)

This is the single most regret-prone decision in plan 015.

## Why

**The rule sounds precise but isn't.** The bump condition is defined as: bump on remove/rename/semantics-change; no bump for strictly-additive optional fields. In practice, "strictly additive" is evaluated by the field author at write time, not by consumers. A field added as "optional context" can silently become load-bearing for a consumer that keys logic on its presence or absence. The plan ships no mechanism to verify that a field remains semantically inert after it's added. The rule is a policy, not a contract check.

**Consumers will appear faster than the rule anticipates.** Today, the only consumer is `scripts/verify-decay.sh` (field whitelist) and the integration test. But plan 015 itself motivates CI coverage precisely because the contract matters operationally. BL-010 (daemon) and future synthesis tooling will add more consumers. Each new consumer raises the probability that a "no-bump" optional field addition silently breaks someone's logic. At n=2 consumers, the rule is fine. At n=5, a borderline field addition will generate a dispute about whether it required a bump.

**The bump-rule doc is human-enforced.** It lives in `docs/schemas/dreaming_pass.json` as a comment block. No tooling enforces it. The integration test asserts `schema_version == 1` but does not gate on whether a bump was warranted. The rule will be followed until someone in a hurry judges a field "additive" and a consumer breaks silently — which is exactly the failure mode the rule exists to prevent.

**Contrast with the other four decisions:**

- Exit 2 unconditional (Step 3): principled, addresses a real anti-pattern, recovery path documented. Low regret.
- `--db-path` pass-through (Step 4): mirrors an existing binary flag, straightforward. Low regret.
- Extend-vs-new-test-file (Step 5): heuristic, inconsequential to reverse. Low regret.
- In-place BL close (Step 6): explicitly accepted phantom-state window, cosmetic risk only. Low regret.

## What reversal looks like

Within 6 months: a new field is added to the `dreaming_pass` JSON (e.g., `cluster_ids` from synthesis work) and judged "optional, no bump needed." A consumer — possibly the BL-010 daemon or a new `verify-decay.sh` check — breaks because it conditionally branches on field presence. Post-incident, the team concludes the bump rules were underspecified for fields that are optional-but-semantically-significant. The rule gets tightened to "bump on any new field that a consumer might branch on" — which is closer to "bump always" and implicitly reverses the original intent.

## Caveat

This is a judgment call with low severity: the consequence of getting it wrong is a schema version bump at v2 earlier than expected, not data loss or a broken deploy. The other four decisions are stable. If forced to pick "no decision is obviously regret-prone," this would still be the weakest link — the bump rules' ambiguity at the margin is the only live risk in this plan.
