---
agent: archaeologist
round: 2
date: 2026-04-23
---

# Archaeologist Findings — Round 2

## New Fact Verification

### 1. What `/ae:roadmap remove` actually does to the BL file

Source: `/Users/ckai/Workspace/Projects/agentic-engineering/plugins/ae/skills/roadmap/SKILL.md`
Section: `/ae:roadmap remove <BL-ID>` (line ~500-510 of SKILL.md).

**Exact behavior** (quoting the spec):
1. Locate the BL file in any backlog subdir.
2. If already in `unscheduled/` → no-op.
3. If source is an ACTIVE sprint, require `--reason "..."` (scope-lock).
4. `mv .ae/backlog/<source>/BL-<ID>-*.md .ae/backlog/unscheduled/` — plain `mv`.
5. Log to source's `## Notes`: `YYYY-MM-DD | descope | BL-<ID> | <reason>`.
6. Output: `Removed BL-<ID> from <source>. Now unscheduled.`

**Key facts for the discussion**:
- **Non-destructive**: the BL file body is fully preserved. It is moved, not deleted.
- **Destination is `unscheduled/`**, not a `closed/` or `.removed/` archive — the file
  stays in the active backlog, reachable via `ls .ae/backlog/unscheduled/`.
- **Notes entry logged**: a `descope` action entry is appended to v0.8.0's `## Notes`,
  creating a permanent audit trail. Format: `YYYY-MM-DD | descope | BL-ID | <reason>`.
