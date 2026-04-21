---
id: BL-verify-decay-script-hardening
status: open
origin: BL-008 /ae:review (challenger C2 MEDIUM-HIGH + gemini P3)
created: 2026-04-20
scope: mengdie (operator tooling robustness)
---

# `scripts/verify-decay.sh` — robustness + CI coverage

## What

Three related gaps in the approval-gate script shipped in plan 013 Step 5:

1. **No CI coverage**: the script is never exercised by the test
   suite. If `mengdie`'s stderr JSON format regresses, nothing catches
   it until an operator runs the script, by which point the damage is
   silent (`--i-reviewed-each` bypass exits 0).
2. **Environmental fragility**: three vectors flagged by Gemini/Gemma:
   - Missing `mengdie` binary → cryptic `jq: parse error` instead of an
     actionable "binary not found" message.
   - Hardcoded DB path (`~/.mengdie/db.sqlite`) → silent wrong-DB
     validation if the operator moved the DB.
   - `RUST_LOG` sensitivity: if the operator's env has
     `RUST_LOG=warn` or similar, the `info!` event doesn't fire and the
     JSON line is missing entirely. Script falls back to "human line
     only" and exits non-zero with a WARNING — less actionable than it
     could be.
3. **Approval-gate + daemon interaction (Doodlestein regret from plan
   review)**: when BL-010 daemon lands, this script's human-approval
   flag loses all meaning. Need a non-interactive mode with threshold
   alarm.

## Why

The script is the designated operator surface for BL-008 until BL-010
daemon replaces it. Silent failures in the interim window erode the
"observable forgetting" rationale for shipping BL-008 before it was
needed.

## How to apply

1. **Binary preflight**: early `command -v mengdie` check with a
   specific error message. Already partially done; clarify the
   "proceeding anyway" branch so it's harder to bypass silently.
2. **DB path param**: accept `--db-path <path>` flag, default to
   `~/.mengdie/db.sqlite`. Document in the script header and the ops
   doc.
3. **RUST_LOG normalization**: script forces `RUST_LOG=info` on the
   subprocess invocation. Makes the script deterministic regardless of
   operator env.
4. **CI coverage**: add a shell test (`scripts/test-verify-decay.sh`
   or a cargo integration test) that runs the script against a seeded
   in-memory DB and asserts the expected exit code + output shape. The
   test belongs somewhere that CI actually runs (Forgejo `ci.yml` —
   pairs with `BL-ci-full-clippy-test`).
5. **Threshold-mode for daemon**: when `--threshold=N` is passed
   (default 0 = error on any breach), emit a warning-level structured
   JSON event `decay_spike` instead of blocking. Paves the way for
   BL-010.

## Trigger

Any of:
- BL-010 daemon work starts (mandatory — the script must evolve before
  daemon flips live demotions on).
- First operator-reported issue with the script (`~/.mengdie/db.sqlite`
  location, missing binary, RUST_LOG confusion).
- The CI pipeline gets a full-clippy+test stage via
  `BL-ci-full-clippy-test`.
