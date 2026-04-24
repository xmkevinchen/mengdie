---
role: challenger
round: 1
discussion: "022"
topic: "01"
created: 2026-04-23
---

# Challenger — Attacking the 4 Options and Their Assumptions

## Challenge 1 (Sharpest): The `KnowledgeType` enum is the root bug — all 4 options paper over it

**Claim**: The correct fix is Option 5 (not enumerated): add `KnowledgeType::Synthesized` and use it. All 4 options treat the symptom; this treats the cause.

**Evidence**: `KnowledgeType` in `src/core/mcp_tools.rs:63` has three variants: `Decisional`, `Experiential`, `Factual`. Synthesis rows are tagged `Factual` because "there is no better value" — the enum simply doesn't have a semantic slot for derived/synthesized knowledge. This is an enum design gap, not a missing audit tool or a missing score multiplier.

The BL itself says: "A hallucination… will pass all validations and land in the database as a factual-tagged row." But the real statement is: "We have no `knowledge_type` value that means 'this was derived by LLM from other rows.'" Options 1-4 all accept `knowledge_type=factual` as a permanent fact and try to compensate around it. Option 5 changes the field value.

**Why this is cheaper than Options 1-4**: `KnowledgeType::Synthesized` is a two-line enum variant addition. It compiles. It gives every downstream consumer — FTS queries, search ranking, MCP snippets, ae:analyze injection — a native discriminator with zero new CLI surface and zero new columns. `SearchResultItem.knowledge_type` already carries the field as `String`. Consumers that don't care about synthesis provenance already ignore it.

**Objection anticipated**: "synthesis rows aren't only factual." True, but the point is that "factual" is the wrong label for a derived row regardless of its accuracy. A synthesis that correctly consolidates three decisions is still not the same epistemological class as a primary decision row. `Synthesized` is accurate; `Factual` is a lie.

**Confidence: High.** The missing enum variant is observable in the code and was silently accepted at ship time because the BL was filed instead of fixed. The 4-option framing in the BL never named this possibility.

---

## Challenge 2: Is the problem real? Zero confirmed hallucinations in 13 syntheses

**Claim**: The BL fires on an anticipated risk, not an observed one. No synthesis has been identified as a hallucination.

**Evidence**: `docs/backlog/BL-clustering-validation.md` (BL-007 empirical results section) records the first real dream run: 13 syntheses, spot-checked 3-5. "No obvious hallucinations in the 3-5 rows spot-checked." The second run (BL-residuals-reduction section) added 14 more syntheses; again "no hallucination patterns spotted in the 3-title spot-check." Both runs used the same `ClaudeCliProvider`.

The BL trigger condition is: "First real-data dream run, OR operator reports bad ae:analyze output, OR >50 synthesis rows." Trigger 1 fired (first real run completed). Trigger 2 has not fired. Trigger 3 has not fired (corpus is 14+14=28 synthesis rows, still below 50).

**Implication**: By the BL's own trigger logic, we are in the state where "real data exists and no bad output was reported." That is not the same as "bad output was confirmed." If the BL's intent was "close this if the first run looks fine," then options 1-4 should be weighed against the possibility that deferral is the correct answer at this moment.

**Counter-counter**: 28 synthesis rows is a small sample. The BL files for V0.8.0. However, this discussion should name the probability: the probability that any of the options prevent a problem that has not yet materialized vs. the cost of shipping them.

**Confidence: Medium.** The hallucination risk is real. But the urgency of any specific option is directly proportional to observed bad output, not theoretical exposure.

---

## Challenge 3: Option 2 (LLM verification) is theater — same model, same biases

**Claim**: A second LLM pass using the same model family that produced the synthesis cannot reliably catch the synthesis's hallucinations. The verification signal is corrupted by construction.

