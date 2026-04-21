---
id: "009"
title: "BL-008 Challenger Review — Adversarial pass on commits 56812cb..HEAD"
type: review
role: challenger
created: 2026-04-20
scope: "docs/plans/013-exponential-decay.md, 14 files, +1764/-26 LOC"
---

# BL-008 Challenger Review

Adversarial pass. Pure opposition. TL synthesizes.

---

## C1 — AC5 structured-JSON was untested against actual stderr for 3 commits

**Claim**: The Step 4 implementation did not verify that `tracing::info!` produced a parseable bare-JSON line on stderr. The test suite only tested `format_dreaming_line` (the human-readable line). No test exercised the actual `eprintln!` / `tracing::info!` call or captured stderr.

**Evidence**:
- Step 4 commit `a6a3b02` added `format_dreaming_line` as a pure helper with 4 unit tests. The Step 4 step-summary explicitly states: "Capturing tracing output for JSON-line unit test. Requires a custom subscriber harness; skipped because the JSON is a `serde_json::json!` literal — its structure is guaranteed by the macro, not by ordering."
- The Step 4 commit emitted the JSON via `tracing::info!(target: "mengdie::dreaming", %structured, "dreaming_pass")` — the `%structured` format causes tracing's default formatter to produce `structured={"event":"dreaming_pass",...}` embedded inside a log-line envelope, not a bare JSON object.
- The fixup commit `32e11ef` confirms this was a P1 bug caught only via "accumulated-diff self-review" — not by any test.
- AC5 explicitly required: "Test parses the line with `serde_json`, asserts types and that `breaches.len() == decay_floor_breaches`." The test added in Step 4 calls `format_structured_json` (a pure in-process function), NOT the actual stderr emission path. The AC was satisfied on the formatting contract, but NOT on the transport contract.

**Objection**: The AC5 JSON tests added in `32e11ef` (`format_structured_json_parses_with_all_required_fields`, `format_structured_json_breaches_array_length_matches_decay_floor_breaches`) now cover the serialization round-trip. The fixup was self-caught and corrected before merge-to-main.

**Counter-objection**: Both of these tests were added in the fixup commit, not in Step 4. They test `format_structured_json(...)` — a pure function that was also introduced in the fixup commit. The AC5 test suite that shipped with Step 4 still has no process-level test confirming the JSON lands on stderr as a parseable line. If someone changes `eprintln!` back to a `tracing::info!` call in the future, the existing unit tests will continue to pass.

**Confidence**: HIGH. The process-level gap is structural — the AC5 tests verify the formatter but not the emitter. A future `tracing::info!` regression is undetectable by the current test suite.

---

## C2 — `verify-decay.sh` has no CI coverage; a JSON-emission regression silently breaks it

**Claim**: `scripts/verify-decay.sh` is an untested shell script. If the structured-JSON emission is changed (e.g., back to tracing, or keys renamed), the script silently degrades: it falls through to the "WARNING: could not parse structured JSON line" branch and exits 1 (or 0 with `--i-reviewed-each`), falsely indicating no review was possible rather than a breach count.

**Evidence**:
- Step 5 step-summary explicitly rejected: "A unit test for `verify-decay.sh`'s JSON extraction. Shell script testing is out of scope."
- The script's fallback on JSON parse failure (`[[ -z "$JSON_LINE" ]]`) exits 1 only when `APPROVED=0`. When `APPROVED=1` (the expected operator flow for zero-breach runs), it exits 0 unconditionally, meaning a broken JSON emission path is invisible to the operator who ran `--i-reviewed-each`.
- `scripts/verify-decay.sh` lines 64-73: if `JSON_LINE` is empty AND `APPROVED=1`, the script prints a warning and exits 0 — the same exit code as a successful zero-breach pass.
- The approval gate's entire purpose is to catch `decay_floor_breaches > 0`. If JSON parsing silently fails, the gate never fires regardless of actual breach count.

**Objection**: The script's grep pattern `^\{.*"event":"dreaming_pass".*\}$` is now correct for the bare-JSON `eprintln!` output. The only way to regress this is to change the emission path.

