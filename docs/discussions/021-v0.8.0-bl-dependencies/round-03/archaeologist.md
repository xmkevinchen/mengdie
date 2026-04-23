---
agent: archaeologist
round: 3
date: 2026-04-23
---

# Archaeologist Findings — Round 3

## A. Code Verification of Round 2 Claims

### A1. Arrow fix in `format_dreaming_line` — literal or computed?

Location: `src/bin/cli.rs:236`

```rust
format!(
    "Dreaming pass: {} promoted, {} ({} floor breaches, avg effective {:.3} → {:.3})",
    ...
)
```

The `→` Unicode arrow is a **literal character embedded directly in the format string**
at cli.rs:236. It is not a variable, not a conditional, not a constant — it is a hard-
coded UTF-8 code point inside the format string literal.

The ops-doc-polish BL action 2 says: "Emit both `→` and `->` in the CLI format string
(or commit to one and update the AC + plan)." Since it's a single literal, the change
is either:
- **Option A** (keep `→`): change nothing in the code, update the AC regex to remove
  the ASCII fallback. 0 LOC code change.
- **Option B** (add both): replace `→` with a runtime choice — but a `if tty { "→" }
  else { "->" }` branch would be ~5 LOC. More than option A.
- **Option C** (commit to `->`): replace one character with two ASCII characters. 1
  char change.

The BL body leans toward "commit to one and update the AC" as the simpler path. Given
the arrow is a literal, this is the smallest possible change in the entire BL. The
"minor cli.rs format tweak" language in analysis.md accurately describes this — it is
literally 1-2 characters.

**Conclusion**: the format_dreaming_line arrow fix is smaller than any peer estimated.
It's not "code + docs" in any meaningful sense — it's a 1-character (or 0-character)
change to the format string, with the real work being the docs additions (SQL snippet,
rollback section). The architect's Round 2 concern ("it has a cli.rs change, so it's
code + docs, not pure docs") is technically correct but overstated — this is effectively
a docs-dominated plan with a trivial format string note.

### A2. `--i-reviewed-each` bypass path — does it approve with zero breach data?

Confirmed: yes, it approves with zero validated breach data.

The exact path (scripts/verify-decay.sh:62-73):
```bash
JSON_LINE=$(grep -E '^\{.*"event":"dreaming_pass".*\}$' "$TMP_ERR" | head -n1 || true)

if [[ -z "$JSON_LINE" ]]; then
  echo "WARNING: could not parse structured JSON line..." >&2
  echo "Raw stderr:" >&2
  cat "$TMP_ERR" >&2
  if [[ $APPROVED -eq 1 ]]; then
    echo "(Proceeding anyway: --i-reviewed-each was passed.)" >&2
    exit 0   # ← exits here, NO BREACH DATA CHECKED
  fi
  exit 1
fi
```

If the JSON line parse fails AND `--i-reviewed-each` is passed, the script:
1. Prints the raw stderr (which may be empty or garbled)
2. Prints "(Proceeding anyway...)"
3. **Exits 0 with no breach count, no breach list, no field validation**

The BREACHES variable is never set on this path. The operator is approving a dry-run
without having seen any structured output about what would be demoted. This is exactly
the "silent bypass" the BL describes: the script exits 0 but the operator waved through
with no data.

Challenger's claim is **fully confirmed**: the bypass path can "approve" a decay pass
with zero breach data.

**Action 1 characterization** (correcting my Round 2 correction): the "proceeding
anyway" branch IS the action 1 target. The BL says "clarify the proceeding anyway branch
so it's harder to bypass silently." This is correct — the current behavior lets an
operator accidentally pass `--i-reviewed-each` on a corrupted stderr and get a success
exit with no review actually happening. Action 1 = make this path more explicit (e.g.,
require a second confirmation flag, or print a more prominent error, or refuse exit 0 on
this path entirely). This is real hardening work, ~3-5 LOC, and my Round 2 description
was accurate.

### A3. Does any existing test spawn `mengdie` as a subprocess for a seeded DB?

**No existing test spawns the `mengdie` CLI binary as a subprocess.**

Checked:
- `tests/e2e.rs`: uses `#[ignore]` for the full pipeline test (requires fastembed ~90MB
  model). The decay smoke test at line 140 seeds a DB and calls library code directly
  (`db.run_dreaming_with_config()`), not the binary. Uses `tempfile::NamedTempFile` for
  isolated DB paths (unique per test, no collision risk).
