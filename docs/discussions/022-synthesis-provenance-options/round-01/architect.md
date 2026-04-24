---
agent: architect
round: 1
created: 2026-04-23
---

# Architect Findings — Option Selection for BL-synthesis-provenance

## Findings per option (evidence-based recommendation for each)

### Option 1: audit subcommand

`mengdie synthesis audit <syn-id>` — prints synthesis + its source memories side-by-side.

**Recommendation: SHIP in v0.8.0.**

Evidence:
- Read-only CLI. Zero blast radius. No schema change.
- The link table (`memory_synthesis_links`) already exists (v4 schema). `count_synthesis_links` is already in `db.rs:398`. A `get_synthesis_sources(synthesis_id)` query is one `SELECT me.* FROM memory_entries me JOIN memory_synthesis_links l ON l.source_memory_id = me.id WHERE l.synthesis_memory_id = ?1` call — new helper, but trivial (no schema migration).
- BL body explicitly calls this out as "cheapest; manual." The 13-synthesis first-real-run (plan 010 AC5) already makes the corpus small enough that manual audit is the right first tool.
- Makes option 4 (CLI prefix) actionable: an operator who sees `[SYN]` in search results can then `audit <id>` to verify fidelity without leaving the CLI.

**Constraint noted in topic summary**: the FK query at `get_synthesis_links` needs to exist. Status: `count_synthesis_links` exists; `get_synthesis_sources` does not. One new query to add in the plan — not a blocker.

---

### Option 2: LLM verification

Second LLM pass per cluster to score fidelity 0–10, stored in new `memory_quality_score REAL` column.

**Recommendation: DEFER to v0.9.0 or later.**

Evidence:
- Heaviest option by implementation cost AND per-run runtime cost.
- **Schema dep**: requires a new column `memory_quality_score REAL`. BL-synthesis-dedup-key ships schema v5 (`synthesis_cluster_hash TEXT` + new unique index). If option 2 co-lands, it either piggybacks on v5 (cleaner — one migration) or needs its own v6. Two independent schema migrations in one sprint adds unnecessary complexity; the right call is to defer option 2 unless we decide the fidelity signal is urgent.
- **LLM cost calculation**: 13 syntheses at current corpus size → 13 additional claude calls per dream pass. Dream is operator-triggered (no daemon yet — BL-010 not landed). Cost is bounded but not zero. BL body says "expensive — only justify if manual audit proves syntheses are unreliable." We don't have that evidence yet. Audit (option 1) is the tool that generates that evidence.
- **Sequencing logic**: option 1 (audit) is the signal generator. Option 2 is the automated response to the signal. Ship 1 first, gather evidence, then decide if 2 is worth it.
- **v0.8.0 budget**: plans 014/015/016 have already shipped. The synthesis cluster budget in v0.8.0 needs to cover BL-synthesis-dedup-key (schema v5, dedup key rework) + this BL. Option 2 would roughly triple the implementation scope of this BL.

---

### Option 3: downrank in search

`score *= 0.5` (or other multiplier) in `search.rs::apply_boost_and_decay`.

**Recommendation: SHIP in v0.8.0, with multiplier 0.7 (not 0.5).**

Evidence:
- Cheapest algorithm change. One line in `apply_boost_and_decay` (`search.rs:47-64`): add a branch `if entry.source_type == "synthesis" { boosted * 0.7 }`.
- **Multiplier justification**: the 13-synthesis first-real-run did not produce a fidelity score (option 2 not yet built), so there is no data supporting a specific value. However:
  - 0.5 is too aggressive. A good synthesis that consolidates 5 high-recall memories should still surface near primary sources when it's the best match for a query. Cutting to 50% would systematically bury accurate syntheses.
  - 0.7 is defensible as a "tie-breaker" multiplier. If a synthesis and a primary source both score 1.0 RRF-normalized, the synthesis becomes 0.7 → the primary wins. If only the synthesis matches (no primary with the same content), it still surfaces at 0.7 (passing most threshold filters).
  - 0.7 is also consistent with the `LONGTERM_BOOST = 1.2` design philosophy: that boost is a modest +20%, not a 2× amplification. A modest downrank (−30%) follows the same proportionality principle.
- **Contention risk is real** (BL body acknowledges it): operators who see syntheses drop in rank without knowing why will be confused. Mitigation: option 4 (CLI prefix) makes the `[SYN]` label visible, so operators can correlate "this result has `[SYN]` and a slightly lower score" with the deliberate policy. This is the primary reason options 3 and 4 should ship together.
- **Rollout discipline**: the score formula change is immediately observable. Document it in the `apply_boost_and_decay` comment and the dream CLI output (`--synthesize` summary line should note "synthesis rows downranked 0.7× in search").
- **No data from the 13-synthesis run** directly supports 0.7 vs 0.5. The choice is conservative-by-design pending real fidelity measurement (option 2). If option 2 later shows that syntheses are >80% accurate, the multiplier can be relaxed to 0.9 or removed.

---

### Option 4: CLI prefix

`[SYN]` prefix on titles in `mengdie search` and `mengdie list` output.

**Recommendation: SHIP in v0.8.0.**

