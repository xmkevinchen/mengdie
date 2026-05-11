---
id: "016"
title: "Review: sqlite-vec compatibility verification spike (F-001 / BL-007)"
type: review
created: 2026-04-28
target: "docs/plans/018-sqlite-vec-compat-spike.md"
verdict: pass
---

# Review 016 — sqlite-vec compatibility verification spike

**Plan**: `docs/plans/018-sqlite-vec-compat-spike.md`
**Range**: `ba1c19c~1..HEAD` (plan 018 commits only — broader feature branch carries 11 prior unrelated commits)
**Spike outcome**: `docs/spikes/sqlite-vec-compat.md` — PASS_WITH_CONDITIONS (L2 default, override `distance_metric=cosine` verified working in =0.1.9)
**Verdict**: PASS — three P2 documentation gaps fixed inline; remaining items are BL-012 / BL-011 plan-time concerns.

## Team composition

| Agent | Angle | Backend | Outcome |
|---|---|---|---|
| architecture-reviewer | Decision capture, scope discipline, downstream BL hand-off | Claude (`ae:review:architecture-reviewer`) | PASS, 2 P3 findings |
| challenger | Pure opposition (claim/evidence/objection structured) | Claude (`ae:workflow:challenger`) | 6 challenges (2 HIGH, 4 MEDIUM) |
| codex-proxy | Production spike pattern + outcome durability | Codex MCP (medium reasoning) | No P1, 2 P2, 5 P3 findings |
| gemini-proxy | AC3 metric-identification rigor + falsifiability | **Gemini MCP quota exhausted** → TL fallback to oMLX `gemma-4-26b-a4b-it-4bit` | 6 findings (2 P2, 4 P3); analysis weaker than challenger's on disambiguation |

**Cross-family fallback note**: Gemini free-tier quota was exhausted at start of review; per CLAUDE.md cross-family fallback strategy, TL routed the Google-family lens to local oMLX `gemma-4-26b-a4b-it-4bit` directly via curl (preserving Google family vector). Gemma's findings were lower-rigor than challenger's on the L2 vs L2² question — challenger correctly identified that the orthogonal-pair evidence (row 5: `dist≈1.4142`) eliminates L2², a chain of reasoning gemma missed.

## Disagreement Value Assessment

| Topic | architecture | challenger | codex | gemma | TL synthesis |
|---|---|---|---|---|---|
| Smoke source preservation | (not raised) | C4 HIGH (chain-of-custody) | Q7 P2 (BL-011 reuse) | (not raised) | **Convergent P2** — fixed inline |
| L2 vs L2² disambiguation | (not raised) | C1 HIGH (prose gap; evidence supports L2 via row 5) | (not raised) | F3 P2 (claimed unresolvable) | **Challenger correct** — evidence supports L2; prose gap fixed inline. Gemma's "unresolvable" claim refuted by orthogonal-pair row. |
| Phantom RRF formula in caveat | F2 P3 | (not raised) | (not raised) | (not raised) | **Single-reviewer P3 → P2** — misleading wording could deceive BL-012 author; fixed inline. |
| Cosine vs dot-product-distance for unit vectors | (not raised) | C2 MEDIUM | (not raised) | F4 P2 | **Convergent but low practical risk** — fastembed normalizes; addressed by parenthetical in L2/L2² fix. |
| Version pin lockfile drift | (not raised) | C3 MEDIUM | Q5 P3 | (not raised) | **Convergent P3** — defer to BL-012 plan-time discipline. |
| Frontmatter durability (os version, AC keys) | (not raised) | (not raised) | Q3 P2 | (not raised) | **Single-reviewer P3** — defer to BL-012 plan-time. |
| Bones-pattern adapter detail | (not raised) | (not raised) | Q6 P3 | (not raised) | Defer to BL-012's plan author. |
| BL-002 stub body | F1 P3 | (not raised) | (not raised) | (not raised) | Defer to BL-002 promotion-to-plan time. |
| AC7 spirit / auto-pass gate self-waiver | (not raised) | C6 MEDIUM | (not raised) | (not raised) | **Procedural** — concerns AE plugin's `/ae:work` auto-pass-gate rules, not mengdie's spike. Out of mengdie's scope. |
| PASS_WITH_CONDITIONS urgency in caveat | (not raised) | C5 MEDIUM | (not raised) | (not raised) | Addressed inline by caveat-description rewrite ("MUST", "correctness bug, not a style choice"). |

