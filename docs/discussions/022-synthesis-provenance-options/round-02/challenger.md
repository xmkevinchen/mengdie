---
role: challenger
round: 2
discussion: "022"
topic: "01"
created: 2026-04-23
---

# Challenger Round 2 — Verification, Corrections, and Convergence

## Round 1 Retraction: Option 4 Redundancy Claim Is Wrong

**Claim retracted**: In Round 1 I argued that Option 4 (`[SYN]` prefix) is redundant
because `SearchResultItem.source_type` already carries the provenance signal.

**Verification finding**: `source_type` is not printed anywhere in `src/bin/cli.rs`
output code (`rg -n "source_type" src/bin/cli.rs` → 0 matches). The current CLI
output for `mengdie search` prints:

```
1. [score: 0.7421] Consolidated threshold lessons (factual)
   source: synthesis/... | entities: ... | recalled: 0x
```

The `(factual)` token is `knowledge_type`, not `source_type`. An operator reading
this output cannot distinguish a synthesis row from a primary factual row. The
`source_type` field is in the struct and in the MCP response; it is invisible at
the human CLI surface.

**Corrected position**: Option 4 (`[SYN]` prefix) OR displaying `source_type` in
the CLI column is required, not optional. The BL's provenance-visibility sub-problem
is real and currently unaddressed in the operator-facing CLI. My Round 1 claim to
the contrary was based on the struct field existing, not on whether it is rendered.

---

## Round 1 Claim Upheld: `KnowledgeType::Synthesized` Is Absent — Option 5 Still Stands

**Verification**: `src/core/mcp_tools.rs:63-67` shows three variants: `Decisional`,
`Experiential`, `Factual`. No `Synthesized` variant exists.

**Additional finding — parser doubles the problem**: `src/core/parser.rs:52-56`
contains the inference rule:

```rust
let knowledge_type = match source_type.as_str() {
    "conclusion" | "plan" => "decisional".to_string(),
    "review" | "retrospect" => "experiential".to_string(),
    _ => "factual".to_string(),
};
```

The `_ => "factual"` arm fires for `"synthesis"` (as well as any unknown future
source type). This means even if a caller passes `source_type: "synthesis"` through
the parser path, it receives `knowledge_type: "factual"` by inference. `dreaming.rs:565`
also hardcodes `"factual".to_string()` directly. The missing enum variant is encoded
in two independent code locations.

**Option 5 migration story (honest assessment)**:

Adding `KnowledgeType::Synthesized` requires:
1. New enum variant in `mcp_tools.rs` (2 lines, trivial).
2. `Display` impl arm (1 line).
3. Update `parser.rs` match arm: `"synthesis" => "synthesized"` (1 line).
4. Update `dreaming.rs:565`: `knowledge_type: "synthesized".to_string()` (1 line).
5. **Migration**: existing synthesis rows in the DB store `"factual"` as a string in
   the `knowledge_type` column. SQLite stores this as raw text — no enum constraint.
   An `UPDATE memory_entries SET knowledge_type = 'synthesized' WHERE source_type = 'synthesis'`
   backfill is required, gated on the schema version bump. This is a one-query
   migration, not a schema shape change (no new columns, no table restructuring).

**This is not 2 lines.** It is 5 code changes plus a data migration. The migration
is safe and idempotent, but it is a migration. The "co-land in v5" argument from
Round 1 still holds: BL-synthesis-dedup-key's v5 migration is already planned, and
the backfill query can be added to the v5 migration block at zero additional schema
version cost.

**Confidence: High.** Option 5 is real, under-scoped in Round 1 (not 2 lines), but
still achievable and correct. The honest scope is: 5 source code edits + 1 migration
query co-landing in v5.

---

## Round 1 Claim Upheld: Zero Hallucinations — Source Is Documented, Not Anecdotal

**Verification**: `docs/backlog/BL-clustering-validation.md` contains two separate
documented empirical sections:

1. **BL-007 empirical results** (first dream run, 2026-04-18): "No obvious hallucinations
   in the 3-5 rows spot-checked." Source: plan AC5 writeback, recorded by Kai.
2. **BL-residuals-reduction empirical results** (second run, 2026-04-19): "No
   hallucination patterns spotted in the 3-title spot-check."

Both are written records in the project backlog, not conversational memory. They are
operator spot-checks (3-5 rows each), not systematic coverage. The honest framing:
27 syntheses total, ~6-10 spot-checked, zero hallucinations in those rows.

