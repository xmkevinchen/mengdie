---
agent: architect
round: 2
discussion: "022"
topic: "01"
created: 2026-04-23
---

# Architect Round 2 — Engage Peers, Revise Positions

## Biggest shift from Round 1

**Option 3 (downrank) deferred.** Codex-proxy's 40% prevalence data (codex-proxy.md:19: "~27 syntheses / 68 total memories ≈ 40%") changes the calculus. At 40% synthesis prevalence, a ranking penalty is a search-behavior overhaul, not a tie-breaker — and challenger documents zero confirmed hallucinations (challenger.md:31-33). Shipping a visible algorithmic change against a zero-hallucination corpus is the wrong risk/reward trade.

**Option 5 (KnowledgeType::Synthesized) accepted as a complement, not a replacement.** Challenger's root-bug argument (challenger.md:13-23) is structurally correct. The enum gap is real and independently fixable. However, it does not fully replace Option 4 (CLI prefix display), and the migration story needs one clarification.

---

## Option 5: KnowledgeType::Synthesized — Accept as complement

### Challenger's argument accepted

Challenger.md:13 states: "The correct fix is Option 5 (not enumerated): add `KnowledgeType::Synthesized` and use it." The evidence is solid:

- `KnowledgeType` enum at `src/core/mcp_tools.rs:63-66` has three variants: `Decisional`, `Experiential`, `Factual`. No `Synthesized` variant exists.
- `dreaming.rs:565` hardcodes `knowledge_type: "factual".to_string()` for all synthesis rows. The BL confirms this was accepted at ship time because the variant was missing.
- Challenger.md:111 correctly notes that downstream filters (FTS queries, contradiction checks) that want to exclude syntheses currently need TWO fields (`source_type` AND `knowledge_type`) where one semantically correct `knowledge_type=synthesized` would suffice.
- `contradiction.rs:104` already has the pattern `if new_knowledge_type == "decisional" && knowledge_type == "decisional"` — adding `synthesized` as a discriminator would give the contradiction engine a clean signal to skip synthesis-vs-primary comparison (a synthesis cannot contradict a primary; only a primary can supersede a synthesis).

**Option 5 is a root fix, not a workaround.** All four original options work around the missing variant. Option 5 fixes the enum gap.

### Does Option 5 replace Option 4?

**Partially, but not entirely.** Challenger.md:114 argues Option 4 "becomes redundant — `knowledge_type` is the machine-readable tag, and the CLI can display it." This is correct for the MCP consumer path: `SearchResultItem.knowledge_type` already carries the value, and once it reads `"synthesized"` instead of `"factual"`, the machine consumer has a native discriminator.

For the CLI human path, Option 4 (`[SYN]` prefix in `mengdie search` output) and the Option 5 fix converge on the same implementation decision: the CLI must display some signal that a result is a synthesis. Displaying `knowledge_type` in the search output (which currently shows `knowledge_type` in `cmd_search` at `cli.rs:617`) would immediately surface "synthesized" once Option 5 lands. The `[SYN]` prefix is redundant IF the `knowledge_type` column is already visible in CLI output.

**Conclusion**: Option 5 subsumes the machine-consumer benefit of Option 4. The human-consumer benefit of Option 4 is achieved by ensuring `knowledge_type` is visible in CLI output (already present at `cli.rs:617`). The `[SYN]` prefix as a separate title mutation (challenger.md:81: "Title prefixes leak into the memory content") is dropped; instead the plan should verify that `knowledge_type` is displayed prominently in CLI search output. Option 4 as originally scoped (title prefix) → replaced by Option 5 + confirming CLI visibility.

However: codex-proxy.md:38 notes `mengdie search` already displays `knowledge_type` in the result line. Once Option 5 changes the value from `"factual"` to `"synthesized"`, the display naturally reflects this. No additional CLI change is needed beyond Option 5.

### Migration story for existing synthesis rows

This is the one non-trivial piece of Option 5. The 27 existing synthesis rows in the production DB currently have `knowledge_type = 'factual'`. Adding the enum variant and changing `dreaming.rs:565` to emit `"synthesized"` fixes new rows going forward but leaves existing rows with stale values.