Evidence:
- Pure UX. Zero algorithm change. Zero schema change.
- `cmd_search` (`cli.rs:610-625`) already formats per-result output; adding `[SYN]` when `r.entry.source_type == "synthesis"` is a 3-line change.
- `cmd_list` (`cli.rs:551-573`) renders a table; same `source_type` check + prefix.
- The `source_type` field is already available on `MemoryEntry` and was added to `SearchResultItem` in the BL-007 review fixup (`b001d6c`). No new data flow needed.
- **Standalone viability**: option 4 does NOT require option 1 to ship first. An operator who sees `[SYN]` in search output gains provenance visibility immediately, even without the audit subcommand. Option 1 makes the prefix *actionable* (drill-down), but option 4 is valuable on its own.
- This is the fastest path to addressing sub-problem 1 (provenance visibility) from the BL framing.

---

## Orthogonality matrix

| | Opt 1 (audit) | Opt 2 (LLM verify) | Opt 3 (downrank) | Opt 4 (prefix) |
|---|---|---|---|---|
| **Opt 1 (audit)** | — | complementary (1 generates evidence for 2) | orthogonal | complementary (4 surfaces what 1 drills into) |
| **Opt 2 (LLM verify)** | | — | complementary (quality score could replace static multiplier) | orthogonal |
| **Opt 3 (downrank)** | | | — | should co-ship (prefix explains why score is lower) |
| **Opt 4 (prefix)** | | | | — |

No conflicts. All four can combine. The only coupling is:
- 3+4 should co-ship (prefix explains the downrank)
- 1 should ship before 2 (audit is the evidence mechanism for justifying LLM verification)
- 1+4 together is the minimum-viable set for provenance visibility (sub-problem 1)

---

## Recommended combination for v0.8.0 plan

**Ship: Options 1 + 3 + 4. Defer: Option 2.**

Rationale:
- Options 1, 3, 4 together address both sub-problems from the BL framing:
  - **Provenance visibility** (sub-problem 1): option 4 marks syntheses in CLI output; option 1 lets the operator drill into any marked synthesis.
  - **Fidelity detection** (sub-problem 2): option 3 reduces the blast radius of a hallucinated synthesis by ensuring it ranks below a primary source on tied queries. Option 1 enables manual verification.
- Option 2 is deferred because: (a) no empirical evidence yet that syntheses are unreliable at a rate that justifies automated scoring, (b) schema migration cost should be consolidated with BL-synthesis-dedup-key's v5 migration if we add option 2 — and the v5 migration is already scoped without option 2's column, (c) v0.8.0 budget is thin after plans 014/015/016.
- The combination is minimum-viable: if option 3's downrank proves too aggressive in practice (operator feedback: "my good syntheses aren't surfacing"), the multiplier can be tuned or removed without any schema or CLI surface change. If option 2 is later justified by audit evidence, it adds a column to v6 and extends the dream pass — no rework of 1, 3, or 4.

**Smallest subset that addresses "factual-tagged hallucinations pollute search" and "operator can't distinguish syntheses":**
- Minimum: options 4 + 3 (prefix + downrank). No new commands, no schema change.
- Recommended minimum: add option 1 (audit). Adds one CLI subcommand (~50 lines), zero schema change, and converts the manual fidelity process from impossible (operator must read DB rows) to ergonomic.

---

## Dep on BL-synthesis-dedup-key (v5 migration dependency if any)

**No hard dependency if option 2 is deferred.**

- BL-synthesis-dedup-key Option A ships schema v5 by adding `synthesis_cluster_hash TEXT` column + a `(project_id, synthesis_cluster_hash) WHERE source_type = 'synthesis'` partial index.
- Options 1, 3, 4 require NO schema change. They land independently of BL-synthesis-dedup-key's v5 migration.
- If option 2 (LLM verification) were chosen, `memory_quality_score REAL` would need a migration. The clean landing would be to co-include it in BL-synthesis-dedup-key's v5 migration (one migration, two new columns), since both are synthesis-cluster concerns. But since option 2 is deferred, there is no co-migration concern for v0.8.0.
- **Schema version after v0.8.0 synthesis cluster**: v5 (from BL-synthesis-dedup-key). No v6 needed.

---

## Open questions

1. **Multiplier tuneability**: should the 0.7 downrank multiplier be hardcoded or configurable via `MengdieConfig`? Given that it's a policy decision with no empirical data yet, a named constant (e.g., `SYNTHESIS_TRUST_MULTIPLIER: f64 = 0.7`) in `search.rs` is sufficient — no need for a config knob until we have data suggesting operators want to override it.

2. **`get_synthesis_sources` helper scope**: option 1 (audit) needs a new `Db::get_synthesis_sources(synthesis_id)` helper. The plan should specify whether this helper is also exposed for future MCP tooling or is CLI-only for now.

3. **`mengdie synthesis` as a subcommand group**: option 1 implies a new top-level subcommand `synthesis` with a nested `audit` sub-subcommand. Alternatively, it could be `mengdie audit <syn-id>` (flat). The nested form is cleaner for extensibility (future `synthesis list`, `synthesis rebuild`) but adds one level of CLI hierarchy. For v0.8.0 scope, flat `mengdie audit <id>` avoids introducing a subcommand group for a single command.
