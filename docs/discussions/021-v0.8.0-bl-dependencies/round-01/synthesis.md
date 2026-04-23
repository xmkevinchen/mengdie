---
round: 1
date: 2026-04-23
author: team-lead (host)
purpose: Index + orientation for Round 2 — NOT a substitute for peer files
---

# Round 1 — TL Synthesis

## Index of per-agent files

All agents read peer files DIRECTLY for Round 2 — synthesis is orientation only.

- `architect.md` — bundle-all, remove-both, actions 1-4, case-by-case + upstream AE feature request
- `archaeologist.md` — fact-checks: actions 1+3 already done, ops-doc edits `format_dreaming_line` not `format_structured_json`, triggers cold
- `challenger.md` — sharpest: action 5 = first-caller anti-pattern; missing topic: sprint gate text becomes incoherent after remove
- `minimal-change-engineer.md` — split 2+1 (ops-doc is separate S plan), do-nothing on defer-items, no policy
- `codex-proxy.md` — bundle-all by dependency edges, remove-both, actions 1-4, NEW admission policy NEEDED (`admission_status: defer-until-trigger` marker)
- `gemini-proxy.md` — scope/risks lens: rollback blast-radius of bundle, /ae:roadmap remove semantics unknown, --db-path test-isolation risk

## Position matrix (Round 1 only)

| Topic | architect | archaeologist | challenger | minimal-change | codex | gemini |
|-------|-----------|---------------|------------|----------------|-------|--------|
| 1: Bundle shape | all-3 | (evidence: smaller than framed) | split ops-doc | split 2+1 | all-3 sequenced | manageable; atomic-rollback concern |
| 2: Defer items | remove-both | (evidence: triggers cold) | 4th option: conditional-promote? | do nothing | remove-both | need roadmap-remove fact-check |
| 3: Action 5 | defer | (evidence: BL-010 absent, 1+3 done) | defer (first-caller) | defer | defer | defer |
| 4: Policy | case-by-case + upstream AE BL | — | ceremony; one-sentence checklist | none | `admission_status` marker NEEDED | case-by-case sufficient |

## Mandatory Field 1 — Pruned

- **Pruned**: "same cli.rs function coupling" argument for mandatory bundling (architect cited, archaeologist disproved — different functions 19 lines apart). Argument now restated as "same-file, adjacent functions" which is weaker.
- **Pruned**: "3 of 5 hardening actions need to be done" (archaeologist verified actions 1 + 3 already live at `scripts/verify-decay.sh:35-38, :47`). Effective work is actions 2 + 4 + decision-on-5.
- **Retained**: all 3 bundle-shape positions (bundle-all / split-ops-doc / split 2+1). Evidence reshuffles but doesn't eliminate.
- **Retained**: Topic 2 remove-vs-donothing — 4 agents lean remove, 1 (minimal-change) argues do-nothing. Needs Round 2 resolution + `/ae:roadmap remove` fact-check (gemini/minimal-change both raise it).
- **Retained**: Topic 4 with new constructive proposal — architect + codex converge on `admission_status: defer-until-trigger` frontmatter. Challenger/minimal-change/gemini's "no policy" may not conflict with this lightweight marker.

## Mandatory Field 2 — Of-framing disposition

Challenges raised that question the framing itself:

1. **Challenger: "Missing topic — sprint gate text becomes incoherent after remove"**. The v0.8.0 roadmap gate says "All 4 BL-decay-* closed" and "All 3 BL-synthesis-* closed" (roadmap.md:32-34). If Topic 2 removes 2 items, the gate count becomes 3 + 2 = 5 items, not 4 + 3 = 7. No topic currently addresses who updates gate text.
   - **TL disposition: INTEGRATE**. Add as a sub-question under Topic 2 for Round 2 — "if we remove, how does the gate text update?" Not a new topic; it's a direct consequence of Topic 2's resolution.

2. **Challenger: "Topic 4 is ceremony, not a real problem"**. Challenges the existence of Topic 4.
   - **TL disposition: LEAVE IN**. Codex's admission_status proposal shows there's a constructive answer beyond "policy document vs nothing". Round 2 will converge on smallest-useful marker or confirm no action.

3. **Architect: "BL-synthesis-provenance option commitment needs a mini-discuss before its plan"**.
   - **TL disposition: DEFER-TO-FOLLOWUP**. Out of framing (synthesis cluster is Out per framing.md:72). File as note in conclusion; not Round 2 material.

4. **Architect: "action 5 close-state — if BL closes with action 5 noted, where does it get re-recorded?"**.
   - **TL disposition: INTEGRATE**. Sub-question under Topic 3 for Round 2 — "is the action-5 re-file mechanism a note in the closed BL, a new BL, or a note on BL-010 when it opens?"

## Mandatory Field 3 — Verification artifact

- archaeologist verified 6 analysis.md claims with file:line evidence (analysis.md:49-57 hard dep, :61-65 conditional dep, etc.) → **verified**
- archaeologist disproved 3 claims (action 3 already done at `verify-decay.sh:47`, action 1 branch doesn't exist at :35-38, ops-doc edits `format_dreaming_line` at `cli.rs:226` not `format_structured_json`) → **disproved, analysis.md claim stale**
- No verification artifact yet for `/ae:roadmap remove` semantics — **unvalidated**, needed for Round 2 Topic 2.
- No verification artifact for plan 014 actual review-depth vs bundle-size correlation — **unvalidated**, challenger's claim. Challenger can cite plan 014 review.md in Round 2.

## Mandatory Field 4 — Frame-challenge disappearance self-check

Comparing Round 0 framing markers vs Round 1 content:

- Round 0 doodle-strategic: "commitment-semantics is higher-leverage than execution-shape" → **survived**, Topic 4 carried it; architect + codex advance a constructive proposal
- Round 0 doodle-adversarial: "BL-010 scope in/out distinction" → **survived**, reflected in framing.md:65-68 and actively discussed in Topic 3 action 5 analysis
- Round 0 codex: 5 bias-anchoring edits → **applied** before spawn; no residual bias detected in Round 1 agent framing language
- Round 0 minimal-change-engineer: timed out; coverage gap was flagged. Round 1 minimal-change-engineer (in-team agent) delivered the scope-creep discipline coverage — **gap closed in Round 1**

Zero silently-dropped frame challenges detected.

## Round 2 priorities (what TL will direct)

1. **Topic 1 — Resolve bundle shape among 3 positions.** Agents must engage archaeologist's factual correction (different functions, smaller effective BL size). Does the reshaped evidence change preferred shape?
2. **Topic 2 — Fact-check `/ae:roadmap remove`.** Archaeologist (or codex) verifies what the command actually does to the BL file. Then resolve remove vs do-nothing.
3. **Topic 2 sub-question — Gate text update.** If remove, what becomes of the sprint-gate text?
4. **Topic 3 — Confirm actions 1+3 done status.** Short confirmation; then converge on "ship actions 2+4, defer 5, file BL-decay-threshold-mode with BL-010 trigger".
5. **Topic 4 — Converge on `admission_status` marker.** Is this the "no ceremony + useful signal" middle ground? Who writes it (mengdie-local CLAUDE.md vs upstream AE BL)?