## P1 / P2 findings

### P2-1: Phantom RRF formula `1 - distance/2` referenced in caveat description (architecture-reviewer F2)

- **Claim**: Caveat description named `1 - distance/2` as "Existing RRF score formula".
- **Evidence**: `src/core/search.rs:220,224` use pure rank-based RRF (`1.0 / (k + rank + 1.0)`); no raw-distance score conversion exists in `src/`. The formula was a phantom.
- **Risk if not fixed**: BL-012 plan author searches for `1 - distance/2` in src/ and finds nothing → confusion or worse, fabricates the formula.
- **Disposition**: **FIXED** in fixup commit (squashed into spike commit). Caveat now correctly states current code is unaffected; preserves the BL-012-time mitigation requirement with stronger language ("correctness bug, not a style choice").

### P2-2: L2 vs L2² disambiguation prose gap in Distance metric finding section (challenger C1 + gemma F3)

- **Claim**: Distance metric finding section claimed "L2 unambiguously" without citing the evidence chain that eliminates L2².
- **Evidence**: For the (A, B) primary probe pair with `dot=0.5`, both L2 and L2² return `1.0` — that observation alone cannot disambiguate. The disambiguation comes from the orthogonal-pair evidence (Evidence section row 5: `distance ≈ 1.4142135` for the `dot=0` pair; L2 = `sqrt(2)`, L2² = `2.0` → matches L2). The evidence is in the record but the prose did not link the chain.
- **Risk if not fixed**: A future reader reproducing the spike from this record might conclude the test was insufficient, when in fact the evidence is sufficient — only the prose was thin.
- **Disposition**: **FIXED** in fixup commit. Added a "Note on L2 vs L2² disambiguation" subsection citing the row-5 evidence explicitly. Also addresses the cosine-vs-dot-product-distance for unit vectors concern by stating the fastembed-normalization assumption.

### P2-3: Smoke binary source not preserved; record overstates preservation (challenger C4 + codex Q7)

- **Claim**: Outcome record stated "Source captured in plan body" but plan 018 body contains only step-level checklist + SQL snippets, not the actual Rust source.
- **Evidence**: `git log --all -- examples/sqlite_vec_smoke.rs` returns 0 commits. Plan 018:65-100 is checklist text, not source. The original misleading claim was a documentation defect.
- **Risk if not fixed**: BL-011 (Linux x86_64 verification) author following the outcome record's pointer would hit a dead end.
- **Disposition**: **FIXED** in fixup commit. Replaced false claim with honest acknowledgment + reconstruction pointer (plan 018 Steps 1-3 + Evidence-section SQL snippets) + explicit note that BL-011 may add its own preserved `examples/` source if stronger audit trail needed.

## P3 findings (deferred — not filed as new BLs)

These are all naturally addressed at downstream BL plan-time and do not warrant orphan backlog entries:

- **codex Q3**: Frontmatter `environment.os` is "macOS (operator's primary dev machine)" not concrete version; `decision_drivers` is free-text not AC-keyed. Defer: BL-012 plan-time editorial pass on the spike record, OR a global review template hardening (out of mengdie scope).
- **codex Q5**: Version-pin upgrade gating language mixes "require" / "should". Defer: BL-012 plan author tightens at adoption time.
- **codex Q6**: Bones-pattern adapter recommendation is too thin for direct adoption. Defer: BL-012 plan elaborates the adapter boundary contract.
- **architecture F1**: BL-002 (`docs/backlog/unscheduled/BL-002-...`) body is a 8-line stub. Defer: BL-002 promotion-to-plan author fleshes from spike-record context.
- **challenger C3**: Cargo.lock-drift risk on `=0.1.9` exact pin not enumerated. Defer: BL-012 plan-time mitigation.
- **challenger C6**: Auto-pass gate self-granted waiver of cross-family code review for "markdown-only spike". Out of scope — concerns AE plugin's `/ae:work` skill rules, not mengdie's spike. (Recommended: file as upstream AE plugin BL if the user wants procedural tightening; not filed here.)

## Outcome Statistics

