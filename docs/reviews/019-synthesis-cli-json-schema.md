---
id: "019"
title: "Review: synthesis CLI --json-schema adoption (BL-027 Path B)"
type: review
created: 2026-05-10
target: "docs/plans/019-synthesis-cli-json-schema.md"
verdict: pass
---

# Review: Plan 019 — synthesis CLI --json-schema adoption

**Scope**: `git diff 40228a6..HEAD` (10 commits including fixup), legacy
plan, standalone (no parent discussion).

**Team**: `plan-019-final-review` — 6 reviewers + TL.
- `security-reviewer` (Claude)
- `architecture-reviewer` (Claude)
- `performance-reviewer` (Claude)
- `challenger` (Claude, pure opposition)
- `codex-proxy` (OpenAI lens via Codex MCP)
- `gemini-proxy` (Google lens via oMLX gemma fallback — Gemini rate-limited)

## Verdict

**PASS** — 0 P1 findings across 6 reviewers. All P2 findings either fixed
in fixup commit `7f71fca` or filed as follow-up BLs with concrete triggers.

## Summary table (all findings)

| # | Finding | Reviewers (convergent) | Severity | Disposition |
|---|---|---|---|---|
| F1 | AC1 + `synthesis.rs` doc-comment stale (still says `oneOf`) | arch + perf + challenger + codex (**4**) | P2 | **FIXED** in `7f71fca` — AC1 rewritten with history-aware note; SYNTHESIS_OUTPUT_SCHEMA doc-comment rewritten to flat shape |
| F2 | BL-039 trigger phrasing vague (human-readable event) | challenger + codex + gemini (**3**) | P2 | **FIXED** in `7f71fca` — trigger now anchored to 3 concrete code artifacts (build_provider arm count / impl LlmProvider count / Cargo.toml deps) |
| F3 | `classify_output` drops stdout on non-zero exit | sec + perf + gemini (**3**) | P2 | **BACKLOG** — filed BL-040 (post-v0.0.1 polish; root cause of Step 4 debugging detour) |
| F4 | `build_provider` lacks BL-039 inline annotation | challenger (1) | P2 | **FIXED** in `7f71fca` — explicit NOTE pointing at BL-039 added |
| F5 | DB backup file mode 644 (`-rw-r--r--`) | security (1) | P2 | **FIXED** in `7f71fca` — `chmod 600` convention documented in spike doc |
| F6 | `CLAUDE_CLI_FLAGS` ↔ `_STRUCTURED_FLAGS` cross-reference | architecture (1) | P3→folded | **FIXED** in `7f71fca` — "Counterpart" cross-reference comment added |
| F7 | `WrapperEnvelope` version-bump verification checklist | architecture (1) | P2 | **BACKLOG** — filed BL-041 (on claude-CLI upgrade, re-probe wrapper shape before assuming structured-output works) |

### Structural recommendations (deferred to follow-up cycle)

These came in as convergent codex-proxy + gemini-proxy findings and
represent meatier follow-up work, not blocking for v0.0.1 verdict:

- **Schema discriminated-union retrofit** (codex Part B #3 + gemini
  Issue 1): current flat schema accepts `{skip: false}` without other
  fields (passes schema, fails `parse_synthesis_response` runtime).
  Discriminated-union shape via a `type: "synthesis" | "skip"` field
  would close the gap structurally — IF Anthropic input_schema accepts
  `if-then-else` conditional required (untested; BL would need a probe
  step).
- **Cross-validation quality heuristic** (gemini Issue 2): replace
  manual spot-check with objective "low-confidence synthesis" flag
  (e.g., `skip:false && entities.is_empty()` → flag for manual review).