**Assessment**: a one-time `UPDATE memory_entries SET knowledge_type = 'synthesized' WHERE source_type = 'synthesis' AND knowledge_type = 'factual'` is required in the schema migration. This is safe and targeted — it matches only synthesis-sourced rows that were forced to `factual` by the old default, leaving all primary `factual` rows untouched. This backfill belongs in the v5 migration block (co-landing with BL-synthesis-dedup-key's `synthesis_cluster_hash` column addition), since it requires no new column, just a data update. Cost: one extra SQL statement in the migration block.

**Migration story**: v5 migration in `schema.rs` already adds `synthesis_cluster_hash TEXT` (BL-synthesis-dedup-key Option A). Add to the same v5 block: `UPDATE memory_entries SET knowledge_type = 'synthesized' WHERE source_type = 'synthesis' AND knowledge_type = 'factual'`. No v6 needed.

### Callsite changes for Option 5

Beyond the enum variant (2 lines) and the `Display` impl (1 line), the callsites that need updating:
- `dreaming.rs:565`: `knowledge_type: "factual"` → `knowledge_type: "synthesized"` (1 line)
- `src/core/parser.rs:55`: the fallback `_ => "factual".to_string()` does NOT need changing — that's the parser's fallback for ingested AE files (conclusions, plans) that don't match the known types. Synthesis rows are never ingested via the file watcher; they're written only by `run_synthesis_pass`. No parser change needed.
- `mcp_tools.rs:63-66`: add `Synthesized` variant, update `Display` impl, update serde deserialization (the `#[serde(rename_all = "lowercase")]` attribute handles it automatically — `"synthesized"` round-trips correctly).
- No schema column change needed (the column is `TEXT`, not an enum constraint).
- Tests in `db.rs` that hardcode `"factual"` for synthesis fixtures (`db.rs:798,818,999,1065`) need updating. Four test callsites.

Total callsite count: ~7 lines changed, 4 test callsites updated. This matches challenger.md:19's "two-line enum variant addition" claim, acknowledging the callsites are slightly more.

---

## Option 3 (downrank) — Defer

### Revised position: defer Option 3

Round 1 recommended 0.7 as a "conservative" multiplier. That was wrong given the corpus state codex-proxy quantified.

Codex-proxy.md:19: "~27 syntheses / 68 total memories ≈ 40% in search denominators, making `×0.5` very visible. Not 'noise' — a real reorder." The same logic applies to 0.7: at 40% synthesis prevalence, even a 30% ranking penalty changes the search result ordering for a substantial fraction of queries. This is not a tie-breaker; it's a structural rerank.

Challenger.md:31-33 documents zero confirmed hallucinations across 27 real-run syntheses (13 from run 1, 14 from run 2, per BL-clustering-validation.md). A ranking penalty is only justified if syntheses are demonstrably unreliable. Current evidence says the opposite.

The correct sequencing:
1. Ship Option 5 — fix the semantic gap.
2. Ship Option 1 — give the operator an audit tool.
3. Accumulate evidence via audit (if any synthesis proves bad, that's the trigger for Option 3).
4. If and when bad-rate data exists, ship Option 3 with a data-justified multiplier (not a guess).

Option 3 is not off the table permanently. It is off the table for v0.8.0 because the precondition ("manual audit proves syntheses are unreliable," per BL body) has not been met.

---

## Option 1 (audit subcommand) — Retain, re-label

Challenger.md:93 correctly notes that Option 1 is a "display command," not an automated audit — the operator does the cognitive work. The word "audit" is slightly aspirational. This does not change the recommendation to ship it: it is read-only, zero blast radius, and it is the mechanism by which the precondition for Option 3 gets evaluated.

One refinement: the subcommand surface should be `mengdie dream audit <syn-id>` or simply `mengdie audit <id>` (flat), not `mengdie synthesis audit` (codex-proxy's open question). Flat is simpler for v0.8.0 scope — no new subcommand group, one new subcommand.

The `get_synthesis_sources(id)` helper (flagged by codex-proxy.md:62 as missing) is a one-SQL helper; no schema change. Confirmed this is the only new code path Option 1 requires beyond the CLI command handler.

---

## Convergence: recommended minimum-viable combination

**Ship: Options 1 + 5. Defer: Options 2, 3. Option 4 superseded by 5.**

Breakdown:
- **Option 5** (`KnowledgeType::Synthesized`): root fix. Closes the enum gap, fixes provenance for machine consumers (MCP), fixes the `"factual"` lie. Requires ~7-line code change + backfill UPDATE in v5 migration (no extra migration). Co-lands with BL-synthesis-dedup-key's v5 block.
- **Option 1** (audit display command): operator drill-down tool. Generates the evidence needed to decide if Option 3 is ever justified. Zero blast radius. One new `get_synthesis_sources` DB helper + CLI handler (~50 lines).
- **Option 4 (title prefix)**: superseded by Option 5. Once `knowledge_type` reads `"synthesized"`, the CLI output at `cli.rs:617` already shows it. No title mutation needed. Challenger.md:81's title-pollution concern is also avoided.
- **Option 3 (downrank)**: deferred. 40% prevalence + zero hallucinations = wrong time to ship a ranking penalty.
- **Option 2 (LLM verification)**: deferred (unchanged from Round 1).

This is a smaller scope than Round 1's 1+3+4. It is the right scope given the evidence: zero confirmed hallucinations in 27 syntheses, 40% corpus prevalence making Option 3 structurally impactful, and Option 5's root fix making Option 4 redundant.

---

## Schema migration dependency summary (revised)

v5 migration block (co-landing with BL-synthesis-dedup-key):
1. Add `synthesis_cluster_hash TEXT` column (BL-synthesis-dedup-key Option A)
2. Add partial unique index on `(project_id, synthesis_cluster_hash) WHERE source_type = 'synthesis'`
3. **New (Option 5)**: `UPDATE memory_entries SET knowledge_type = 'synthesized' WHERE source_type = 'synthesis' AND knowledge_type = 'factual'`

No v6 needed. Options 1 and 5 both fit within v5. Option 2 (`memory_quality_score REAL`) would require v6 if it were chosen — it is not, so v5 is the terminal migration for this sprint.

---

## Open questions (revised)

1. **Contradiction engine**: once `knowledge_type = 'synthesized'` exists, should `contradiction.rs:104` explicitly skip synthesis-vs-primary comparisons? The current logic only triggers on decisional-vs-decisional. Syntheses against primaries are already not checked. This is a non-change today but worth noting for the plan's non-goals section.

2. **`IngestParams.knowledge_type` validation**: MCP `IngestParams` accepts `KnowledgeType` via serde. Adding `Synthesized` to the enum means external callers can now ingest with `knowledge_type: "synthesized"`. This is probably fine (it's a solo-operator tool), but the plan should document whether `"synthesized"` via MCP ingest is intentionally allowed.
