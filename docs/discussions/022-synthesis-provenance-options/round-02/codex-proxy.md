---
agent: codex-proxy
round: 2
created: 2026-04-23
---

# Round 2: Option 5 Evaluation + Final Recommendation

## Summary

**Final v0.8.0 recommendation: Ship Option 1 + Option 4 only. Defer all others.**

Option 5 (new `KnowledgeType::Synthesized` variant) is not the root fix — the root is already fixed in the model (separate `source_type` field exists). Option 5 confuses the axes further.

## Option 5 Detailed Evaluation

### Does it address the BL problem?

**No.** The BL's two sub-problems are:
1. **Provenance visibility**: operator can't distinguish syntheses from primaries in CLI output
2. **Fidelity detection**: no way to tell which syntheses are accurate without reading sources

The code already handles provenance correctly:
- `SourceType::Synthesis` exists (added in BL-007) and distinguishes origin
- `SearchResultItem.source_type` field already carries this signal to machine consumers
- The gap is **CLI display**: `mengdie search` shows `knowledge_type` but not `source_type`

Adding `KnowledgeType::Synthesized` does not fix this gap. It relabels the epistemic class of syntheses but leaves `source_type` invisible in CLI output.

### Cost analysis

Challenger claims "~2 lines" but cost is actually:
- Add enum variant + `Display` branch: ~5 lines
- Update synthesis writer in `dreaming.rs`: ~2 lines
- Update serde validation in `mcp_tools.rs`: ~5 lines
- **Backfill existing 27-28 synthesis rows**: yes, data migration needed (UPDATE query)

Not free. More importantly, it doesn't solve the visibility problem.

### Model pollution

`knowledge_type` is used for behavior (contradiction detection, search filters, insertion rules). `source_type` is the provenance field. Collapsing them:
- Breaks query semantics (searching for "factual knowledge" would now exclude syntheses)
- Confuses downstream logic (contradiction rules could become contradictory)
- Doesn't actually surface provenance to the operator (still need to display the field separately)

**Verdict**: Option 5 is a model change that creates semantic debt, not a fix.

---

## Redundancy check: Option 5 vs. Option 4

If Option 5 shipped, would Option 4 become unnecessary?

**No.** Option 4 (`[SYN]` prefix in titles) is a display choice, not a semantic fix. If you want to display provenance via `source_type` instead of a prefix, that is a *variation* of Option 4, not a replacement. The core problem (operator can't see origin in CLI) persists with either approach — it's just "show the field" vs. "put a prefix."

The right display enhancement is: show `source_type` column in `mengdie search` / `mengdie list` output, not mutate the title.

---

## Schema migration coupling analysis

**Challenger claimed**: Option 5 can co-land in BL-synthesis-dedup-key's v5 migration.

**Reality**: BL-synthesis-dedup-key adds `synthesis_cluster_hash TEXT` column + index. Option 5 needs:
- No column addition (knowledge_type is already TEXT)
- Enum change + code changes
- Backfill of existing rows with an UPDATE statement

They can co-exist in the same plan, but Option 5 has its own data-migration cost. No automatic coupling.

---

## Minimum-viable v0.8.0 final ranking

**Candidates** (cost analysis):
- `1+4` (audit + prefix): ~100 lines of code, zero schema change, zero arbitrary numbers, solves visibility problem
- `1+5` (audit + enum): ~50 lines model change + backfill, solves nothing that 1+4 doesn't, adds debt
- `1+3+4` (audit + downrank + prefix): same as 1+4 plus arbitrary 0.7 multiplier and changed search behavior
- `1+4+5`: redundant; adds 5's cost without benefit
- `5-alone`: doesn't solve visibility problem; wrong axis

**Winner: `1+4`**

Why:
1. **Solves the actual operator problem**: operator can see synthesis origin via `[SYN]` prefix or (better) `source_type` column display
2. **Avoids arbitrary policy**: no downrank multiplier until failure data exists
3. **Clean axes**: keeps `source_type` (provenance) and `knowledge_type` (epistemic) separate
4. **Matches observed reality**: 28 syntheses, zero confirmed hallucinations → no need for ranking penalties
5. **Minimal surface**: 1 read-only CLI command + 3 lines of CLI formatting

---

## Challenger's correct observations (reflected in recommendation)

✓ Option 2 (LLM verification) is self-correction corruption — same model, same biases
✓ Option 3's 0.7 multiplier is arbitrary — no data to justify it
✓ Zero hallucinations found — argue for deferral of preventative options
✓ Option 4 should be "show source_type in CLI" not "mutate title"

All of these informed the shift from Architect's `1+3+4` to Codex's `1+4`.

---

## 2027 forward heuristic

Keep axes orthogonal:
- `source_type` = origin/provenance (synthesis vs. primary)
- `knowledge_type` = epistemic class (factual/decisional/experiential)

Add verification or ranking penalties only when:
1. Audited failure rate > 1 in 20 syntheses, **AND**
2. Syntheses regularly occupy 2+ of top-5 on normal queries, **OR**
3. Manual audit volume becomes operationally annoying (>50-100 syntheses per quarter)

No preventative optimization without signal.