**Evidence**: The BL does not specify a different model or prompt strategy for the verification pass. In practice it would be another `ClaudeCliProvider` call — same Claude CLI, same underlying model. If `claude-sonnet` hallucinated "the BL-006 threshold is 0.80" when the primary sources said 0.75, a second `claude-sonnet` call scoring the synthesis would tend to agree (the model's prior for "0.80 sounds like a reasonable threshold" is the same in both calls).

This is not a hypothetical: LLM self-correction research (GPT-4 evaluating GPT-4 outputs, etc.) consistently shows high agreement rates between same-family evaluators even when both are wrong. Cross-family verification (Claude verifying Codex output) would have signal. Same-family verification is correlation, not independence.

**Additionally**: `memory_quality_score REAL` stored per-synthesis implies a false precision. A score of 7.2 vs. 6.8 carries no calibrated meaning across runs, prompts, or model versions. It creates an artifact that looks like instrumentation but measures nothing stable.

**Objection anticipated**: "Even imperfect LLM scoring catches gross hallucinations." Possibly. But the BL's own text says "only justify if manual audit proves syntheses are unreliable." We have not proved they are unreliable (see Challenge 2). Option 2 is doubling LLM call count per dream pass for a verification signal that is corrupted and would only be justified by evidence we do not yet have.

**Confidence: High on the independence problem. Medium on whether imperfect verification has zero value.**

---

## Challenge 4: Option 3 (downrank `score *= 0.5`) — the 0.5 is made up

**Claim**: There is no data in this project that justifies 0.5 specifically. The number is a placeholder presented as a design decision.

**Evidence**: The BL says "`score *= 0.5` or similar." The framing doc repeats it verbatim. No derivation is given. No search-quality data exists (the BL itself notes that "synthesis_hit_rate instrumentation is deferred" in `BL-clustering-validation.md`). No A/B comparison exists between synthesis-vs-primary search result quality.

In the absence of data: if syntheses are 90% accurate (likely, given zero confirmed hallucinations), then 0.5 multiplier means an accurate synthesis ranks below a lower-relevance primary source — actively degrading search quality. If syntheses are 50% accurate (the hallucination-risk scenario), 0.5 is still too generous. The correct multiplier is determined by measured accuracy rate, which we do not have.

**Specific consequence**: The framing doc notes "some operators will want syntheses to rank high when they are accurate." That's not a preference; it's the correct behavior for a high-accuracy synthesis corpus. Option 3 makes the search experience worse for the confirmed-working state.

**Confidence: High.** 0.5 is arbitrary. Shipping an arbitrary number as a feature introduces a calibration debt that accumulates as synthesis quality either proves high (0.5 was too aggressive) or proves low (0.5 was too generous).

---

## Challenge 5: Option 4 (`[SYN]` prefix) — `source_type` already carries this signal

**Claim**: Option 4 is redundant for machine consumers and has limited value for human consumers in an AI-primary workflow.

**Evidence**: `SearchResultItem.source_type` was explicitly added in the BL-007 review fixup (review `b001d6c`, finding #4). The field exists precisely so "consumers can distinguish syntheses from primaries." If `ae:analyze` or a calling agent wants to filter synthesis rows, it queries `source_type == "synthesis"`. No prefix needed.

For CLI human usage: mengdie is used by one person (Kai). The synthesis rows are identifiable by the `source_type` column in `mengdie search` output if that column is displayed. If it isn't displayed, the fix is to display it — not to additionally prefix the title.

**Deeper issue**: Title prefixes leak into the memory content. If the synthesis is later used as a primary source for another synthesis (which is not impossible as corpus grows), the `[SYN]` literal ends up in the prompt. That's noise in the LLM context and could confuse the prompt parser.

**Objection anticipated**: "Displaying source_type requires column formatting; `[SYN]` is visible at a glance." Valid UX argument. But this is a partial option (pure UX, no algorithm change) being evaluated alongside three algorithmic options. It should be assessed as "display source_type in CLI output" rather than "mutate the title string."

**Confidence: High on the redundancy with source_type. Medium on whether the UX convenience justifies it anyway.**

---

## Challenge 6: Option 1 (audit subcommand) — it's deferred maintenance, not a fix

**Claim**: Option 1 names itself "audit" but produces no code that audits anything — it gives the operator a tool to do the audit manually. That is deferred maintenance with a CLI wrapper.

**Evidence**: The BL description: "prints the synthesis content alongside its source memories. Operator eyeballs fidelity." The operator does the audit. The code does a DB join. The word "audit" implies automated verification; this is a side-by-side display that requires a human to read and compare.

For a solo-developer project (per the project status note: "solo dev"), the option asks Kai to manually read each synthesis against its sources. That's already possible today via direct DB queries. The subcommand reduces friction but does not reduce the cognitive load or time cost of the audit.

**Structural problem**: If the 13 current syntheses are all accurate (empirically the case so far), running `mengdie synthesis audit <id>` on each would confirm what the spot-check already confirmed. The audit tool earns its keep only when bad syntheses start appearing — at which point the operator knows something is wrong anyway (via ae:analyze giving bad suggestions) and can already identify the problem through `source_type` filtering.

**However**: Option 1 is the cheapest option and has zero blast radius (read-only). If combined with Option 4 (CLI prefix) as the minimum-viable provenance package, it is defensible as "we are doing something that costs little and closes the BL." The issue is framing it as an audit mechanism rather than a debug tool.

**Confidence: Medium.** Option 1 is harmless. The challenge is against calling it an audit — it is a display command.

---

## Blind Spot Summary

The BL and framing doc share one structural blind spot: **they treat `knowledge_type=factual` on synthesis rows as a fixed constraint and build around it.** The four options are all compensations. None of them question whether `KnowledgeType::Synthesized` should exist.

The implications:
1. `knowledge_type=factual` is searchable as a filter. If a caller wants "only primary factual records," it cannot exclude synthesis rows without also filtering on `source_type`. Two fields are now needed where one should suffice.
2. Contradiction detection in `contradiction.rs` uses entity tags and temporal validity. If `knowledge_type` were `synthesized`, contradiction checks could apply stricter rules (e.g., a synthesized row cannot contradict a primary decisional row — only the inverse matters). The current schema loses this signal.
3. The `KnowledgeType` enum is small (3 variants). Adding `Synthesized` is a one-migration change. The v5 migration from `BL-synthesis-dedup-key` is already planned — co-landing a `knowledge_type` semantic fix there costs nothing extra in migration terms.

**Recommended test**: If Option 5 (`KnowledgeType::Synthesized`) were chosen, which of Options 1-4 would still be needed? Option 4 (prefix) becomes redundant — `knowledge_type` is the machine-readable tag, and the CLI can display it. Option 3 (downrank) could use `knowledge_type=synthesized` as the ranking signal instead of a hardcoded multiplier. Option 1 (audit subcommand) remains useful as a display tool regardless of the enum fix. Option 2 (LLM verification) remains unjustified regardless.

The minimum-viable fix, given zero confirmed hallucinations, may simply be: `KnowledgeType::Synthesized` + display the field in CLI output. That addresses the BL's provenance-visibility sub-problem without any new CLI surface, any new column, or any algorithmic change.