- **`--reason` required** for active-sprint descope (v0.8.0 is active, so this applies).
- **Discoverability**: items in `unscheduled/` remain visible in default `/ae:roadmap`
  output — SKILL.md says unscheduled items are read in default view (table: "Yes —
  default view").

**Answer to minimal-change-engineer's open question**: `/ae:roadmap remove` does NOT
block or delete. The BL file moves intact to `unscheduled/` with a `--reason` audit
entry. The "do nothing" position therefore has no hard tooling blocker — the close
behavior is warn-by-default, not refuse.

### 2. `/ae:roadmap close` behavior with open items — does it block?

Source: SKILL.md `/ae:roadmap close <version>` section, step 4 (line ~449-453).

**Exact behavior**: the close subcommand uses **warn-by-default**:
> For each item whose `status` is NOT done/closed, emit:
> `⚠ BL-NNN (status: <value>): not marked done — closing anyway.`
> Close proceeds. `--strict` flag escalates to refusal.

**Conclusion**: default close does NOT refuse on open items — it emits a warning per
item and proceeds. `/ae:roadmap close v0.8.0` (without `--strict`) would succeed even
with both defer-until-trigger BLs still open, logging one warning line per open item.

This directly resolves minimal-change-engineer's one open question ("if the skill
REFUSES to close with open items, my 'do nothing' position is wrong") — the skill does
NOT refuse by default. The "do nothing" position is tooling-compatible.

### 3. v0.8.0 gate language and count coherence after Topic 2 removal

Source: `.ae/roadmaps/v0.8.0.md`, lines 16 and 30-34.

**Gate frontmatter (single line)**: `"All 7 review-originated follow-ups closed, CI
runs cargo clippy + cargo test on PR (not just fmt)"`

**Gate body (expanded)**:
1. All 4 `BL-decay-*` items closed — no open P2 residuals from plan 013 review.
2. All 3 open `BL-synthesis-*` items closed — no open residuals from plans 010/012 reviews.
3. `ci.yml` runs `cargo clippy --all-targets -- -D warnings` and `cargo test` on PR.
4. No regressions in existing review history (008/010/011 still PASS after fixes).

**Current actual item counts** (from `.ae/roadmaps/v0.8.0.md` Items table):
- v0.8.0 has 9 items in `initial_items`, NOT 7. Two were already closed (plan 014):
  `006-ci-runner-env-cleanup` and `BL-ci-full-clippy-test`. That leaves 7 open.
- Of those 7, the gate says "All 4 BL-decay-*" — there are exactly 4 BL-decay-* items
  in initial_items: `BL-decay-dreaming-pass-optim`, `BL-decay-json-schema-contract`,
  `BL-decay-ops-doc-polish`, `BL-verify-decay-script-hardening`.
- "All 3 BL-synthesis-*": exactly 3 in initial_items: `BL-synthesis-dedup-key`,
  `BL-synthesis-preload-db-miss-edge`, `BL-synthesis-provenance`.

**Impact of `/ae:roadmap remove` on the 2 trigger-gated items**:

If `BL-decay-dreaming-pass-optim` and `BL-synthesis-preload-db-miss-edge` are removed:
- Sprint dir would contain 7 remaining items (9 initial − 2 already closed − 2 removed... 
  wait: 9 initial, 2 closed by plan 014 = 7 open today, remove 2 = 5 remain open in sprint).
- Gate condition 1 ("All 4 BL-decay-*") — with `BL-decay-dreaming-pass-optim` removed,
  only 3 decay items remain in sprint. The gate says "4" but only 3 are there. The gate
  text becomes factually broken unless updated.
- Gate condition 2 ("All 3 BL-synthesis-*") — with `BL-synthesis-preload-db-miss-edge`
  removed, only 2 synthesis items remain. Gate says "3" but 2 are there. Also broken.

**Challenger's observation is correct**: if Topic 2 removes both items, the gate counts
become incoherent. Gate body line 1 says "All 4 BL-decay-*" but only 3 would exist in
sprint; line 2 says "All 3 BL-synthesis-*" but only 2 would exist.

**However**: the gate frontmatter single-line says "All 7 review-originated follow-ups
closed" — if the sprint drops to 5 open items (after removal), this number is also
wrong. The SKILL.md scope-delta mechanism (`close-scope-delta` Notes entry) would record
the removal but does NOT automatically update the gate text — gate text is user-owned
per the spec (`## Gate — user-owned. Definition of Done. ae:roadmap renders from
frontmatter gate: initially; user may expand.`).

**Conclusion**: removing items requires manually updating the gate text (in SKILL.md's
terms, the gate is user-owned and not auto-regenerated). This is a low-friction one-line
update, but the discussion should acknowledge it as a required housekeeping step — not
a reason to avoid removal.

### 4. Was the "proceeding anyway" branch in verify-decay.sh at any commit?

**Answer: yes, it existed from the very first commit (`fd910e3` — BL-008 Step 5) and
still exists in the current script.**

Git log for `scripts/verify-decay.sh` shows exactly 2 commits:
- `fd910e3` — "BL-008 Step 5: e2e smoke + verify-decay approval gate" (original ship)
- `e882be9` — "BL-008 post-ship fixup: bare-JSON stderr + script parse + baseline log"

The diff between them (`git diff fd910e3 e882be9 -- scripts/verify-decay.sh`) shows the
fixup changed ONLY the grep pattern for extracting the JSON line (3 lines deleted, 6
inserted — updated the comment and the regex). The `--i-reviewed-each`/`APPROVED=1`
logic and the "proceeding anyway" branch were NOT touched in the fixup.

**Current state** (scripts/verify-decay.sh:64-73, both commits):
```bash
if [[ -z "$JSON_LINE" ]]; then
  echo "WARNING: could not parse structured JSON line..." >&2
  echo "Raw stderr:" >&2
  cat "$TMP_ERR" >&2
  if [[ $APPROVED -eq 1 ]]; then
    echo "(Proceeding anyway: --i-reviewed-each was passed.)" >&2
    exit 0
  fi
  exit 1
fi
```

So the BL body's claim that action 1 needs to "clarify the 'proceeding anyway' branch
so it's harder to bypass silently" is NOT stale — that branch exists in both historical
and current versions. My Round 1 finding ("no 'proceeding anyway' path") was WRONG.

**Correction of Round 1 error**: The binary preflight block (lines 35-38) exits 2 on
missing binary, which is correct. But the "proceeding anyway" language in the BL refers
to the DIFFERENT branch at lines 64-73 — the fallback when the JSON line can't be
parsed. In that branch, if `--i-reviewed-each` is passed, the script exits 0 even
though it couldn't validate the JSON. The BL is asking for clarification of THIS
silent-bypass path. Action 1 ("clarify the proceeding anyway branch") is a separate
concern from the binary preflight, not a duplicate.

**Revised assessment of action 1**: the action is real work, not already done. The
binary preflight at lines 35-38 is done. But "clarify the proceeding anyway branch"
refers to the JSON-parse-failure fallback at lines 64-73, which the BL considers too
permissive: passing `--i-reviewed-each` silently succeeds even when no structured JSON
was parsed — meaning the approval was based on zero breach data. Action 1 = ~3-5 lines
to make this case more visible (e.g., print a louder warning before exiting 0, or
require a second confirm flag on the fallback path).

**Updated sizing for Topic 3**: Actions 1, 2, 3, 4 are all real work.
- Action 1: ~3-5 LOC (fallback branch clarity, not binary preflight)
- Action 3: STILL ALREADY DONE at line 47 (`RUST_LOG="${RUST_LOG:-info}"`) — present
  in BOTH commits. The BL body is incorrect/stale on action 3.

Net revised: actions 1, 2, 4 are work; action 3 is already done; action 5 is deferred.

---

## Responses to Peer Positions

### On minimal-change vs. architect (Topic 1: "2+1 split" vs "bundle all 3")

**Evidence supports minimal-change-engineer's 2+1 split over the architect's 3-bundle,
but for a different reason than MCE stated.**

The file-touch pattern I verified in Round 1:
- `BL-decay-json-schema-contract` edits `format_structured_json` (cli.rs:207-222).
- `BL-decay-ops-doc-polish` edits `format_dreaming_line` (cli.rs:226-243) — DIFFERENT function.
- `BL-verify-decay-script-hardening` edits only `scripts/verify-decay.sh`.

The two functions that appear to overlap (both in cli.rs) are 19 lines apart and have no
shared variables or test infrastructure. The "same-file" coupling in a 3-bundle does not
represent any actual merge risk.

MCE's real argument is correct: the shared test harness only couples json-schema-contract
+ verify-decay-hardening (both spawn `mengdie dream --decay-dry-run` and parse stderr).
ops-doc-polish has no test interaction at all — its 3 actions (SQL snippet, arrow fix,
rollback section) are pure documentation plus one trivial format string change. Bundling
it into the code+test plan:
- Adds review scope without review benefit (docs review and code review are different)
- Doesn't share the integration test infrastructure (test spawns `mengdie dream`, not
  the format_dreaming_line function)

