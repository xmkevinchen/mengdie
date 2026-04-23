---
id: "014"
title: "Review: decay ops doc polish (plan 016)"
type: review
created: 2026-04-23
target: "docs/plans/016-decay-ops-doc-polish.md"
verdict: pass
---

# Review: Plan 016 — Decay Ops Doc Polish

## Scope

Deep review of 7 commits in `fde945f..HEAD` landing plan 016 (discussion 021 Topic 1 "Plan B"). Covers:

- `docs/operations/dreaming-decay.md` — threshold snippet + Rollback section + `breached_ids` → `breaches` cleanup
- `docs/plans/013-exponential-decay.md` — post-ship AC5 correction note (blockquote, line 192)
- `tests/ops_doc_sql.rs` — new, 2 drift-guard tests (structural + semantic)
- `docs/plans/016-decay-ops-doc-polish.md` — plan checkbox updates
- `docs/milestones/016/step-summaries.md` — new, 4 step summaries
- `.ae/backlog/v0.8.0/BL-decay-ops-doc-polish.md` — local-only BL close-out

## Team

| Agent | Angle | Backend |
|-------|-------|---------|
| code-reviewer | Rust idioms, test quality, shell/SQL correctness, doc accuracy | Claude |
| challenger | pure opposition: did the plan-time 3 P1s actually land? operator-facing failure modes | Claude |
| codex-proxy | cross-family: implementation correctness — doc↔code alignment, filter parity, rollback inversion | OpenAI Codex (high reasoning) |
| gemini-proxy | cross-family: edge cases + operator-facing risks (bash/jq robustness, doc portability, test robustness) | Google Gemini → local gemma-4-26b-a4b-it-4bit (fallback on quota) |

## Findings Summary

| Severity | Count | Resolution |
|----------|-------|------------|
| P1       | 0     | — |
| P2       | 6     | all fixed in `2d81576` |
| P3       | 2     | 1 fixed (challenger blind spot), 1 cosmetic added (Gemini Q5 marker comment) |
| "No issue" | 7   | Codex Q2-Q6 all passed + architecture/security/dep-analyst parity from plan-review |

## P1 — None

The plan-review team's 3 P1s (threshold filter missing `last_recalled IS NOT NULL`, rollback `valid_until` false recovery, AC1 eval-runtime-dependent) were all applied before Step 1 kicked off. Challenger independently verified all three are genuinely fixed in the implementation, not just verbally addressed:
- Threshold filter triple present and enforced by `tests/ops_doc_sql.rs` at the substring level
- Rollback section's "lost breach list" branch documents honest limits, no `valid_until` recovery path
- AC1 review-time verifiable — Step 4's test runs hermetically against a tempfile DB; no runtime `~/.mengdie/db.sqlite` access required

## P2 — Fixed

### P2.1 — Broken jq quote-escaping (code-reviewer)

**Finding**: the Step 2 jq one-liner `jq -r '.breaches | map("'" + . + "'") | join(", ")'` produced garbage output (`+ . + , + . +`) — the quote-sandwich doesn't survive bash parsing inside a single-quoted jq expression.

**Fix** (`2d81576`): replaced with idiomatic `jq -r '.breaches | map(@sh) | join(", ")'`. jq's `@sh` filter handles single-quote wrapping correctly. Verified against `{"breaches":["abc-123","def-456"]}` fixture — output is now `'abc-123', 'def-456'`.

### P2.2 — sqlite3 non-numeric response masks root cause (Gemini P2.1)

**Finding**: threshold bash helper blew up with "invalid arithmetic operator" when sqlite3 emitted anything non-integer (e.g., "file is not a database"). Cryptic error masked the real problem.

**Fix** (`2d81576`): added a regex guard `if ! [[ $count =~ ^[0-9]+$ ]]; then echo "error: sqlite3 returned non-numeric output: '$count'" >&2; exit 1; fi` immediately after the sqlite3 invocation. Clear error naming the actual bad input now.

### P2.3 — JSON trust-source assumption undocumented (Gemini P2.2)

**Finding**: the rollback SQL splices breach IDs into the UPDATE without defensive escaping. Safe because IDs come from `DreamingResult` (Rust, UUIDs), but this trust assumption wasn't stated. A paranoid operator hand-editing JSON could hit the unsafe path.

**Fix** (`2d81576`): added a "Trusted source only" callout to the Rollback section explicitly stating that the `breaches` array must come from a real `mengdie dream` invocation.

### P2.4 — Test doesn't enforce single-block invariant (Gemini P2.3)

**Finding**: `tests/ops_doc_sql.rs` uses `.find("```sql")` to extract the first fenced block between markers. If a reviewer adds a second ```sql block for clarity, the test silently extracts the wrong one.

**Fix** (`2d81576`): added `assert_eq!(section.matches("```sql").count(), 1, ...)` before the extraction. Future multi-block edits fail loudly.

### P2.5 — "Exactly matches" filter overclaim (Codex P2.1)

**Finding**: the doc prose claimed the threshold SQL "exactly matches" `dreaming.rs:163-167`, but the actual decay pass has `{project_filter_simple}` appended when `--project <id>` is passed. The static predicates match; the claim was overbroad.

**Fix** (`2d81576`): softened to "the three primary predicates match the decay pass's static filter" + added a parenthetical explaining the project-scope addition. Accurate without losing the key information.

### P2.6 — `breached_ids` cleanup incomplete (Codex P2.2)

**Finding**: plan 016's Step 2 AC called for `rg -n "breached_ids" docs/operations/dreaming-decay.md` returning zero matches. Two occurrences remained as "historical context" explanatory notes — not misleading operators, but didn't meet the literal AC.