- **Schema runtime tightening** (codex Part B #3 multi-pointed):
  require `skip` in runtime validation; require `reason` always
  (empty = no skip); add `minLength` on `title`/`content`, `minItems`
  on `entities`; stop accepting `{skip:true}` without reason.

These are post-v0.0.1 enhancements — not filed as BLs in this cycle
because each requires a small design discussion before implementation
(do not pre-commit to a specific shape).

## Disagreement Value Assessment

Three philosophical findings from challenger (MEDIUM confidence) where
reviewers disagreed in spirit but TL synthesized a position:

### DV1 — Karpathy boundary (challenger #1)

**Claim**: scanner was working (13 syntheses, 0 parse_errors in
production); plan 019 violates "don't refactor things that aren't
broken."

**TL position**: Accept the implementation; plan framing could have been
clearer. The replacement's value is generation-time token-decode
constraint + rate-limit observability, not parser-complexity reduction.
The plan's goal statement leaned on the latter framing, which earned
the Karpathy critique. Future plans of this shape should frame as
**capability addition** not **refactoring**.

### DV2 — production-run minimality (challenger #5)

**Claim**: real Pro tokens (275K, ~$0.40 USD-equivalent) for AC4
validation was over-validation; a 1-cluster smoke probe would have
caught the same Anthropic API surface.

**TL position**: Defensible as shipped. AC4 explicitly required a full
Dreaming pass to measure rate-limit behavior, which a single-cluster
smoke cannot do. Under Pro flat-fee subscription real spend is $0;
token-budget impact (~275K) is well under daily quota. The
"verify API contract" + "measure throughput" coupling in AC4 was
acceptable for v0.0.1. For future plans of this shape, consider
splitting into Pre-Step probe (1 call, schema verify) + Step N
production (multi-call, throughput measure).

### DV3 — version probe deletion over-corrected (challenger #6 + arch #8)

**Claim**: Doodlestein-regret + adversarial #3 convincingly killed the
`OnceLock`-cached startup probe (stale-cache foot-gun in long-running
daemon). But a non-cached construction-time probe at
`ClaudeCliProvider::from_config` time was never evaluated.

**TL position**: Accept the shipped state for v0.0.1. The error-message
hint ("verify claude >= 2.1.138 supports --json-schema") appears on
both `StructuredOutputMissing` and `StructuredOutputWrapperInvalid`
Display strings; an operator who upgrades to 2.1.138+ via Pro plan
auto-update never sees these errors. The non-cached construction-time
check would be strictly better-diagnosed but isn't worth a separate
BL for this scope. If the failure mode surfaces post-v0.0.1 in
practice (e.g., a user pins an older claude-CLI), revisit.

## Retrospective findings (recorded, not fixed in code)

Three process-level findings from cross-family proxies that don't map
to code changes but should inform future plan-review cycles:

### R1 — Plan-review must probe provider-specific schema assumptions

**Source**: codex-proxy PART A self-accountability + challenger #2.

The plan-review panel of 9 reviewers (architect, dep-analyst,
code-reviewer, sec-reviewer, codex-proxy, gemini-proxy + 3 Doodlestein)
endorsed the original `oneOf` schema design. None ran an executable
probe against the actual Anthropic API. Codex-proxy explicitly cited
OpenAI's `response_format: json_schema strict:true` precedent as the
basis for endorsement — which is true for OpenAI but doesn't transfer
to Anthropic's `input_schema` subset.

**Process change**: when a plan depends on provider-specific schema
behavior beyond plain object + types + required, plan-review MUST run
an actual API/CLI probe and cite the output. Citation alone is
sufficient for plain object schemas; probe required for
`oneOf`/`anyOf`/`allOf`/`const`/`additionalProperties:false`/
conditional required / wrapper shape / error-shape assumptions.

The mengdie `mengdie::ae:plan-review` skill should be updated to
include a "provider-contract probe checklist" before endorsement.

### R2 — Path C cost-doubling argument was phantom under flat-fee subscription

**Source**: gemini-proxy PART A.

Plan 019's "Out of scope" rejection of Path C (direct Anthropic HTTP
API) cited "~$0.24 per 40K-token call doubles cost vs CLI flat-fee
subscription". Under Claude Code Pro the actual subscription marginal
cost is $0 (tokens count against quota, not direct spend) — the cost
argument doesn't apply to the operator's deployment.

The real reason Path C was correctly rejected: it would have changed
the credential model from CLI-delegated to `ANTHROPIC_API_KEY`-in-
process, breaking the existing privacy posture. Future plans should
rationalize provider-choice on credential-model and operational
complexity, not phantom cost arguments.