- `tests/dream_synthesis.rs`: spawns `claude` CLI (not `mengdie`), also `#[ignore]`.
- `tests/llm_claude_cli.rs`: spawns `claude` CLI for LLM provider tests, also `#[ignore]`.

**No test uses `std::process::Command` to spawn `mengdie dream --decay-dry-run`.**

The BL-decay-json-schema-contract integration test (action from the BL body: "spawns
`mengdie dream --decay-dry-run` via `std::process::Command`, captures stderr, parses
JSON") would be the FIRST test to do this. Infrastructure required:
1. Build the `mengdie` binary before the test (or require it to be pre-built)
2. Seed a DB file (prior art: `tempfile::NamedTempFile` + direct rusqlite writes, as
   in `tests/e2e.rs:145-176`)
3. Spawn the subprocess with `--db-path` pointing to the temp DB
4. Capture stderr, grep for the JSON line, assert fields

The DB-seeding infrastructure exists (`tempfile` + `rusqlite::Connection::open` pattern
from tests/e2e.rs:145-177). The subprocess-spawn infrastructure does NOT exist for the
`mengdie` binary itself. This is new work.

**Codex's concern is confirmed**: the infrastructure cost is real — not enormous (the
seeding pattern is established, ~20 LOC setup), but spawning a binary requires the
binary to exist in a known location. The standard approach is to use
`std::env::current_exe()` parent dir or `cargo test` conventions
(`env!("CARGO_BIN_EXE_mengdie")`). This needs one-time setup.

**Gemini's test isolation concern** (temp file collision): confirmed not a risk in
existing tests — each test uses a unique `tempfile::NamedTempFile` path. The pattern
would carry over to the new integration test. The `verify-decay.sh` itself already uses
`mktemp` for its temp files (lines 42-43), so parallel runs of the script would have
unique temp paths. Low isolation risk.

---

## B. Topic Scores

### Topic 1: Bundle boundary (2+1 split vs 3-bundle)

**Consensus score: 5/5**

All agents who engaged with the code evidence converged on 2+1 split:
- Archaeologist (R1): confirmed different functions, soft coupling for ops-doc
- Architect (R2): explicitly revised from bundle-all-3 to 2+1, citing archaeologist's
  correction
- MCE (R1): proposed 2+1 from the start, coupling analysis was correct
- Challenger (R2): cited plan-013 evidence that supports M+M coupling (schema-contract
  + verify-decay), NOT 3-bundle. Explicitly said ops-doc-polish should split.
- Gemini (R2): "bundling is safe" but focused on the M+M pair; no dissent on splitting
  ops-doc-polish
- Codex (R1): "prefer the bundle" for review verifiability — but the argument was about
  internal dependency capture, which applies to the M+M pair. Codex did not specifically
  argue for including ops-doc-polish.

**Decision**: Plan 015 = json-schema-contract + verify-decay-hardening (M+M, shared
integration test harness). Plan 016 = ops-doc-polish (S, after Plan 015 merges).
Ops-doc-polish can reference the now-stable schema, and the arrow fix is 1-2 characters.

**No remaining dissent worth naming.** Codex's "prefer the bundle" in R1 was aimed at
the schema-contract/verify-decay pair — not a vote for including ops-doc-polish.

---

### Topic 2: Fate of the 2 defer-until-trigger items

**Consensus score: 4/5**

**Converged position**: `/ae:roadmap remove` both with `--reason`, update gate text.

Agreement:
- Architect (R2): remove both, update gate text, provides exact gate text wording
- Codex (R1): remove is required for gate honesty
- Challenger (R2): BLIND SPOT raised — partial-failure recovery path not specified. Does
  `/ae:roadmap remove` process both in one call or require two sequential calls?
- Gemini (R2): gate consistency risk is real; gate text update required
- MCE (R1): "do nothing" position was tooling-compatible (close warns, not refuses), but
  challenger's close-with-warnings = items archived to `done/v0.8.0/` with `status:open`
  was noted in architect R2 — that's a discoverability regression the MCE position didn't
  account for

**Why score 4 not 5**: Challenger's partial-failure concern is unresolved. Two sequential
`/ae:roadmap remove` calls have no atomicity guarantee. If one fails mid-way, the sprint
is asymmetric. SKILL.md does not specify a batch form of `remove`. Resolution: the
executor should run both removes in sequence and verify both succeed before updating the
gate text. Recovery path = `/ae:roadmap add <BL-ID> v0.8.0 --reason "re-adding: prior
remove failed, retrying both atomically"`. This is trivial but should be named.

