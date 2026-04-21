---
author: doodlestein
date: 2026-04-20
topic: most-likely-reversal in 6 months
---

# Regret Analysis — BL-008 Conclusion

## Decision most likely to be reversed: Topic 1 — Floor = 0.20

**The assumption most likely to fail**: the corpus will remain tight (IQR ≈ 0.015) for six months.

The floor of 0.20 was set because the archaeologist found `avg_relevance` compressed into a narrow band around 0.487, making any percentile-based approach useless and requiring an absolute floor. The conclusion correctly documents the revisit trigger — "IQR widens past 0.05" — but treats that widening as a speculative future event rather than an expected one.

**Why this assumption is fragile**: the current corpus is 41 long-term memories, nearly all produced by a single first real dream run. As real usage accumulates over 90+ days, two forces push the distribution wider: (1) synthesis memories tend to score higher because they consolidate high-signal content, while raw ingest memories span a wider range; (2) the `recall_count` burst-inflation pathology (open tail #1) will inflate `avg_relevance` for memories that happen to get hit in a session burst, creating a high-end tail. A wider IQR shifts the effective demotion trigger. Under the current formula, a memory with `avg_relevance = 0.22` crosses the floor at ~11 days of silence — aggressive enough to demote recently-ingested but briefly-unretrieved content that is actually still relevant.

**The reversal shape**: floor raised to 0.15 or made corpus-relative (e.g., `mean - 1.5×IQR`) once real demotion events begin appearing in `DreamingResult.demoted` and the `--dry-run-decay` counter shows unexpectedly high `decay_floor_breaches`. The dry-run flag is load-bearing precisely because the first real demotion won't arrive until ~77 days post-ship — but if the corpus distribution drifts before that, operators have no early signal except the breach count.

**Not a blocker for shipping**: the converged design is correct as stated. This is a tuning constant in a pure function; reversal costs one line edit and a re-run of the regression table.