### R3 — Path D rejection was over-cautious

**Source**: gemini-proxy PART A.

Plan 019's "Out of scope" rejection of Path D (codex-CLI as primary)
cited "no `--system` equivalent for context loading" and invoked
"avoid re-inventing wheels" thesis. Both true individually, but the
implementation arc shows the response was inverted-risk: mengdie now
carries 200 LoC of wrapper-parse code to extract claude-CLI's
structured output, vs. an estimated ~20 LoC of context-prepending
that codex-CLI would require.

**Note**: this is a retrospective observation. Reopening Path D is NOT
recommended (BL-039 sharpens the second-provider trigger appropriately).
The lesson is that "X is cleaner because Y" rejections should be
quantified, not just asserted.

## Outcome Statistics

```
Steps completed: 5/5 (Pre-Step + 4 numbered steps)
Rework rate: 1/4 steps needed fixup (Step 4 schema redesign mid-execution
  due to Anthropic API constraint = 25% rework rate; this is the
  single largest signal in the cycle)
P1 escape rate: 0 P1 findings in /ae:review — pre-commit caught
  everything that was caught
Drift events: 1 approved (resources/synthesis-output-schema.json
  added to Step 1 expected files via user-direct; operator-directed,
  recorded in commit message)
Fix loop triggers: 0 circuit breaker activations during /ae:work
Auto-pass rate: ~80% — 4 step transitions auto-continued; Step 4
  paused twice (once to confirm review mode, once after Anthropic API
  hit forced schema redesign)
Deferred resolution rate: 1/1 resolved (0 waived, 0 to backlog) at
  /ae:work close; review identified 2 new backlog items (BL-040,
  BL-041) and 1 sharpening on existing BL (BL-039)
```

The 25% Step rework rate is the single most informative number: it
quantifies the cost of the missing R1 process gate. A 9-reviewer plan
review that misses a provider-API constraint at plan-time forces a
mid-execution schema redesign — 6 commits + ~400 LoC committed before
the wall was hit. Recommendation: implement R1 as a hard gate in the
plan-review skill before another plan of this shape ships.

## Fixups squashed

Fixup commit `7f71fca` lands all P2 fixes directly on top of plan-019
execution commits — no rebase / autosquash performed (the fixup is
plan-019-scope-final-polish, not retroactive corrections to individual
plan-019 step commits). Branch state: `feature/v0.0.1-rebuild` =
`main` + plan 019 (10 commits) + this fixup commit (`7f71fca`).

```
0072548 docs(spikes): file 019 stdin-vs-argv reconnaissance (Pre-Step)
0b9bd76 feat(synthesis): file SYNTHESIS_OUTPUT_SCHEMA + anti-lazy-skip prompt (Step 1)
fae11f8 feat(llm): add ClaudeCliProvider::complete_structured (Step 2)
30a516c refactor(synthesis): delete brace-depth scanner (Step 3)
b30a8ac test(synthesis): add e2e fixtures + rate-limit instrumentation (Step 4 partial)
0b261db fix(synthesis): flat schema + skip:bool discriminator (Step 4 complete)
857ed3f docs(plan-019): final hash sync
1d5d72c docs(backlog): file BL-039
7f71fca fixup(plan-019): apply final-review findings  ← THIS REVIEW
```

## Backlog items filed

- `docs/backlog/unscheduled/BL-040-classify-output-drops-stdout-on-nonzero-exit.md`
- `docs/backlog/unscheduled/BL-041-wrapper-shape-version-bump-verification.md`

Existing `docs/backlog/unscheduled/BL-039-rig-extractor-revisit-when-second-llm-provider.md`
trigger phrasing sharpened in fixup commit.

## Next step

Plan 019 review complete. Branch ready for PR or merge per operator's
source-control workflow.

Suggested PR title: `Plan 019: synthesis CLI --json-schema adoption (BL-027 Path B)`.
PR body should include the fixup commit's body verbatim — it
documents both the final shape and the retrospective findings.
