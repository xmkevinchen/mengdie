---
id: "021"
title: "v0.8.0 remaining BLs — execution shape — Conclusion"
concluded: 2026-04-23
plan: ""
entities: [v0.8.0, sprint, bundle-boundary, decay-operator-surface, json-schema-contract, verify-decay-hardening, ops-doc-polish, defer-trigger, roadmap-remove, gate-text, admission-status, hardening-action-5, threshold-mode, bl-010, sprint-commitment-policy]
---

# v0.8.0 remaining BLs — execution shape — Conclusion

Discussion 021 ran 2 rounds with 6 agents (architect, archaeologist, challenger, minimal-change-engineer, codex-proxy, gemini-proxy) + TL. All 4 topics converged without escalation to user. Decisions are evidence-driven — archaeologist's file:line verifications shifted architect's and codex's positions between Round 1 and Round 2.

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Bundle boundary for the decay-cluster plan | **Split 2+1** — Plan A bundles `BL-decay-json-schema-contract` + `BL-verify-decay-script-hardening` (M+M, ~100 LOC, hard-dep interior); Plan B is `BL-decay-ops-doc-polish` alone (S, ~20-30 LOC, soft-dep) | Unanimous. Archaeologist verified ops-doc-polish edits `format_dreaming_line` (cli.rs:226), NOT `format_structured_json` (cli.rs:207) — architect's "same function" bundling argument refuted. Hard dep (schema-contract → verify-decay test on `schema_version`) stays atomic in Plan A. Ops-doc coupling is docs-level (rollback references `breaches[]`). Split preserves 4-agent review depth; challenger cited plan 013 review evidence for bundled-plan AC discovery on M+M bundles | high |
| 2 | Fate of 2 defer-until-trigger items | `/ae:roadmap remove` both `BL-decay-dreaming-pass-optim` and `BL-synthesis-preload-db-miss-edge`. Both removes + manual gate-text edit in ONE commit. `--reason` cites trigger-not-fired | Unanimous (minimal-change conceded). Archaeologist verified /ae:roadmap remove is non-destructive (moves BL to `unscheduled/`, preserves body, appends `descope` Notes entry). Gate text prefix counts (`All 4 BL-decay-* / All 3 BL-synthesis-*`) become factually wrong after remove — must update in same commit. `/ae:roadmap close` uses warn-by-default not refuse, so do-nothing was tooling-compatible but put items in wrong archive tier | high |
| 3 | Which hardening actions ship in v0.8.0 | Ship actions **1 + 2 + 4** in Plan A. Action 3 already implemented at `scripts/verify-decay.sh:47` — mark done in BL, no plan work. Action 5 (threshold-mode) defers; close BL on Plan A merge, re-file as `BL-decay-threshold-mode` (trigger: BL-010 daemon plan approved). Plan step order: action 2 (`--db-path`) before action 4 (CI test) | Unanimous on action 5 defer (challenger: first-caller anti-pattern, BL-010 has no design). Archaeologist Round 2 self-correction: action 1 is real remaining work (JSON-parse fallback branch at verify-decay.sh:64-73, present in both commits fd910e3 and e882be9). Challenger added intra-plan sequencing (CI test invokes `mengdie --db-path` to isolate from operator's real DB) | medium |
| 4 | Sprint-commitment policy | File upstream AE BL proposing `admission_status: defer-until-trigger` frontmatter + `/ae:roadmap plan` scan-filter (prospective-only). Add one-line checklist to mengdie CLAUDE.md Review Rules. Do NOT retroactively mark existing mengdie BLs | 5-1 converge (minimal-change-engineer preserved dissent: "marker is premature abstraction over n=2; checklist line alone is sufficient"). Codex estimated 70-120 LOC AE skill change; gemini scoped prospective-only to avoid retroactive-marking cost. Compromise honors dissent: upstream BL is zero-cost-to-mengdie (AE owns followthrough); mengdie action is just the checklist line | high |

## Doodlestein Review

Three post-conclusion reviewers ran against this document. Findings:

- **doodlestein-strategic — smartest improvement**: Next Steps left the synthesis cluster path open-ended ("needs its own mini-discuss on provenance options first" — trailing sentence, no owner, no completion condition). Same phantom-active failure mode this discussion caught for defer-until-trigger items. **ACTED ON** — added explicit step 7 to Next Steps defining the v0.8.0 sprint close criterion + synthesis-cluster mini-discuss owner. See `round-doodlestein/strategic.md`.
- **doodlestein-adversarial — first failure in real use**: Next Step 2's "atomic one-commit" requirement is unenforceable at the gate-text edit — it's a manual prose edit with no tooling guard, and the exact before/after strings are not spelled out, so an executor reading this in 3 months has to re-derive from the decision table. **ACTED ON** — spelled out the exact before/after text in Next Step 2. See `round-doodlestein/adversarial.md`.
- **doodlestein-regret — likeliest reversal in 6 months**: Decision 4 (upstream AE BL) — its success depends on the AE project planning an unscheduled BL, which the team cannot control; minimal-change's preserved dissent could prove correct if the upstream BL sits 2-3 sprints without attention. **NO ACTION** — the decision's design already hedges this: the mengdie-local action IS only the checklist line (minimal-change's preferred solution). If the upstream BL never ships, mengdie loses nothing; the checklist was always the load-bearing piece. Recording as acknowledged observation. See `round-doodlestein/regret.md`.

No Topic decisions were reopened. Two document-quality improvements were applied inline. One regret observation was noted but accepted as already-hedged.

## Spawned Discussions

None. All 4 topics resolved in-team without spawning sub-discussions.

## Deferred Resolutions

None. Zero items entered the Sweep — all 4 topics converged by end of Round 2.

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude Opus 4.7 | Start |
| architect | solution design, bundle shape, plan-size trade-offs | Claude (Opus per agent card) | Start |
| archaeologist | code-surface verification, factual backbone | Claude (Sonnet per agent card) | Start |
| challenger | assumptions / frame-challenge / blind spots | Claude (Sonnet per agent card) | Start |
| engineering-minimal-change-engineer | scope-creep discipline, minimum-viable shape | Claude (project agent) | Start |
| codex-proxy | Plan-quality angle (step decomposition, AC verifiability) | OpenAI Codex (high reasoning) | Start |
| gemini-proxy | Scope & risks angle (hidden deps, edge cases, integration risks) | Google Gemini | Start |

## Process Metadata

- Discussion rounds: 2 (Round 1 independent research → Round 2 share & explore)
- Topics: 4 total (4 converged, 0 spawned, 0 deferred, 0 explained)
- Autonomous TL decisions: 4
- User escalations: 0
- Framing reviews (Round 0): 1 (v1 REVISE applied inline + overridden; v2 not re-run per auto-mode + user signal)
- Consensus verification runs: 0 (evidence-driven convergence; not groupthink — skipped per protocol)
- Doodlestein post-conclusion challenges: pending

## Notable shifts across rounds

- **architect** Topic 1: Round 1 "bundle all 3" → Round 2 "split 2+1" (archaeologist's factual correction)
- **codex-proxy** Topic 1: Round 1 "bundle all 3, sequenced" → Round 2 "split 2+1" (same evidence)
- **minimal-change-engineer** Topic 2: Round 1 "do nothing" → Round 2 "remove both" (archaeologist's /ae:roadmap remove non-destructive verification)
- **archaeologist** Topic 3: Round 1 "action 1 appears done" → Round 2 self-correction "action 1 is real remaining work" (re-read of lines 64-73)

Three of these shifts are on evidence, not social pressure — the convergence is genuine. Verified via the Round 2 synthesis frame-challenge self-check.

## Out-of-framing items recorded for follow-up

1. **BL-synthesis-provenance option commitment** — analysis.md:61-65 flagged 4 unresolved options. This discussion explicitly scoped synthesis cluster as Out (framing.md:70-72). A separate mini-discuss or a `/ae:discuss` invocation on just provenance options is needed before `/ae:plan` for the synthesis cluster.

2. **ops-doc-polish arrow/ASCII fallback choice** — architect flagged (architect.md:181-184) that ops-doc's item "commit to one or emit both" is undecided. This is a plan-time decision inside Plan B, not discussion-time.

## Next Steps

1. `/ae:plan` on Plan A: "decay operator surface hardening" — `BL-decay-json-schema-contract` + `BL-verify-decay-script-hardening` (actions 1, 2, 4). Plan step order locks action 2 before action 4.
2. One commit containing: both `/ae:roadmap remove` invocations + `.ae/roadmaps/v0.8.0.md` gate-text update. **Exact gate-text edits** (per doodlestein-adversarial — spell them out, the manual edit has no tooling guard):
   - Line 32: `- All 4 BL-decay-* items closed` → `- All 3 BL-decay-* items closed (BL-decay-dreaming-pass-optim removed per discussion 021 — trigger not fired)`
   - Line 33: `- All 3 BL-synthesis-* items closed` → `- All 2 BL-synthesis-* items closed (BL-synthesis-preload-db-miss-edge removed per discussion 021 — trigger not fired)`
   - Atomicity: if either `/ae:roadmap remove` fails, `git reset HEAD` on the work-in-progress edit and revert any successful remove before re-staging. Do NOT land partial state.
3. File upstream AE BL: `../agentic-engineering/.ae/backlog/unscheduled/BL-admission-status-defer-until-trigger.md`.
4. Edit `CLAUDE.md` Review Rules section to add the one-line checklist.
5. On Plan A merge: update BL-verify-decay-script-hardening body to checkmark actions 1, 2, 4 (action 3 already done); close BL; file `BL-decay-threshold-mode` in same commit.
6. Plan B (ops-doc-polish) can trail Plan A — lower urgency, pure-docs scope.

7. **Before planning the synthesis cluster**: run a mini-`/ae:discuss` scoped to the 4 provenance fix options listed in analysis.md:61-65 (the BL body doesn't commit to one). **v0.8.0 sprint close criterion** (per doodlestein-strategic — eliminate the phantom-active gap): sprint closes when Plan A merges + roadmap-removes commit lands + synthesis-cluster plans complete OR are explicitly moved to v0.9.0 via `/ae:roadmap move` with matching gate-text update. Without this criterion, the synthesis cluster replays the same defer-items ambiguity this discussion just resolved.

After these, the v0.8.0 sprint is down to the 2-item synthesis cluster (`BL-synthesis-dedup-key`, `BL-synthesis-provenance`; `BL-synthesis-preload-db-miss-edge` is removed per Topic 2) for the next planning cycle.