**Caveat accepted**: 6-10 spot-checks out of 27 is not statistical coverage. The
"zero hallucinations" claim is an observation from a small sample, not a proof of
reliability. A 5% hallucination rate would expect ~1 bad synthesis in 27; a spot-check
of 10 has ~60% probability of missing it. The claim should be: "No hallucinations
observed in the spot-checked rows, with incomplete coverage of the full corpus."

**This does not change the urgency calculus.** The BL's own text says "only justify
[option 2] if manual audit proves syntheses are unreliable." The spot-check data
is the available evidence; it points toward reliable, not unreliable. The argument
for deferring Option 2 remains intact.

---

## Position on Peer Findings

### Architect's recommendation: Options 1 + 3 + 4, defer Option 2

**Agreement**: defer Option 2 (same reasoning: no fidelity data, doubles LLM cost,
schema migration overhead).

**Partial agreement on Option 3**: the architect's multiplier argument is better
calibrated than my Round 1 attack. `0.7` as a "tie-breaker" (primary wins when scores
are tied) is more defensible than `0.5`. However, the architect's own text says:
"No data from the 13-synthesis run directly supports 0.7 vs 0.5." This is exactly
my Round 1 objection, just with a different arbitrary number. `0.7` is less aggressive
than `0.5` but equally uncalibrated. The question is not whether to pick 0.5 or 0.7;
it is whether to ship a multiplier at all before any operator has reported a problem
caused by synthesis rows outranking primary sources.

**Codex-proxy disagrees with architect on Option 3** and is correct: ship Options
1 + 4, defer Option 3 until failure data exists. Codex-proxy notes that syntheses
now constitute ~40% of the search denominator — a `0.7` multiplier at 40% prevalence
is highly visible behavior for a change with no empirical justification.

**Option 5 interaction with architect's recommendation**: if Option 5 (`KnowledgeType::Synthesized`)
co-lands in v5, Option 3's downrank can use `knowledge_type == "synthesized"` as
its discriminator instead of `source_type == "synthesis"`. This is strictly cleaner:
`source_type` describes provenance chain; `knowledge_type` describes epistemic status.
A downrank on epistemic status is semantically correct. A downrank on provenance
chain is an approximation. If Option 3 ships at all, it should use `knowledge_type`.

### Codex-proxy's recommendation: Options 1 + 4, defer Options 2 + 3

**Agreement**: this is the minimum-viable correct set given current data.

**Reinforcing finding**: codex-proxy correctly identified that `source_type` is not
in `list --format json` output — but more importantly, `source_type` is not in ANY
CLI output currently (confirmed above). The fix should be: display `source_type`
in `cmd_search` and `cmd_list` output (adding it to the existing column that shows
`knowledge_type`), not just add a `[SYN]` prefix. The prefix is one implementation
of the fix; displaying the `source_type` column is another.

---

## Revised Position After Round 2 Verification

**What I'm defending**:

1. **Option 5 (`KnowledgeType::Synthesized`) is the correct structural fix**, not
   2 lines but achievable: 5 code edits + 1 backfill query in v5. It removes the
   semantic lie ("synthesis is factual") at the type level and provides a machine-readable
   discriminator for all downstream callers. It co-lands in BL-synthesis-dedup-key's
   v5 at zero additional schema-version cost.

2. **Option 4 is necessary** — I was wrong in Round 1. `source_type` is invisible
   in CLI output; the provenance-visibility sub-problem is real. Whether implemented
   as a `[SYN]` prefix or a new `source_type` column in CLI output, something must
   change.

3. **Option 3 should defer** — codex-proxy's position is better supported by the
   evidence than architect's. No failure data, high synthesis prevalence, arbitrary
   multiplier. Defer until audit evidence (Option 1) produces a fidelity signal.

4. **Option 2 should defer** — unanimous across all Round 1 agents. No disagreement.

**Minimum viable v0.8.0 recommendation**:

- Option 1 (audit subcommand) + Option 4/5 (surface provenance in CLI, ideally via
  `KnowledgeType::Synthesized` + display it, falling back to `[SYN]` prefix if
  Option 5 co-landing is judged out-of-scope for v0.8.0).
- Option 3 (downrank): defer. Revisit when Option 1 audit data shows any hallucination.
- Option 2 (LLM verify): defer to v0.9.0 or when audit data proves need.

**Confidence: High on the combination. Medium on whether Option 5 will be judged
in-scope for v0.8.0 (it requires a migration, which may land it in v5 scope
alongside BL-synthesis-dedup-key, which could be the right home anyway).**
