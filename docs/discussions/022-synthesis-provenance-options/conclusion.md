---
id: "022"
title: "Synthesis provenance — fix option selection — Conclusion"
concluded: 2026-04-23
plan: ""
entities: [synthesis, provenance, knowledge-type-enum, source-type-field, cli-output, audit-subcommand, bl-synthesis-provenance, bl-synthesis-dedup-key, fidelity-detection, ranking-downrank]
---

# Synthesis Provenance — Fix Option Selection — Conclusion

Mini-discussion per discussion 021 Next Step 7. Ran 2 rounds with 3 agents (architect, challenger, codex-proxy) + TL. Round 0 framing review skipped per proportionality (BL body enumerates 4 clear options; decision shape is pick-from-list). Consensus verification skipped — convergence was evidence-driven (40% prevalence data + source_type code check + axis-discipline argument), not groupthink.

## Decision Summary (Converged)

| # | Topic | Decision | Rationale | Reversibility |
|---|-------|----------|-----------|---------------|
| 1 | Which fix option(s) ship in the v0.8.0 synthesis plan | **Ship Option 1 (audit subcommand) + Option 4 reinterpreted as "surface source_type in CLI output"**. **Defer** Option 2 (LLM verification) and Option 3 (downrank). **Reject** Option 5 (new `KnowledgeType::Synthesized` enum variant) on axis-discipline grounds. Architect dissent preserved. | Codex's axis-discipline argument decided Option 5: `knowledge_type` is epistemic (factual/experiential/decisional), `source_type` is provenance (conclusion/review/plan/synthesis); mixing "synthesized" into knowledge_type conflates axes. Option 4 reinterpreted respects both axes — synthesis rows already have `source_type = "synthesis"` stored (verified at `dreaming.rs:564`, `db.rs:1064`); CLI formatter just needs to render it. Options 2 and 3 defer per codex's data-gating rule (ship when audited failure > 1/20 OR syntheses dominate top-5). At 40% corpus prevalence (27/68 rows) with no confirmed hallucinations at sample size 10/27, both would change behavior blindly against a clean corpus. | high |

## Doodlestein Review

Skipped for this mini-discussion per proportionality. The decision scope is narrow (pick from enumerated options), reversibility is high (CLI formatter change + new subcommand file, no schema/enum change, no migration), and 3-agent review + 2 rounds provided adequate stress-testing. Doodlestein will run at `/ae:plan` time for the synthesis-cluster plan that follows.

## Spawned Discussions

None.

## Deferred Resolutions

None — zero deferred items survived Round 2.

## Team Composition

| Agent | Role | Backend | Joined |
|-------|------|---------|--------|
| host | TL (moderator) | Claude Opus 4.7 | Start |
| architect | option analysis, orthogonality matrix, migration dep | Claude (Opus per agent card) | Start |
| challenger | assumption attack, Option 5 root-fix proposal | Claude (Sonnet per agent card) | Start |
| codex-proxy | cross-family: empirical grounding, prevalence data, axis discipline | OpenAI Codex (high reasoning) | Start |

## Process Metadata

- Discussion rounds: 2 (Round 1 independent + Round 2 share-and-converge)
- Topics: 1 total (1 converged, 0 spawned, 0 deferred)
- Autonomous TL decisions: 1
- User escalations: 0
- Framing reviews (Round 0): 0 (skipped per proportionality)
- Consensus verification runs: 0 (evidence-driven convergence; not groupthink)

## Notable shifts across rounds

- **architect** Round 1 → Round 2: recommended 1+3+4 → **recanted Option 3** after codex's 40% prevalence data; **accepted Option 5** as root fix (later overruled by TL on axis discipline). Net shift: 1+3+4 → 1+5.
- **challenger** Round 1 → Round 2: **retracted** Option 4 redundancy claim after verifying `source_type` is NOT printed in CLI (only `knowledge_type` is). Revised Option 5 scope estimate from 2 lines to 5 edits + backfill UPDATE.
- **codex-proxy** Round 1 → Round 2: held position (1+4), **added axis-discipline argument** against Option 5 in Round 2 — this argument decided the Option 5 question.
- **TL** (host): converged on 1+4 (codex's direction) with Option 4 reinterpreted as "surface source_type" rather than the BL's literal "hardcode `[SYN]` prefix on title" — all 3 agents implicitly preferred no title mutation.

Three of these shifts are on evidence (40% prevalence, code verification, axis discipline) — genuine convergence, not social pressure.

## Dissent Preserved

**Architect dissent on Option 5 rejection**: architect argued co-landing `KnowledgeType::Synthesized` with BL-synthesis-dedup-key's v5 migration is cleaner engineering than adding CLI formatter logic. TL overruled on axis-discipline grounds. Forward trigger: if a future plan needs epistemic-level search discriminator AND the axis-dilution cost proves less than the co-landing pragmatism benefit, revisit Option 5.

**Challenger residual argument**: if Option 3 (downrank) ever ships, discriminate on `knowledge_type == "synthesized"` (epistemic status), not `source_type == "synthesis"` (provenance chain). This argument lives with Option 5's future-revisit trigger — if Option 3 ships AND we want epistemic-level discrimination, Option 5 lands with it.

## Out-of-framing items recorded for follow-up

1. **Sample-size discipline for "zero hallucinations" claim**: operator spot-checks of 10/27 is documented evidence but not systematic. If Option 1 (audit subcommand) lands, use it to grow the sample to 100% and record the rate. Not a plan-time blocker, but a metric worth collecting for future Option 2/3 ship-gate decisions.

2. **BL-synthesis-provenance body update**: the BL enumerates 4 options. This conclusion's Option 4 reinterpretation ("surface source_type" vs "hardcode `[SYN]` prefix") is a meaningful semantic change. The v0.8.0 synthesis plan should note the reinterpretation in its scope + update the BL body at close time.

## Next Steps

1. `/ae:discuss` remains needed for the **full** v0.8.0 synthesis cluster plan scope question? NO — the 2 open BLs (`BL-synthesis-dedup-key` + `BL-synthesis-provenance`) can now plan directly. Dedup-key is already unambiguous (replace `content_hash` as dedup key via v5 migration); provenance is resolved here.
2. `/ae:plan` on the v0.8.0 synthesis cluster — scope = BL-synthesis-dedup-key (v5 migration + dedup key replacement) + BL-synthesis-provenance Options 1 + 4. Bundle boundary TBD at plan-time (similar "2+1" vs "both together" question as discussion 021 Topic 1 faced for the decay cluster). Given both BLs are synthesis-subsystem changes and dedup-key's v5 migration is the heaviest item, bundling into one plan is likely correct.
3. After v0.8.0 synthesis cluster ships: v0.8.0 complete. Move to v0.9.0 per the roadmap theme (BL-009 MCP Dream Tool).