**Counter-objection**: The script has no test enforcing that the emission path stays a bare `eprintln!`. The step-summary decision to skip shell script testing means there is no CI checkpoint. Corpus drift, daemon integration (BL-010), or a future refactor could silently break the JSON parse — and the script would continue to exit 0 on zero-breach days without anyone noticing the gate is dead.

**Confidence**: MEDIUM-HIGH. The immediate risk is low (emission path is now correct). The structural risk is that a non-obvious regression path exists with no automated detection.

---

## C3 — Production smoke run was self-administered; the operator procedure's own gate was bypassed

**Claim**: The `docs/operations/dreaming-decay.md` procedure states that `scripts/verify-decay.sh` "MUST be run before first live `mengdie dream` post-ship." The TL ran this inline during `ae:work` Step 5, meaning the person who wrote the procedure is the same person who ran it, moments after writing it. This is not independent verification.

**Evidence**:
- The fixup commit `32e11ef` message states: "Production smoke result (ran `scripts/verify-decay.sh` against the actual `~/.mengdie/db.sqlite` at 2026-04-20): 0 would-demote."
- The procedure was written in Step 4 commit `a6a3b02`. The script was written in Step 5 commit `522db8a`. The smoke run happened in the fixup commit `32e11ef` — the same commit that fixed the script's grep pattern after discovering it was broken.
- The baseline was recorded in `docs/operations/dreaming-decay.md` in that same fixup commit.

**Objection**: The run produced valid output (breach_count=0, avg_effective_before=0.4712), which was correct and matches the archaeologist V3 prediction. The procedure's goal was to confirm the corpus was safe before live mutation. That goal was achieved.

**Counter-objection**: The procedure says "MUST be run before first live `mengdie dream`." The script's grep pattern was broken at the time of Steps 4 and 5. The TL fixed the grep pattern and ran the script in the same commit. The sequence was: (1) discover script is broken, (2) fix script, (3) run script, (4) declare AC7 closed. The spirit of the AC7 gate — running an independent verification before declaring safety — was satisfied by self-review, not independent verification. The script itself was modified in the same commit as the "passed" run.

**Confidence**: MEDIUM. The corpus result is correct. The procedural gap is real but the outcome is not invalidated.

---

## C4 — Dry-run regex `\d+\s+would-demote\s+\(DRY RUN\)` is tested against a mock fixture, not actual output bytes

**Claim**: The dry-run unit tests in `cli.rs` test `format_dreaming_line` against a `DreamingResult` fixture — they do not verify the exact byte sequence that `cmd_dream` produces. The format string and the regex can diverge.

**Evidence**:
- `format_dreaming_line` at `src/bin/cli.rs:230-242` produces: `"{count} would-demote (DRY RUN)"` — single spaces around the phrase, ASCII parentheses, UTF-8 `→`.
- The AC5 regex: `\d+\s+would-demote\s+\(DRY RUN\)` — `\s+` matches one or more whitespace chars of any kind, `\(` matches literal `(`.
- `format_dreaming_line_dry_run_matches_ac5_regex` tests the regex against `format_dreaming_line(&sample_result(0, 3, 0.421, 0.421), true)` — a fixture with `decay_floor_breaches=3` and `demoted=0`. This is correct.
- The test does verify the actual format string because it calls the same pure helper. The risk is that `cmd_dream` produces the line via `println!("{}", format_dreaming_line(...))` — so it goes to stdout, not stderr. The `→` arrow is UTF-8 `→`. If a terminal, log scraper, or test harness converts the output to ASCII, the arrow becomes unmatched by the regex `(?:→|->)` — wait, the regex in the AC tests only uses `→`, not `->`.

**Evidence (arrow variant)**: The AC5 regex in the live test is:
```
\d+\.\d+\s+→\s+\d+\.\d+
```
The regex only allows `→` (UTF-8 U+2192). If the output is piped through a tool that strips non-ASCII or if the format string is changed to `->`, the regex would fail. The plan's AC5 describes the regex as `(?:→|->)` with the ASCII alternative, but the implemented test regex does NOT include the `->` alternative.

**Objection**: This is an internal unit test for an internal helper. The format string and regex are in the same file; a developer changing the format string would see the test.

