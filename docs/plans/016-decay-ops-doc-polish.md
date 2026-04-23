---
id: "016"
title: "Decay ops doc polish — actionable threshold, rollback, AC5 correction note"
type: plan
created: 2026-04-23
status: done
discussion: "docs/discussions/021-v0.8.0-bl-dependencies/"
---

# Feature: Decay ops doc polish — actionable threshold, rollback, AC5 correction note

## Goal

Fill the three operator-experience gaps in `docs/operations/dreaming-decay.md` identified by `BL-decay-ops-doc-polish`: (1) make the approval-gate threshold computable via a copy-ready SQL + bash snippet instead of prose, (2) add a Rollback procedure for over-aggressive demotion incidents, (3) correct plan 013's AC5 regex language where it promised an arrow ASCII fallback that was never implemented.

## Background

Source: discussion 021 conclusion Topic 1 "Plan B" — split out from the decay-operator-surface bundle because `ops-doc-polish` edits `docs/operations/dreaming-decay.md` + one post-hoc note on plan 013; it does NOT share test infrastructure with the Plan A BLs (json-schema-contract + verify-decay-hardening, shipped in plan 015). Pure-docs scope, separate review surface.

BL body (`.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md`) lists 3 actions:

1. **Approval-gate threshold not actionable** — doc says "HALT if `decay_floor_breaches > max(10, 10% of long-term count)`" but provides no command to retrieve the long-term count.
2. **Arrow regex ASCII fallback promised, not delivered** — plan 013 AC5:194 documents the human-line regex as `(?:→|->)` but the test at `src/bin/cli.rs:723` asserts only the Unicode `→`. Current code emits only `→`. This is an AC-vs-code mismatch recorded as a BL for closure, not a code bug today.
3. **Missing rollback procedure** — no guidance on re-promoting memories if an over-aggressive demotion is detected post-fact.

**Decision on action 2 (resolved at plan-draft time, accepted risk)**: commit to Unicode-only `→` and correct the plan 013 AC5 text. The rejected alternative is **dual emission (both `→` and `->` side-by-side, or a switchable fallback)**. Rationale for rejecting the alternative: it would require a more complex format string + regex change across the test suite at `src/bin/cli.rs:719-736`.

**This is an accepted-risk decision, not a validated-robustness one**: there are real scenarios where `→` (U+2192, three UTF-8 bytes) can be dropped — SSH to a `LC_ALL=C` host, piping through certain `awk`/`sed` locales, logging systems that normalize to ASCII. To date Kai-the-operator has reported zero such incidents, but the risk exists and is not refuted by "no operator-reported issues" (n=1 operator in a solo-dev project). If the pipe-eating scenario ever materializes, the reversal is:
- Emit both arrows in `format_dreaming_line` (one-line change at `cli.rs:226`), AND
- Update the AC5 regex tests at `cli.rs:719-736` to `(?:→|->)`, AND
- Remove the plan 013 AC5 correction note added here.

The reversal is scoped and cheap. Accepting the risk now keeps plan 016 docs-only; the ASCII-fallback reversal is a 10-line diff whenever needed.

**Scope explicitly OUT** (per discussion 021 framing.md + plan-015 review P2.4 followthrough):
- No changes to `format_dreaming_line` or any code in `src/bin/cli.rs` (the arrow stays Unicode-only).
- No changes to `format_structured_json` or `docs/schemas/dreaming_pass.json` (the contract locked in plan 015 stays put; the `→` in the human line is distinct from any machine-contract field).
- No new tests **for arrow/Unicode behavior** — the existing AC5 regex tests at `cli.rs:719-736` already encode the Unicode-only reality and don't need modification. (Scope qualified per plan 016 Doodlestein strategic: Step 4 DOES add one new test `tests/ops_doc_sql.rs` for doc-embedded SQL drift guarding — that is in scope and load-bearing per the triple-confirmed test gap finding.)

## Steps

### Step 1: Add actionable threshold computation to the first-run procedure (AC1)