**Fix** (`2d81576`): removed both occurrences. Rust struct field is still named `breached_ids` internally (renamed to `breaches` via serde on output); the trust-source callout now just says "Rust-populated UUIDs emitted by the binary" instead of naming the struct field. `rg breached_ids dreaming-decay.md` returns zero matches — AC literally met.

## P3 — Fixed + Noted

### P3.1 — Symmetric zero-result silent failure (Challenger blind spot)

**Finding**: the rollback UPDATE template and verification SELECT both use hardcoded example IDs `abc-123`/`def-456`. An operator pasting both without substituting real UUIDs sees both "succeed" with zero rows and no error — operationally indistinguishable from "rollback complete" for someone unfamiliar with SQLite's silent-zero-update behavior. The plan's Step 4 explicitly deferred the rollback template test; this was the blind spot the deferral didn't cover.

**Fix** (`2d81576`): added `SELECT changes();` immediately after the UPDATE template, plus a prose callout above: "if it returns 0 you either forgot to substitute the example IDs or the IDs aren't in the corpus. A successful rollback of N memories should print `N`." Converts silent failure to loud signal at zero code cost.

### P3.2 — Rollback-snippet markers undocumented (Gemini Q5)

**Finding**: the `<!-- rollback-snippet:begin -->` / `end` markers in the doc aren't consumed by any test or automation (unlike the threshold-snippet markers, which `ops_doc_sql.rs` validates). Reader sees them and wonders if they're dead weight.

**Fix** (`2d81576`): added a `<!-- Reserved: future automation may validate this UPDATE template against a fixture DB — see plan 016 Step 4 deferred bullet. -->` comment inside the `rollback-snippet:begin` marker. Documents intent.

## Dissent Preserved

- **Challenger ATTACK 3** (late summary commit `ea81084` is "mildly untidy"): challenger's own confidence was LOW; style issue only, no action. Preserved in plan history.
- **Challenger ATTACK 4** (plan 013 AC5 correction is "graveyard prose nobody reads"): cost-benefit dissent preserved from plan-review phase. The correction does appear before the original AC line 194 in the rendering, so a reviewer opening AC5 sees it first — but the challenger's cost-benefit critique of the whole exercise stands as noted commentary. No action.

## Codex "No Issue" Block (7 checks passed)

- **Q2** (bash helper arithmetic pathological cases): count=0 → 10 stable, count=99 → 10 with correct crossover at count=110. ✓
- **Q3** (rollback inversion): `is_longterm = 1` exactly inverts demotion's `is_longterm = 0`. No other columns need resetting. ✓
- **Q4** (test guard strength): single-edit mutation causes both structural and semantic failures. ✓
- **Q5** (AC5 correction placement): blockquote rendered before historical bullets; audit trail + first-line visibility both achieved. ✓
- **Gemini Q4** (blockquote markdown portability): portable across GitHub, GitLab, mdBook, Pandoc. ✓

## Outcome Statistics

```
Steps completed: 4/4
Rework rate: 0/4 (no mid-work fixup commits)
P1 escape rate: 0 (plan-review + /ae:work pre-commit caught everything)
Drift events: 0 Check B drift during /ae:work (all steps matched Expected files)
Fix loop triggers: 0 circuit breaker activations during /ae:work
Auto-pass rate: 4/4 steps auto-continued
Deferred resolution rate: N/A (no `notes.md` DEFERRED entries)
```

**P1 escape rate: 0** — the 3 P1s caught at plan-review (+6 P2s) landed pre-merge. The 6 P2s caught at /ae:review are all new findings (not plan-review misses) — 1 genuine bug (broken jq), 1 scope-creep fix (`breached_ids` cleanup), 3 defense-in-depth hardening (sqlite3 guard, trust note, single-block assertion), 1 wording-precision (filter overclaim).

**Interpretation**: the plan-review process front-loaded the correctness-critical findings; the execution-review surfaced the implementation-detail findings that only manifest in actual code. This is the expected pattern.

## Fixups Landed

One consolidated fixup commit `2d81576` per project convention (main-branch /ae:review fixups). 8 fixes in one commit. One fixup commit instead of 6 `--fixup=<hash>` + autosquash keeps the fixup scope reviewable atomically and matches the plan-015 review convention.

## Deferred Findings Audit

No `docs/milestones/016/notes.md` was created during `/ae:work` — no findings were deferred mid-step. Audit: **no unresolved deferred findings**.

## Verdict

**PASS** — 0 P1, 6 P2 all fixed, 2 P3 (1 fixed, 1 cosmetic). Plan 016 ships as-is.

## Knowledge Capture Targets

Three review-pattern memories to ingest (plan 016 review yielded more reusable discipline than one-off bug fixes):

1. **jq single-quote wrapping**: avoid the bash `'"'"'` sandwich inside jq expressions; use `jq -r 'map(@sh)'` — jq's own filter handles SQL-compatible single-quoting correctly and reads cleanly.
2. **Symmetric-zero-result silent failures in incident-response docs**: when the rollback SQL and verification query share hardcoded example values, a no-substitution operator sees both "succeed" identically. Always add a loud signal (`SELECT changes();` for SQLite, or equivalent row-count check) to incident-response templates.
3. **Sqlite3 stdout-in-variable bash patterns need numeric guards**: when piping `sqlite3 ... "SELECT COUNT(*)..."` into a bash var, follow with `if ! [[ $var =~ ^[0-9]+$ ]]; then exit 1; fi` — otherwise error messages leak into arithmetic and produce cryptic failures.
