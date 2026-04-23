---
round: 2
date: 2026-04-23
author: team-lead (host)
purpose: Index + orientation — NOT a substitute for peer files
---

# Round 2 — TL Synthesis

## Index of per-agent files

- `architect.md` — **shifted** Topic 1 (bundle-all → split 2+1); Topic 2 remove-both with manual gate update; Topic 3 actions 2+4; Topic 4 upstream AE BL
- `archaeologist.md` — 3 new facts: roadmap-remove is non-destructive, close uses warn-by-default, gate count becomes incoherent; **self-correction**: action 1 IS real remaining work (JSON-parse fallback branch at verify-decay.sh:64-73)
- `challenger.md` — new blind spot: `/ae:roadmap remove` atomicity risk (two sequential commands); **shifted** Topic 1 to M+M + separate S; new intra-plan constraint: action 2 must precede action 4; Topic 4 medium-ceremony without AE tooling
- `minimal-change-engineer.md` — split 2+1 strengthened; **conceded** Topic 2 remove-both; proposes updating BL in-place (mark 3 done); rejects `admission_status` marker, proposes one-line checklist
- `codex-proxy.md` — **revised** to 2+1 split based on new evidence; Plan A (json-schema + verify-decay, ~100 LOC), Plan B (ops-doc, ~20-30 LOC); marker = 70-120 LOC upstream AE change, amortizes ~4-6 sprints/year
- `gemini-proxy.md` — bundling risk mitigated by smaller scope; gate-text breakage confirmed operational risk; marker: prospective-only adoption to avoid retroactive cost; test isolation on --db-path likely safe with temp paths

## Position matrix (Round 2)

| Topic | architect | archaeologist | challenger | minimal-change | codex | gemini | Convergence |
|-------|-----------|---------------|------------|----------------|-------|--------|-------------|
| 1: Bundle shape | split 2+1 | (facts support 2+1) | split 2+1 | split 2+1 | split 2+1 | split OK | **UNANIMOUS** |
| 2: Defer items | remove both + gate update | (remove is non-destructive) | remove + atomicity note | remove both (conceded) | remove both | remove; gate text break is real | **UNANIMOUS** |
| 3: Action 5 | defer; ship 1+2+4 | (1 IS real work, 3 done) | defer; action 2 before 4 | defer; BL update in-place | defer; re-file | defer | **UNANIMOUS on defer-5** |
| 4: Policy | upstream AE BL for marker | — | upstream AE BL; don't apply now | no marker; one-line checklist | implementable upstream | prospective-only marker | **5-1**: file upstream BL |

## Mandatory Field 1 — Pruned

- **Pruned**: "bundle all 3" (Topic 1) — architect's original position, reversed in Round 2 on evidence. Codex's original position also reversed. Now zero agents hold bundle-all.
- **Pruned**: "do nothing on defer items" (Topic 2) — minimal-change-engineer conceded explicitly. Now zero agents hold do-nothing.
- **Pruned**: "action 3 is remaining work" (Topic 3) — archaeologist verified already done at verify-decay.sh:47.
- **Promoted**: "action 1 IS remaining work" (Topic 3) — archaeologist's Round 1 assessment was wrong; the "proceeding anyway" branch is the JSON-parse fallback at :64-73, not the binary preflight.
- **Retained**: minimal-change-engineer's dissent on Topic 4 marker. Fair dissent, scored explicitly.

## Mandatory Field 2 — Of-framing disposition

Challenges to the framing raised this round:

1. **Challenger: `/ae:roadmap remove` atomicity blind spot** — if we issue two remove commands sequentially and one fails, sprint wedges.
   - **TL disposition: INTEGRATE as sub-question under Topic 2**. Archaeologist's Round 2 finding (warn-by-default close, non-destructive remove) mitigates some of the risk, but the atomicity concern is valid for the commit sequence. Answer: do both removes + gate update in one commit; on failure, revert the commit.

2. **Challenger: intra-plan sequencing (action 2 before action 4) — Topic 3 new constraint**.
   - **TL disposition: INTEGRATE**. Carried into the Topic 3 decision — plan A step order locks this.

3. **Codex: admission policy amortizes over 4-6 sprints/year** — reframes minimal-change's "n=2 is premature" argument.
   - **TL disposition: REJECT-WITH-REASON**. Valid statistical argument, but mengdie is solo-dev and the plugin work is upstream (AE project owns the maintenance). For *mengdie*, the applicable dissent stands: no local YAML field, just file the BL.

4. **Gemini: prospective-only adoption** — compromise on marker.
   - **TL disposition: INTEGRATE**. Upstream AE BL specifies prospective-only scope; no retroactive marking on existing mengdie BLs.

No silently-dropped frame challenges detected.

## Mandatory Field 3 — Verification artifact

- `/ae:roadmap remove` behavior → **verified** by archaeologist reading SKILL.md. Non-destructive; moves to `unscheduled/`; appends Notes entry.
- `/ae:roadmap close` behavior → **verified** by archaeologist; warn-by-default on open items, `--strict` flag to refuse.
- Gate text incoherence → **verified** by archaeologist reading `.ae/roadmaps/v0.8.0.md`; gate uses prefix counts ("All 4 BL-decay-*" / "All 3 BL-synthesis-*") that become wrong after remove.
- Action 1 JSON-parse fallback branch → **verified** at `scripts/verify-decay.sh:64-73` (archaeologist Round 2 self-correction).
- Action 3 already implemented → **verified** at `scripts/verify-decay.sh:47`.
- Git history for action 1 branch → **verified** by archaeologist checking commits fd910e3 and e882be9.
- Plan 014 review evidence → **cited** by challenger (round-02/challenger.md references plan 013 reviews for bundled-plan AC discovery).

No unvalidated claims remain that block a decision.

## Mandatory Field 4 — Frame-challenge disappearance self-check

- Round 0 doodle-strategic "commitment-semantics higher-leverage" → survives into Topic 4 score; addressed
- Round 0 doodle-adversarial "BL-010 scope in/out" → survives into Topic 3 action 5 defer + re-file plan
- Round 1 challenger "missing sprint-gate topic" → integrated as Topic 2 sub-question; addressed in Round 2
- Round 1 architect "BL-synthesis-provenance needs mini-discuss" → explicitly out of framing; recorded for conclusion Next-Steps
- Round 2 challenger "remove atomicity" → integrated as Topic 2 execution detail (one commit)
- Round 2 minimal-change "marker = premature abstraction" → NOT silently dropped; preserved as explicit dissent in Topic 4 score

Zero silent drops detected.

## Consensus Verification Plan

- Topics 1, 2, 3 → **converged on evidence** (archaeologist's facts drove the shift), not groupthink. Evidence from different angles. **SKIP forced FOR/AGAINST verification** — per skill: "all agents independently reached the same conclusion with strong evidence from different angles".
- Topic 4 → has genuine in-team dissent (minimal-change) that already stress-tested the majority. **SKIP forced verification** — the dissent IS the verification. Score with dissent noted.

## Round 2 Outcome

Ready to score. All 4 topics have converged or have scorable dissent. No Round 3 needed.