**The hard dependency flows this way**:
- json-schema-contract (`schema_version: 1` in format_structured_json) → verify-decay
  hardening test (asserts schema_version). Hard coupling → must co-ship or mock.
- json-schema-contract (docs/schemas/dreaming_pass.json) → ops-doc-polish (rollback
  procedure references breaches[] array). This field ALREADY EXISTS in current output
  (cli.rs:219 `"breaches": result.breached_ids`). The rollback procedure could reference
  breaches[] today without waiting for schema_version. **Soft coupling is softer than
  analysis.md claimed** — ops-doc-polish is not actually blocked by schema_version.

**Verdict**: MCE's 2+1 split is better-supported by the file-touch pattern. The
integration test infrastructure is the real coupling signal, and it only binds 2 of the 3
BLs together. Ops-doc-polish has zero test coupling and minimal cli.rs edit distance
(format_dreaming_line is a different function from format_structured_json).

### On codex-proxy's gate-coherence concern (Topic 2)

Codex said: "The roadmap gate is absolute: 'All 4 BL-decay-* items closed' and 'All 3
open BL-synthesis-* items closed.' 'Open but unmet' ≠ 'closed.'" And therefore `/ae:roadmap
remove` is required, not optional.

**Partially confirmed, partially refined**:
- The gate text IS count-specific ("All 4" / "All 3") — codex is correct that removing
  items breaks the gate counts. Verified above: removing both trigger-gated items changes
  the counts to 3 decay + 2 synthesis, making the gate text factually wrong.
- However, the gate text is USER-OWNED per SKILL.md — it can be updated. The gate is not
  auto-enforced by tooling; `/ae:roadmap close v0.8.0` warns on open items (default) but
  does not parse the gate text to enforce counts. The gate is a human-readable definition
  of done, not a machine-enforced predicate.
- The close subcommand's actual "done" check is: `BL frontmatter status: done/closed`.
  It does NOT parse the gate body. So the gate count incoherence is a documentation
  problem, not a tooling-enforced blocker.

Codex's conclusion (remove is required) is still correct in spirit — leaving them with
open status in the sprint makes the gate semantically dishonest. But the mechanism is
"gate text needs update" + "warn-by-default close proceeds", not "tooling refuses close."

### On gemini-proxy's destructiveness concern (Topic 2)

Gemini said: "If removal is destructive (deletes the BL file entirely), risk is HIGH."

**Confirmed non-destructive**: per SKILL.md, `/ae:roadmap remove` is `mv` to
`unscheduled/` — file body fully preserved, permanent audit entry in v0.8.0 Notes. Low
risk. Gemini's concern is resolved.

### On minimal-change-engineer's action-3-already-done claim (Topic 3)

MCE said: "RUST_LOG normalization (action 3) — SHIP. One line: `env RUST_LOG=info...`".
And in Round 1 I confirmed action 3 is already implemented at line 47.

Both are confirmed correct — action 3 IS already done. The BL body's framing of it as
remaining work is stale. The hardening BL should note this in its implementation. This
effectively reduces the BL from 5 actions to 4 real actions (or 3 if action 5 deferred).

### On challenger's "sprint gate coherence" blind-spot flag

Challenger noted: "The v0.8.0 gate says 'All 7 review-originated follow-ups closed' —
if Topic 2 removes the 2 defer-until-trigger items, the gate's 'All 7' count becomes
incoherent." (challenger.md line ~107)

**Confirmed accurate**. Gate frontmatter (v0.8.0.md:16) says "All 7 review-originated
follow-ups closed" — but initial_items has 9 items, and after plan-014 closed 2, there
are 7 remaining. Removing 2 more leaves 5, making the "7" count wrong. The expanded
gate body also says "All 4 BL-decay-*" and "All 3 BL-synthesis-*" — both counts would
be wrong after removal.

**Action required**: if items are removed, the gate text needs updating to reflect actual
sprint scope. This is a 1-2 line edit, user-owned, low friction.

### On architect's "bundle all 3" position

I did not read the architect.md from round-01 since it returned "unchanged since last
read" — the TL noted it was in round-01. Reading is not available. Based on context from
peer summaries: architect favored bundling all 3 decay BLs. The file-touch evidence
supports MCE's 2+1 split as cleaner (ops-doc-polish has no test coupling and edits a
different function). Bundling all 3 is not wrong but adds review surface without benefit.

---

## Corrections to Round 1

1. **Action 1 binary preflight** — Round 1 said "no proceeding anyway path." WRONG. The
   "proceeding anyway" branch exists at lines 64-73 (JSON parse failure fallback), and
   was present in both git commits. Action 1 is real remaining work (clarifying that
   fallback, not the binary check at lines 35-38).

2. **Action 3 RUST_LOG** — Round 1 confirmed already done. CONFIRMED CORRECT, and
   validated against both git commits (present in original fd910e3 and unchanged in
   e882be9).

---

## Updated Summary of Topic 3 Actions

| Action | Status | LOC estimate |
|--------|--------|-------------|
| 1. Binary preflight "proceeding anyway" clarification | Real work (fallback path, lines 64-73) | ~3-5 LOC |
| 2. DB path `--db-path` flag | Real work | ~10-15 LOC |
| 3. RUST_LOG normalization | ALREADY DONE (line 47, both commits) | 0 LOC |
| 4. CI coverage (integration test) | Real work (shares harness with schema-contract BL) | ~40-60 LOC |
| 5. Threshold-mode `--threshold=N` | DEFER (BL-010 absent, dead code) | N/A |

---

## Open Questions (Round 2 residual)

1. **Gate text update ownership**: If items are removed from v0.8.0, who updates the
   gate text? SKILL.md says gate is user-owned. Is this handled by the TL as part of
   the Topic 2 decision, or does it need a separate tracking item?

2. **`/ae:roadmap close --strict` behavior**: if the discussion recommends `--strict`
   semantics for v0.8.0 close, all remaining items must have status: done/closed.
   The 2 trigger-gated items have status: open. This would force removal (or force-flag)
   even under the "do nothing" policy. Worth confirming if Kai's practice for sprint
   close uses --strict or default.

3. **Does removing items change the `initial_points` count?** Per SKILL.md Invariant 3,
   `initial_items` and `initial_points` are frozen at plan time. Removing items does NOT
   change the frozen snapshot — only the current dir contents change. The scope-delta
   is visible via `--gaps` audit. No velocity math breaks from removal.