- [x] Edit `docs/operations/dreaming-decay.md` "Required first-run procedure" section (currently line 20-51). Expand step 3 with a copy-ready SQL query block that retrieves the **decay-eligible** long-term count (P1 fix from plan 016 review: the filter MUST match the decay pass's exact WHERE clause at `src/core/dreaming.rs:163-167`), plus a bash one-liner that pipes that count into the `max(10, 10% of count)` threshold computation.
- [x] The SQL block MUST use the exact filter `WHERE is_longterm = 1 AND valid_until IS NULL AND last_recalled IS NOT NULL` — all THREE conditions. The `last_recalled IS NOT NULL` guard is critical: memories with NULL `last_recalled` are permanently immune to demotion (skipped at `src/core/dreaming.rs:163-167`) and counting them in the denominator would inflate the threshold and make the approval gate less sensitive than intended.
- [x] Wrap the SQL block AND the bash helper in HTML-comment markers: `<!-- threshold-snippet:begin -->` and `<!-- threshold-snippet:end -->`. These are the **locked marker names** — any step/AC reference to "the threshold snippet" uses these exact tags. Both Step 1 and AC1 reference the same marker string; changing requires updating both.
- [x] The bash helper invokes `sqlite3 ~/.mengdie/db.sqlite "<query>"` and computes `max(10, count/10)` via shell arithmetic; display the computed threshold alongside the breach count so the operator can eyeball the comparison without mental math.
- [x] Preserve the existing step-3 prose (the HALT recommendation + recalibrate-floor option) — append the snippet, don't rewrite the step.

Expected files: `docs/operations/dreaming-decay.md`

### Step 2: Add Rollback procedure for over-aggressive demotion (AC2)

- [x] Insert a new `## Rollback: re-promoting a falsely-demoted memory` section into `docs/operations/dreaming-decay.md` **immediately after the "Required first-run procedure" section, before "Metric interpretation guide"** (roughly after line 51, before current line 53). Per plan 016 review (challenger blind spot): incident-response material belongs where operators first encounter the approval gate, not buried between historical Baseline data and the AE-parser audit footer.
- [x] Section contents, in this **stress-readable order** (codex P2 fix):
  1. **When to use** — "a demotion was noticed post-fact to be premature" with concrete symptom signals.
  2. **Required input**: the `breaches[]` array from the structured-JSON line of the offending pass. Name the field explicitly as it appears in `docs/schemas/dreaming_pass.json` — which is `breaches` (plan 015 Step 1). *NOTE: the same doc currently says `breached_ids` at line 65 — that field-name inconsistency must be fixed in this step too (codex P2): change `breached_ids` → `breaches` in the Metric-interpretation guide to match the actual JSON contract.*
  3. **Branch: if breach list is LOST (no captured stderr, no log file)** — surface this upfront so stressed operators see it before the happy path. Honest recovery procedure: (a) there is NO persistent dreaming output log by default; (b) demotion only writes `is_longterm = 0` — it does NOT set `valid_until`, `demoted_at`, or any audit timestamp (verified at `src/core/dreaming.rs:251-256`). So the `is_longterm = 0` set is only a superset of the recently-demoted memories (includes all historically-short-term rows too). Exact recovery requires external evidence: shell history (`history | grep mengdie dream`), terminal scrollback, or a redirected-stderr file if the operator captured one. If none exists, **the demotion is not row-level reversible** — the rollback path requires reconstructing the breach list from context (e.g., "memories I was actively recalling that day"). Surface this honest limit.
  4. **Rollback SQL (with breach list available)**: parameterized template `UPDATE memory_entries SET is_longterm = 1 WHERE id IN ('uuid-1', 'uuid-2', ...)`. **JSON→SQL quoting callout (architect P2)**: IDs in the `breaches[]` JSON array are double-quoted (`"abc123..."`). SQLite requires single quotes. The operator must strip the double quotes and re-wrap in single quotes; otherwise SQLite will see "unknown identifier" and silently do nothing (the UPDATE matches zero rows). Include a one-line `sed`/`tr` example that converts the JSON list to a SQL-valid list.
  5. **Verification query** to confirm the re-promotion took effect (`SELECT COUNT(*) ... WHERE id IN (<same list>) AND is_longterm = 1`).
  6. **`last_recalled` note**: this rollback does NOT touch `last_recalled`. The rolled-back memory will decay on the same schedule as before. An operator who wants to shield from immediate re-demotion must ALSO run `UPDATE ... SET last_recalled = <now>` — include a template.
  7. **Demotion-is-minimal-write note** (gemini P3 clarification): demotion only touches `is_longterm` (`src/core/dreaming.rs:252`). Setting `is_longterm = 1` is a complete reversal at the row level. No avg_relevance / last_recalled / valid_until state needs resetting beyond what items 5-6 already call out.

Expected files: `docs/operations/dreaming-decay.md`

### Step 3: Post-hoc AC5 correction note on plan 013 (AC3)

- [x] Edit `docs/plans/013-exponential-decay.md` with a one-paragraph "Post-ship correction (2026-04-23, plan 016)" note inserted **immediately after the AC5 heading at line 192** (architect P2 precision — NOT after line 201 where AC5 ends, which would bury the note inside AC6).
- [x] Note contents: the AC5 regex language at line 194 says `(?:→|->)` but the actual test regex at `src/bin/cli.rs:723` asserts only `→` (Unicode). The decision (made at plan 013 ship time, not recorded until now) is to keep the human line Unicode-only; **the rejected alternative was dual emission** (emitting both `→` and `->` side-by-side, which would have required format-string + regex-test changes). Unicode-only is an **accepted-risk decision** (see plan 016 "Decision on action 2, accepted risk"), not a validated-robustness one — the risk of pipe-eating in non-UTF-8 environments is real but unwitnessed in practice; reversal is a scoped 10-line diff if operationally triggered. The test at `cli.rs:723` is the source of truth; this note closes the documentation loop.
- [x] Do NOT modify line 194 itself — leave the historical AC text intact. Appending a post-ship correction note preserves the audit trail (what the plan said at ship time vs what the code actually does today). Amending a done plan's AC text in-place would rewrite history. (Dissent from plan 016 challenger C2 noted in the review record: "post-hoc corrections on done-plan ACs are graveyard prose nobody reads." Counter-argument per architect-approved pattern + the AE review-rules convention: audit-trail preservation is the explicit intent, narrow-audience readership is accepted.)
- [x] Close `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md` with `status: done` frontmatter + a "Shipped in plan 016" blockquote at the top of the body (matches plan 015's BL close-out pattern).

Expected files: `docs/plans/013-exponential-decay.md`, `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md` (local-only, `.ae/` gitignored)

### Step 4: Lightweight doc-SQL verification test (AC4)

Rationale (triple-confirmed by plan 016 review — challenger C3 + gemini P2.4 + codex P1.2 correction trail): the threshold SQL in Step 1 and the rollback SQL in Step 2 are **executable code pasted into docs**. Docs-only scope does not exempt them from silent-drift risk — wrong columns or typoed WHERE filters silently produce wrong counts or no-op updates. One lightweight test closes this risk without changing the plan's docs-centric nature.

- [x] Create `tests/ops_doc_sql.rs` — a new integration test (following the `CARGO_MANIFEST_DIR` + `tempfile::NamedTempFile` pattern from `tests/decay_contract.rs`).
- [x] Test parses `docs/operations/dreaming-decay.md`, extracts the content between `<!-- threshold-snippet:begin -->` and `<!-- threshold-snippet:end -->` (the locked markers from Step 1), then extracts the SQL query from the fenced code block inside that region (a single ```sql ... ``` block).
- [x] Test asserts the extracted SQL contains the required filter triple — `is_longterm = 1`, `valid_until IS NULL`, `last_recalled IS NOT NULL`. These are literal-substring checks, not regex — simple and robust against whitespace/newline variation.
- [x] Test seeds a tempfile SQLite DB with 3 rows: one decay-eligible long-term, one NULL-`last_recalled` long-term (immune), one invalidated (`valid_until` set). Opens a direct `rusqlite::Connection`, runs the extracted SQL, asserts the returned count is exactly 1 (only the decay-eligible row — confirms the filter is correct).
- [ ] If the Rollback section's SQL template (Step 2) is also wrapped in a similar marker pair (e.g., `<!-- rollback-snippet:begin -->` / `<!-- rollback-snippet:end -->`), add a parallel test: seed a demoted row, extract the template, substitute a test UUID, execute, assert `is_longterm = 1` post-update. **Deferred — non-blocking**: the threshold-snippet test is the primary drift guard (the denominator failure was the P1). The rollback-snippet template uses hardcoded example UUIDs (`abc-123`, `def-456`) that operators are instructed to replace; testing the literal template would either require a template-substitution helper or hardcoding the example UUIDs as test fixtures. Neither adds much value beyond "the UPDATE syntax parses" which rusqlite would catch at any real call site. Trade-off: 30-40 LOC of substitution-helper plumbing for marginal coverage. Skip per proportionality.

**Explicitly NOT doing** (per plan 016 Doodlestein regret): no cross-check against `run_dreaming_with_config`. That call would couple this docs-guard test to an internal `DreamingResult` field name (e.g., `avg_effective_score_before`) that is likely to evolve during active Phase 2 work (BL-residuals-reduction, synthesis). The 3-row fixture + count==1 assertion above already closes the filter-drift risk without the internal-API coupling. Keep the test docs-facing; if doc SQL drifts from code, the filter-substring check catches it at CI.

Expected files: `tests/ops_doc_sql.rs` (new)

## Acceptance Criteria

### AC1: Threshold snippet has locked markers + decay-eligible filter + bash helper (review-time verifiable)

**Verification** (all review-time static checks, no runtime DB access needed — replaces the earlier `eval`-based AC that was runtime-dependent per plan 016 review P1.3):

1. **Structural check**: `rg -n "threshold-snippet:begin|threshold-snippet:end" docs/operations/dreaming-decay.md` returns both markers exactly once, paired, inside the "Required first-run procedure" section.
2. **Filter check**: reviewer reads the content between the markers and confirms the SQL query includes ALL THREE conditions: `is_longterm = 1`, `valid_until IS NULL`, `last_recalled IS NOT NULL`. Missing the third condition is the P1.1 regression from plan 016 review.
3. **Bash helper check**: reviewer confirms a bash snippet using `sqlite3` is present, computing `max(10, count/10)` and printing both values.
4. **AC4's automated test** (Step 4) is the runtime guard — if Step 4 passes in CI, AC1's content checks transitively held at test time.

Visual doc-render check is the primary review-time gate; Step 4 catches drift.

### AC2: Rollback section is complete, correctly located, and actionable

**Verification** (all review-time static checks):

1. **Location**: section `## Rollback: re-promoting a falsely-demoted memory` exists in `docs/operations/dreaming-decay.md`, positioned **after "Required first-run procedure" and before "Metric interpretation guide"** (per plan 016 review challenger blind spot — incident-response material colocated with the approval gate, not buried after Baseline).
2. **Ordered content** (codex P2 stress-readable ordering): (a) when-to-use, (b) required input naming the field as `breaches` (not `breached_ids`), (c) **branch: "if breach list is LOST"** surfaced BEFORE the happy-path SQL, with honest recovery limits (no persistent log by default; demotion does NOT write `valid_until`/`demoted_at`/audit timestamps — verified at `src/core/dreaming.rs:251-256`), (d) rollback SQL template with JSON→SQL quoting callout (double-quoted JSON UUIDs must become single-quoted SQL literals, with one-line conversion example), (e) verification query, (f) `last_recalled` note + SQL template, (g) demotion-is-minimal-write clarification.
3. **Field-name consistency fix**: `rg -n "breached_ids" docs/operations/dreaming-decay.md` returns zero matches (the pre-existing inconsistency at line 65 of the doc is fixed to `breaches` in this step — codex P2).
4. **No false recovery paths**: `rg -n "valid_until" docs/operations/dreaming-decay.md` in the Rollback section is absent for the recovery path (the broken `valid_until timestamp range` hint is removed per plan 016 review P1.2).

Reviewer reads end-to-end and confirms stressed-operator flow works: input required → branch on breach-list-presence → SQL → verify.

### AC3: Plan 013 AC5 closure note + BL closure

**Verification** (all review-time checkable):
- `docs/plans/013-exponential-decay.md` has a "Post-ship correction (2026-04-23, plan 016)" paragraph **immediately after the AC5 heading at line 192, BEFORE the first AC5 bullet at line 194** (architect P2 precision — not after line 201 which would fall inside AC6).
- The correction note names the rejected alternative explicitly as "dual emission" (codex P2).
- Line 194 itself is preserved unchanged (audit-trail preservation per architect-approved pattern; challenger C2 dissent that post-hoc correction is "graveyard prose" noted in review record but not acted on).
- `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md` frontmatter has `status: done` and body has a "Shipped in plan 016" blockquote.
- No code changes to `src/bin/cli.rs` or `format_dreaming_line` — `git diff --name-only` on the plan-016 range returns only docs files + the new `tests/ops_doc_sql.rs` from Step 4 + (local) BL file.

### AC4: Doc-SQL verification test passes

**Verification**:
- `cargo test --test ops_doc_sql` passes.
- Test extracts SQL between the `<!-- threshold-snippet:begin -->` / `<!-- threshold-snippet:end -->` markers, asserts the three required filter conditions present (literal substring: `is_longterm = 1`, `valid_until IS NULL`, `last_recalled IS NOT NULL`), seeds a 3-row fixture DB (one decay-eligible, one null-`last_recalled` immune, one invalidated), runs the extracted SQL, asserts count == 1.
- Test does NOT call `run_dreaming_with_config` or assert on internal `DreamingResult` field names (plan 016 Doodlestein regret fix — avoids internal-API coupling that would break on Phase 2 metric evolution).
- Regression guard: if the doc's SQL filter is edited without matching the decay pass's semantics, either the filter-substring check fails (structural) or the fixture count fails (semantic). Both pin the regression to a specific line.

## Step Dependency Graph

```
Step 1 (threshold SQL) ─────┐
                            ├─→ Step 4 (test extracts snippet markers from Step 1,
Step 2 (Rollback section) ──┤                asserts filter + cross-checks decay pass)
                            └─→ (Step 4 also asserts rollback SQL if Step 2 adds
Step 3 (AC5 correction + BL close) — independent; edits plan 013 + (local) BL file
```

Serial execution order: **Step 1 → Step 2 → Step 3 → Step 4**.

Steps 1 and 2 both edit `docs/operations/dreaming-decay.md` but in disjoint sections — serializing them avoids `/ae:work` drift-detection complaints. Step 3 is independent (different files). Step 4 depends on Step 1's marker block existing (and optionally on Step 2's rollback marker block if the implementation adds one).

## Parallel Strategy

None — 3 small steps execute serially in `/ae:work`. No parallelism benefit.

## Out of Scope

- Any change to `format_dreaming_line` or `format_structured_json` (decision locked at plan-draft time per "Decision on action 2" above).
- Any new test coverage (existing AC5 regex tests at `cli.rs:719-736` already encode the Unicode-only reality).
- Any follow-up on BL-010 / `BL-decay-threshold-mode` — that's the other deferred BL, not this plan's scope.
- Any change to `docs/schemas/dreaming_pass.json` — machine contract is unchanged by this plan (the `→` arrow is in the human line, not the JSON fields).
- Synthesis cluster planning — discussion 021 Next Step 7 requires its own mini-discuss.

## Known Risk / Review Focus

- **Docs contain executable code**: Steps 1 and 2 add SQL and bash snippets that operators copy-paste against production data. Step 4's test is the automation safety net — without it, silent SQL drift (wrong filter, wrong column) produces wrong thresholds or no-op rollbacks. With Step 4, a CI failure pins the regression to a specific file:line. This is the triple-confirmed test gap from plan 016 review.
- **Threshold SQL filter parity with decay pass**: the filter MUST be `is_longterm = 1 AND valid_until IS NULL AND last_recalled IS NOT NULL` — all three conditions. Omitting the third inflates the denominator with null-`last_recalled` immune rows and makes the approval gate less sensitive. Grounded against the actual query at `src/core/dreaming.rs:163-167` (verified by plan 016 review dep-analyst + gemini). The existing SQL at `docs/operations/dreaming-decay.md:79-88` in the "Data freshness" section uses a similar filter and is a reference template.
- **Rollback SQL uses string IDs** (UUIDs per `docs/schemas/dreaming_pass.json`). Template MUST show single-quoted, comma-separated. JSON→SQL quote conversion: the `breaches[]` JSON array uses double quotes — operators copy-pasting directly without conversion will see SQLite match zero rows (silent no-op). Step 2 includes an explicit conversion-example (sed or tr one-liner) to close this failure mode.
- **Rollback "lost breach list" recovery is honest-limit, not false-hope**: original plan had a `valid_until`-based recovery hint that was structurally false (demotion does not write `valid_until` — verified at `src/core/dreaming.rs:251-256`). Step 2 now surfaces the honest limit: without a captured breach list, exact row-level rollback is not possible; recovery requires shell history, scrollback, or external log evidence. Fixed per plan 016 review P1.2.
- **Plan 013 AC5 note position**: insert **after line 192 (AC5 heading)**, before line 194 (first AC5 bullet). NOT after line 201 (which would land inside AC6). Architect P2 precision.
- **Unicode-only is accepted risk, not validated robustness**: see "Decision on action 2" in Background. The pipe-eating scenario is real but unwitnessed; reversal is a scoped 10-line diff if triggered.
