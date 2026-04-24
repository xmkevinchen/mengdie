---
round: 2
date: 2026-04-23
author: team-lead (host)
purpose: Index + orientation + TL decision for Topic 1
---

# Round 2 — TL Synthesis

## Per-agent files (read directly)

- `architect.md` — **shifted** Round 1 → Round 2: drop Option 3 (40% prevalence argument accepted), accept Option 5 (`KnowledgeType::Synthesized`) as root fix + co-land in v5 migration (5 code edits + 1 backfill UPDATE), drop Option 4 (5 supersedes). Final rec: **1 + 5**.
- `challenger.md` — **retracted** Round 1 Option 4 redundancy claim (verified `source_type` NOT printed in CLI; only `knowledge_type` is). Option 5 scope revised to 5 edits + backfill. Converges with codex on defer Option 3. Accepts either Option 4 or Option 5 for CLI rendering. Still-standing: if Option 3 ever ships, discriminate on `knowledge_type == "synthesized"`, not `source_type`.
- `codex-proxy.md` — **rejects Option 5** on axis discipline: `knowledge_type` is epistemic (factual/experiential/decisional), `source_type` is provenance (conclusion/review/plan/synthesis). Mixing "synthesized" into knowledge_type conflates axes. Final rec: **1 + 4 only**.

## Position matrix

| Option | Architect R2 | Challenger R2 | Codex R2 | TL decision |
|---|---|---|---|---|
| 1 audit | ship | ship | ship | **SHIP** |
| 2 LLM verify | defer | defer | defer | **DEFER** |
| 3 downrank | defer (recanted) | defer | defer | **DEFER** |
| 4 CLI provenance surface | supersede by 5 | accept | ship | **SHIP (as "surface source_type")** |
| 5 new enum variant | ship | accept | reject (axis) | **REJECT** (see rationale) |

## Mandatory Field 1 — Pruned

- **Pruned**: Option 3 (downrank) — unanimous defer after 40% prevalence + zero observed hallucinations evidence. Architect recanted Round 1 0.7 recommendation.
- **Pruned**: Option 4 as "hardcoded `[SYN]` title prefix" — all agents agreed title mutation is not the right UI. Option 4 stays in the ship list but reinterpreted as "surface the existing `source_type` field in CLI output" (verified at `dreaming.rs:564` that synthesis rows have `source_type = "synthesis"` already — CLI just needs to display it).
- **Pruned**: challenger's "Option 4 redundant" claim — explicitly retracted after verification that CLI prints `knowledge_type` only.
- **Retained split**: Option 5 accepted by architect + challenger vs rejected by codex on axis discipline.

## Mandatory Field 2 — Of-framing disposition

1. **Codex: axis discipline** — `knowledge_type` enum is epistemic, `source_type` enum is provenance; conflating loses category cleanliness. Disposition: **INTEGRATE** and **adopt** — this is the strongest semantic argument in Round 2.
2. **Architect: co-landing with v5 migration is pragmatic** — reclassify via the migration already shipping for dedup-key. Disposition: **REJECT-WITH-REASON** — pragmatism doesn't override axis integrity when the alternative (Option 4 = surface source_type) solves visibility without touching the migration.
3. **Challenger: structural argument — if Option 3 ever ships, discriminate on `knowledge_type == "synthesized"`** — this is the strongest argument FOR Option 5 and against my decision below. Disposition: **ACCEPT with defer path**: if/when Option 3 ships AND we want epistemic-level discrimination, revisit Option 5. For now, Option 4 is sufficient for visibility.

## Mandatory Field 3 — Verification artifact

- **Verified**: `source_type = "synthesis"` in synthesis rows at `src/core/dreaming.rs:564` and `src/core/db.rs:1064`. Option 4 as "surface source_type" requires only CLI formatter change, no new data.
- **Verified**: CLI currently prints `knowledge_type` but not `source_type` (challenger Round 2 code read). Provenance is genuinely invisible at the operator surface.
- **Verified**: 40% prevalence (27 synth / 68 total) — codex data point, accepted.
- **Unvalidated but downgraded**: "zero observed hallucinations" — challenger clarified it's operator spot-checks of 6-10 of 27 rows (not systematic), documented in BL-clustering-validation.md. Downgraded from "zero confirmed" to "no confirmed bad at sample size 10/27". Still sufficient signal to defer Option 2.

## Mandatory Field 4 — Frame-challenge self-check

- Round 1 challenger raised Option 5 as root fix. Round 2 engaged it directly. No silent drop.
- Round 1 codex raised 40% prevalence. Round 2 architect recanted Option 3. No silent drop.
- Round 2 codex added axis discipline (new in Round 2). TL integrates and adopts it.
- Round 2 challenger retracted Round 1 Option 4 redundancy claim. Recorded as falsification, not silent drop.

## TL Decision (Topic 1)

**Ship Option 1 + Option 4 (surface `source_type` field in CLI output). Defer Options 2 and 3. Reject Option 5.**

**Rationale**:

1. **Axis discipline (codex)** — `knowledge_type` is epistemic, `source_type` is provenance. Syntheses are factually-*shaped* content (they make factual claims) with synthesized *provenance* (LLM-generated from source memories). Tagging them "synthesized" in `knowledge_type` breaks the epistemic category tree — what happens to a synthesis that derives from three experiential sources? Forcing it to "synthesized" loses that it's a factual distillation. The right axis IS the provenance axis, which we already have.

2. **Option 4 reinterpreted as "surface source_type"** — not a hardcoded `[SYN]` title prefix (all 3 agents implicitly preferred not mutating titles). Implementation: CLI formatter prints source_type alongside title/snippet. `mengdie search` output gains a visible provenance line. No schema change, no enum change, no migration.

3. **Minimum-viable ship** — Options 1 + 4 solve:
   - Sub-problem A (provenance visibility): operator sees `source_type = synthesis` in search output.
   - Sub-problem B (fidelity audit path): Option 1's audit subcommand gives a read-only way to inspect synthesis vs source memories.
   Options 2 and 3 defer pending observed bad-rate data (codex's "2027 rule": ship verification/downrank only when audited failure > 1/20 OR syntheses dominate top-5 regularly).

4. **Architect's Option 5 dissent preserved** — if axis discipline loses to pragmatism in a future plan (e.g., if the team wants an epistemic-level search discriminator per challenger's remaining argument), revisit Option 5. Not now.

**Reversibility: HIGH**. Option 4 is a CLI formatter change (2-10 LOC). Option 5 can be added later if the axis-discipline call proves wrong. Option 3 can be added later once bad-rate data justifies a multiplier.

**Reversibility basis**: no data commits to the schema; no migration; no enum change. The decision lives entirely in `src/bin/cli.rs` formatting.