- **Steps completed**: 5/5 + spike-merge step (all `- [x]`)
- **Rework rate**: 1/5 steps required fixup (spike commit ba1c19c → squashed into 11d6540 with 3 P2 doc fixes; bookkeeping commit 6851462 → bddbdb8 unchanged content)
- **P1 escape rate**: 0 — plan-review caught + addressed all 7 Must Fix before /ae:work; no P1 surfaced during /ae:review
- **P2 found in /ae:review**: 3 (all docs-only, fixed inline)
- **P3 found**: 6 (all deferred to BL-012 / BL-011 plan-time, no new BLs filed)
- **Drift events**: 0 (no contract violations during /ae:work)
- **Fix loop triggers**: 0 (no circuit breaker activations)
- **Auto-pass rate**: 1/1 (single-commit spike; auto-pass gate fired with `cross_family_degraded: false` self-waiver — see challenger C6 procedural concern)
- **Deferred resolution rate**: 0/0 (no DEFERRED items at /ae:work-start; none added during execution)

## Cross-family proxy degradation log

- **Codex**: OK (medium reasoning, returned 7 findings)
- **Gemini**: **DEGRADED** — free-tier quota exhausted; TL fallback to oMLX `gemma-4-26b-a4b-it-4bit` (Google family vector preserved). Gemma's analytical depth on the L2 vs L2² question was lower than challenger's; the convergent P2 finding stands but the second-source value-add was weaker than a healthy Gemini run would have provided.

## Fixups

| Original commit | Fixup target | Squashed into | Findings addressed |
|---|---|---|---|
| ba1c19c (spike outcome) | `--fixup=ba1c19c` | 11d6540 | P2-1 (caveat formula), P2-2 (L2 vs L2² prose), P2-3 (smoke source preservation) |

Single fixup commit `c6436fe` was created targeting `ba1c19c`, then squashed via `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash eb5080e`. Resulting `11d6540` retains AC7 invariant: 1 file changed, 188 lines added, only `docs/spikes/sqlite-vec-compat.md`.

## Knowledge Capture

`memory_search` MCP tool was not registered for this session, so no Mengdie ingest attempted. Patterns worth capturing on a future-session re-run:

- **Spike outcome record durability pattern**: when documenting a spike whose source binary is deleted, the outcome record's reconstruction pointer must be honest (cite specific plan-step SQL snippets rather than vague "source in plan body"). Phantom claims become audit dead-ends 6+ months later. Knowledge_type: experiential. Entities: `spike-record-durability`, `audit-trail-honesty`.
- **L2 vs L2² unit-vector disambiguation requires non-(A,B) evidence**: for any (A,B) pair with `dot=0.5`, both L2 and L2² return 1.0. A two-pair test design (probe + orthogonal pair) is the minimum to disambiguate. Knowledge_type: experiential. Entities: `vector-distance-metric-identification`, `spike-test-design`.
- **Phantom-formula bug class in spike records**: caveat descriptions referencing src/ formulas should be grep-verified against actual code before merge. Easy to write a plausible-sounding formula that doesn't exist. Knowledge_type: experiential. Entities: `caveat-description-fact-check`, `phantom-code-reference`.

## Verdict

**PASS** — three P2 documentation gaps fixed inline; remaining concerns are BL-012 / BL-011 plan-time editorial work that does not block the spike's primary deliverable.

The spike's primary value is intact:
- L2 default identified (correctly, on sound evidence chain after fix)
- `distance_metric=cosine` override verified functional in `=0.1.9`
- BL-002 trigger fired (operator may schedule when ready)
- BL-011 + BL-012 follow-up filings recommended with concrete scope
- AC1–AC7 all met; AC6 architectural firewall (no `src/` touched) holds

## Next steps

- Spike commit history: `eb5080e` (plan reviewed) → `11d6540` (squashed spike outcome with fixes) → `bddbdb8` (bookkeeping).
- File BL-011 (Linux x86_64 sqlite-vec smoke verification) when the operator is ready to run on Forgejo CI.
- File BL-012 (vector.rs sqlite-vec adoption with bones-pattern adapter + `distance_metric=cosine` override + score-conversion design) when adoption is scheduled.
- BL-002 (Reflection module consolidation) trigger has fired; operator may schedule when ready.
