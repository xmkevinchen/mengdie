---
id: BL-decay-json-schema-contract
status: open
origin: BL-008 /ae:review (architecture P2-3 + challenger C1 + gemini P3)
created: 2026-04-20
scope: mengdie (machine-contract stability for the dreaming_pass event)
---

# Document + stabilize the `dreaming_pass` structured-JSON contract

## What

Three related gaps around the structured-JSON line emitted by
`mengdie dream` on stderr:

1. **No schema version** — `format_structured_json` emits
   `{"event":"dreaming_pass",...}` with seven fields. Any rename or
   addition silently breaks `scripts/verify-decay.sh` and any future
   consumer. (Architecture P2-3 + Gemini P3.)
2. **No stderr integration test** — the Step 4 tests call
   `format_structured_json` directly. A future regression back to
   `tracing::info!` (which wraps the JSON inside a log line with
   timestamp + `structured=` prefix) would pass all current tests.
   The post-ship fixup commit `32e11ef` (now squashed into Step 4) was
   necessary precisely because this regression was in the shipped
   Step 4 before self-review caught it. (Challenger C1 HIGH.)
3. **Contract lives only in Rust + shell sync** — no schema doc
   anywhere operators can find. (Gemini P3.)

## Why

Operators will run `verify-decay.sh` when the daemon isn't yet landed
(Phase 2.2 BL-010). If the structured JSON breaks without a test, the
approval gate silently fails to the `--i-reviewed-each` bypass and the
operator waves through demotions without actually reviewing them.

## How to apply

Three actions (can land as one plan):
1. Add `"schema_version": 1` as a top-level field in
   `format_structured_json`. Bump when any other field's semantics
   change. Document the contract at the top of `dreaming-decay.md`
   ops doc with a simple version table.
2. Add an integration test that spawns `mengdie dream --decay-dry-run`
   via `std::process::Command` against an in-memory DB, captures stderr,
   greps for the bare-JSON pattern, parses it with `serde_json::from_str`,
   asserts all required fields and schema_version.
3. Emit the schema as a JSON Schema file at
   `docs/schemas/dreaming_pass.json` so future consumers can target it.

## Trigger

Any of:
- A second consumer of the `dreaming_pass` event lands (MCP tool, daemon
  alerting, analytics export).
- A PR proposes changing any field in `format_structured_json`.
- BL-010 daemon work starts (the daemon is the second consumer).