**Gate text update required**: v0.8.0 gate frontmatter says "All 7 review-originated
follow-ups closed"; gate body says "All 4 BL-decay-*" and "All 3 BL-synthesis-*".
After removal, update to: "All 5 actively-workable follow-ups closed; BL-decay-dreaming-
pass-optim and BL-synthesis-preload-db-miss-edge removed to unscheduled (trigger not
fired)." Gate body lines 1 and 2 can become "All 3 BL-decay-* items closed" and
"All 2 BL-synthesis-* items closed."

---

### Topic 3: Which hardening actions ship in v0.8.0

**Consensus score: 5/5**

**Converged position**:
- Actions 2 + 4 are the operative work (SHIP)
- Action 3 (RUST_LOG) is already done — confirm at plan time, no LOC
- Action 1 (binary preflight clarification) is real ~3-5 LOC work — the "proceeding
  anyway" bypass at lines 64-73 needs hardening (now code-verified above)
- Action 5 (threshold-mode) DEFERS — BL-010 absent, no consumer, first-caller anti-
  pattern risk confirmed by challenger and architect
- BL-verify-decay-script-hardening closes after shipping 1+2+4 (confirming 3 already
  done); action 5 re-files as a new BL with origin provenance per challenger R2

**Note from A3**: challenger's "action 2 before action 4" in-plan ordering is confirmed
correct by the substrate — the CI test needs `--db-path` to point to a known temp file.
That ordering must appear explicitly in the plan's step sequencing.

**Additional note from A3**: integration test infrastructure requires `CARGO_BIN_EXE_mengdie`
or equivalent to locate the binary. This is one-time setup (~5 LOC in the test), but
the plan must acknowledge it. DB-seeding pattern is already established in
`tests/e2e.rs:145-177`.

---

### Topic 4: Sprint-commitment policy

**Consensus score: 4/5**

**Converged position**: no policy changes in this discussion; file one AE upstream
backlog item.

Agreement:
- MCE (R1): no policy, case-by-case is sufficient
- Challenger (R2): `admission_status` is medium-ceremony without AE tooling; file as
  AE upstream BL only
- Architect (R2): AE upstream BL for the `admission_status` marker; nothing in mengdie-
  local CLAUDE.md
- Gemini (R2): marker is useful prospectively but has retroactive cost; only apply going
  forward if adopted
- Codex (R1): `admission_status: defer-until-trigger` frontmatter as preferred mechanism

**Why score 4 not 5**: Codex wanted the frontmatter field adopted (R1), challenger says
it's ceremony without AE tooling enforcement. These positions have not explicitly
reconciled. The practical resolution is: file the AE upstream BL (architect's call),
and don't add the field to current BLs. If AE implements the skill filter, the field
becomes worth adding retroactively to the 2 current items. No action in mengdie itself.

---

## C. Remaining Blockers

### Topic 2 (score 4): partial-failure recovery path

**Blocker**: SKILL.md `/ae:roadmap remove` takes one BL-ID per invocation. Two calls
are required to remove both items. If the second fails, the sprint is asymmetric.

**Resolution**: name the recovery path in the discussion conclusion:
> Run `/ae:roadmap remove BL-decay-dreaming-pass-optim --reason "trigger not fired; returning to unscheduled"` then verify, then run the second remove. If either fails, use `/ae:roadmap add <BL> v0.8.0 --reason "restoring: prior remove failed"` to restore and retry both. Not atomic, but recoverable.

This unblocks the decision — no code change needed, just a named procedure.

### Topic 4 (score 4): Codex/challenger alignment gap

**Blocker**: Codex wants `admission_status` frontmatter on current BLs; challenger says
don't add the field without tooling.

**Resolution**: file AE upstream BL for the skill filter (architect's call). Don't
retroactively mark current BLs. The two trigger-gated items are already being removed
from v0.8.0 (Topic 2), so they don't need the field now. When and if the AE skill filter
lands, the field becomes mechanical to add. No action required in this discussion.

---

## Summary for TL

| Topic | Score | Decision |
|-------|-------|----------|
| T1: Bundle boundary | 5/5 | Plan 015 = json-schema-contract + verify-decay-hardening (M+M); Plan 016 = ops-doc-polish (S), after 015 |
| T2: Defer-trigger fate | 4/5 | `/ae:roadmap remove` both with `--reason`, update gate text; partial-failure recovery path named |
| T3: Hardening scope | 5/5 | Ship actions 1+2+4 (action 3 already done, action 5 defers + re-files); action 2 must precede action 4 in plan |
| T4: Sprint policy | 4/5 | No mengdie-local change; file AE upstream BL for `admission_status` filter; don't add field without tooling |
