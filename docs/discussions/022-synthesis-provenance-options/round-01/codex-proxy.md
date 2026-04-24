---
agent: codex-proxy
round: 1
created: 2026-04-23
---

# Codex Proxy: Cross-Family Scoring

## Summary

Ship **Option 1 (audit subcommand) + Option 4 (CLI prefix)** for v0.8.0.

Defer Option 2 (LLM verification) and Option 3 (downrank) until failure data exists.

## Detailed Findings

### Option 3 (`×0.5` downrank)

**Operationally meaningful threshold**: ~15-20% of likely hit candidates. At your current corpus state, ~27 syntheses / 68 total memories ≈ 40% in search denominators, making `×0.5` very visible. Not "noise"—a real reorder. This is exactly why shipping it blind is risky.

### Option 2 (LLM verification cost)

- **Per-pass cost**: ~14-25 LLM calls per dream run (one per cluster).
- A verification pass would double this to ~28-50 calls serially (not parallelized by `#[tokio::main]`).
- **Wall-clock hit**: serial subprocess spawns are additive latency.
- **Verdict**: Overkill at current scale; wait until audit data shows hallucination prevalence.

### Audit scale breakdown

"~50 synthesis rows" is guessed, not data-grounded. Practical scale:
- ~30 syntheses: still workable for manual audit.
- ~50: fuzzy upper bound where "audit everything" becomes annoying.
- ~100: painful; require sampling.
- ~500: manual audit dead.

### Option 4 CLI prefix UX

`[SYN]` prefix is safe. JSON question is moot: `mengdie search` has no `--format json` today. MCP already distinguishes via `source_type`. If shipped, add `source_type` to `list --format json` output rather than polluting title.

Concrete UX:
```
1. [score: 0.7421] [SYN] Consolidated threshold lessons (factual)
```

### Minimum viable v0.8.0

Ship **Options 1 + 4**:
- Solves current problem: provenance visibility + audit path.
- No schema coupling to dedup-key.
- Preserves search behavior while "no hallucinations spotted."
- Option 3 already meaningful at 40% prevalence—ship it only after failure data.
- Option 2 doubles subprocess work; defer until audit or incidents prove need.

### 2027 forward heuristic

"When syntheses exceed ad-hoc audit capacity, sample 20/month. If >1 fails fidelity check or syntheses routinely occupy 2+ of top-5 results, enable automated verification before ranking penalties."

---

## Implementation Notes

- **Option 1 caveat**: Link table exists but repo lacks `get_synthesis_sources(id)` bulk helper. Add it.
- **Option 4 caveat**: Include `source_type` in `list --format json` (currently missing); preserve search rank order.
- **No coupling to BL-synthesis-dedup-key**: both options are orthogonal to schema migration (v5).
