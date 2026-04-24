---
round: 1
date: 2026-04-23
author: team-lead (host)
purpose: Index + orientation for Round 2 — NOT a substitute for peer files
---

# Round 1 — TL Synthesis

## Per-agent files (read directly, not via synthesis)

- `architect.md` — ship 1+3+4 defer 2; multiplier 0.7 not 0.5; no schema dep
- `challenger.md` — **Option 5 (new `KnowledgeType::Synthesized` enum variant)** is the root fix; all 4 options paper over enum gap. Zero observed hallucinations in 27 real syntheses (13+14). Option 2 same-family has no independence. Option 3's 0.5 multiplier unjustified. Option 4 redundant with `source_type` field. Option 1 is display, not audit.
- `codex-proxy.md` — ship 1+4 defer 2+3. **40% prevalence (27 synth / 68 total)** makes any downrank loud, not a nudge. Audit threshold ~30 ok, ~50 fuzzy, ~100 painful.

## Position matrix

| Option | Architect | Challenger | Codex | Leaning |
|---|---|---|---|---|
| 1 audit | ship | ship (with naming concern) | ship | **UNANIMOUS ship** |
| 2 LLM verify | defer | defer (same-family concern) | defer | **UNANIMOUS defer** |
| 3 downrank | ship @ 0.7 | no multiplier justified | no — 40% prevalence loud | **2-1 defer** |
| 4 CLI prefix | ship | redundant | ship | **2-1 ship** |
| 5 NEW enum variant | not addressed | root fix (+v5 co-land) | not addressed | **untested by most** |

## Mandatory Field 1 — Pruned

- **Pruned**: Option 2 (LLM verification) unanimously deferred. No further discussion needed.
- **Pruned**: Option 3's specific multiplier value (0.5 vs 0.7) — codex's 40% prevalence data makes the discussion shift from "pick number" to "ship with no data vs defer".
- **Retained (split)**: Option 3 ship-vs-defer decision (2 defer, 1 ship@0.7).
- **Retained (split)**: Option 4 ship-vs-redundant (2 ship, 1 redundant-to-source_type).
- **Surfaced new**: Option 5 (enum variant) — challenger-only; needs architect + codex perspective.

## Mandatory Field 2 — Of-framing disposition

1. **Challenger: Option 5 missing from BL enumeration**. The BL listed 4 options; challenger adds a 5th (new `KnowledgeType::Synthesized` enum variant — currently syntheses are misclassified as `Factual` because the variant doesn't exist).
   - **TL disposition: INTEGRATE**. Add to Round 2 prompt for architect + codex to evaluate against their option sets. This is potentially the minimum-viable root fix.

2. **Challenger: zero observed hallucinations — solving anticipated not observed problem**. 27 real-run syntheses (13 + 14) have been spot-checked clean.
   - **TL disposition: INTEGRATE**. Reframes "how much fix is enough" — if observed bad-rate is zero, the minimum-viable fix (Option 5 alone, or Options 1+4+5) may be sufficient; Options 2 and 3 are over-engineering.

3. **Codex: 40% prevalence changes Option 3's math**. A ×0.5 multiplier on 40% of search results is not a nudge, it's a systemic search-behavior change.
   - **TL disposition: INTEGRATE**. Architect's 0.7 was a guess without the prevalence data; codex's number shifts weight heavily toward defer.

## Mandatory Field 3 — Verification artifact

- **Verified**: synth prevalence 27/68 ≈ 40% (codex cross-family review). Should double-check against actual DB state in Round 2.
- **Verified**: no `KnowledgeType::Synthesized` variant exists (challenger — would need to spot-check `src/core/parser.rs` or wherever the enum is defined).
- **Unvalidated**: actual hallucination rate from 27 real syntheses — challenger claims "all clean" but this is operator judgment, not a systematic audit. Round 2 may ask for the source of this claim.
- **Unvalidated**: `get_synthesis_sources` DB helper existence for Option 1 (architect claims "needs one new DB helper").

## Mandatory Field 4 — Frame-challenge self-check

- No frame challenges silently dropped between Round 0 (skipped) and Round 1. All 3 agents engaged the BL's 4-option framing directly; challenger added Option 5 as extension, not replacement.

## Round 2 priorities

1. Ask architect + codex to evaluate Option 5 (enum variant). Is it the right root fix, a complement, or an unnecessary detour?
2. Resolve Option 3: given 40% prevalence, is downrank defensible at any multiplier without data?
3. Resolve Option 4: is the CLI prefix genuinely additive (Architect + codex view) or redundant-with-existing-field (challenger view)? What does current `mengdie search` output actually show when a synthesis row matches?
4. Confirm minimum-viable combination. Leading candidate: Option 1 + Option 4 + Option 5 (display + label + correct classification) with Options 2+3 deferred pending observed bad-rate data.