**Counter-objection**: The discrepancy between the plan's documented AC5 regex (`(?:→|->)`) and the implemented test regex (only `→`) is a spec/impl gap. The implemented regex is stricter than the plan intended, meaning a future change from `→` to `->` would break both the regex and the AC5 test simultaneously — which is good, not bad. However: the plan said "loose regex — tolerates whitespace + decimal-place variation so safe operator-output improvements don't break the AC." The arrow character is not a "safe operator-output improvement" — it is a rendering choice that could legitimately change (e.g., for terminal compatibility). The plan promised robustness it did not deliver on this axis.

**Confidence**: LOW-MEDIUM. Not a bug today. The `->` omission is a spec/impl gap, not an active failure.

---

## C5 — Doodlestein cross-family checkpoint was replaced by inline self-review

**Claim**: Per the plan 013 `work.accumulated_doodlestein` invariant, cross-family Doodlestein proxies should have fired at final step (Step 5 of 5). Instead, the TL performed a self-review inline during the fixup commit. The intent of Doodlestein is a different model family seeing the full accumulated diff — the same agent that wrote the code is not a different family.

**Evidence**:
- The fixup commit message says "Accumulated-diff self-review at plan completion caught a P1 bug in the Step 4 stderr JSON emission path."
- The P1 bug was found — demonstrating the review was productive.
- The Doodlestein skill's intent: cross-family agent reviews the accumulated diff to surface blind spots the implementing family missed. The implementing TL found a bug in their own code, but this is not the same as independent cross-family review.

**Objection**: The self-review was triggered by the same mechanism (accumulated-diff review at Step 5), found a real P1 bug, and produced a fixup commit. The outcome — code correctness before merge — was achieved.

**Counter-objection**: The bug found (`tracing::info!` wrapping JSON) is exactly the class of error a fresh cross-family reader would catch: "wait, does tracing format this field as bare JSON or wrapped?" A Claude-family TL writing the code knew the intent and wrote the emission path that way without questioning the transport format. A Codex or Gemini family reader, looking cold at the code, is more likely to ask "does `%structured` in a tracing macro emit raw JSON?" The self-review found the bug because the TL re-read the code with fresh eyes, not because cross-family perspective was applied. For a plan that explicitly schedules cross-family Doodlestein, inline self-review is a substitution, not an equivalent.

**Confidence**: MEDIUM. The immediate risk was addressed. The process gap is structural — inline self-review cannot replace cross-family perspective in the general case, even if it happened to work here.

---

## C6 — `breached_ids: Vec<String>` is allocated every pass, including zero-breach passes; no short-circuit

**Claim**: Every call to `run_dreaming_with_config` allocates a `Vec<String>` for `breached_ids`, fetches all long-term memories from the DB, and iterates over them to compute effective relevance — even when the result is zero breaches. There is no early-exit path.

**Evidence**:
- `src/core/dreaming.rs:190`: `let mut breached_ids: Vec<String> = Vec::new();`
- The SELECT at line 163 runs unconditionally: all long-term memories are fetched into `longterm_rows: Vec<(String, f64, String)>`.
- The iteration loop at line 191 runs over every row, parsing timestamps and computing effective relevance.
- No short-circuit exists. Even with zero long-term memories, the SELECT, Vec allocation, and loop execute.

**Objection**: This is not a real performance issue. The typical corpus has tens of memories, not millions. A `Vec::new()` is a pointer/length/capacity triple — no heap allocation until `.push()`. The SELECT is the only IO, and it's already necessary to compute the before-mean.

**Counter-objection**: The TL's attack surface question was specifically about the daemon path (BL-010): "when the daemon ships, this vector is allocated per pass regardless of consumers." The stronger concern is not the Vec but the SELECT + full row fetch. In a production DB with 10,000 long-term memories, the demotion pass fetches all 10,000 rows into memory every daily run, even if none breach the floor. There is no `WHERE effective_relevance < floor` predicate possible in SQL (decay is a Rust-time computation), but the current code makes no attempt to SHORT-CIRCUIT even the aggregate stats when the corpus is known-fresh (e.g., `AND last_recalled > <cutoff>` for a safe zone). The per-pass cost scales linearly with long-term corpus size with no mitigation.

**Confidence**: MEDIUM. Not a bug for current corpus size. Becomes load-bearing when the daemon runs daily against a corpus with O(thousands) of long-term memories — which is the stated Phase 2 direction.
