---
id: BL-041
title: "WrapperEnvelope version-bump verification checklist — re-probe on claude-CLI upgrade"
status: open
created: 2026-05-10
origin: "plan 019 final review — architecture-reviewer Item 5 (P2 acknowledged risk)"
trigger: "claude-CLI version bump beyond 2.1.138 detected (CI version check or operator-noticed upgrade) — re-run the production-DB fixture pair against the new wrapper to confirm field names unchanged"
depends_on: []
size: XS
v_target: "ongoing operational hygiene"
---

# BL-041 — WrapperEnvelope version-bump verification

## Problem

`src/core/llm.rs::WrapperEnvelope` is documented as "pinned to claude-CLI
2.1.138 (verified 2026-05-09)". The struct uses `#[serde(default)]` on
each field — which silently maps absent fields to default values.

Failure mode: if Anthropic renames `is_error` → `error` in a future
claude-CLI release, `serde(default)` yields `is_error = false`, and a
model-level error propagates as `StructuredOutputMissing` rather than
`NonZeroExit`. The operator-facing diagnostic still includes
"(verify claude >= 2.1.138 supports --json-schema)" but it points at
the wrong cause — the version is fine; the field rename is silent.

## Proposed action

When a claude-CLI version bump is detected:

1. Run `claude --version` to confirm new version.
2. Run a one-shot probe with the current schema:
   ```bash
   SCHEMA=$(cat ~/Workspace/Projects/mengdie/resources/synthesis-output-schema.json)
   claude -p --json-schema "$SCHEMA" --output-format json \
     --no-session-persistence --permission-mode bypassPermissions \
     --tools "" --model claude-sonnet-4-6 \
     --system-prompt "test" "say hello" \
     < /dev/null > /tmp/probe-stdout.json 2>&1
   ```
3. Verify stdout contains exactly the fields `WrapperEnvelope`
   deserializes: `is_error`, `result`, `structured_output`, and
   optionally `duration_ms`, `total_cost_usd`, `usage`.
4. If any of those four required field names changed → update
   `WrapperEnvelope` and run `cargo test --test synthesis_e2e` to
   confirm the fixture pair still parses.
5. If shape unchanged → bump the "pinned to 2.1.138" comment to the
   newly-verified version. Update `BL-041` history with the new pin date.

## Out of scope

- Automating this check via CI — the value of the manual probe is
  catching field renames AT the version-bump moment, not after. CI
  automation would require the test runner to have claude-CLI installed
  and authenticated, which the existing `#[ignore]` integration tests
  already gate around.
- Adding a fixture for `is_error: true` to the e2e test pair — that's
  a separate BL (file as BL-NNN-wrapper-is-error-fixture if needed).
