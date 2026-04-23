---
agent: archaeologist
round: 1
date: 2026-04-23
---

# Archaeologist Findings — Round 1

## Findings (per topic, with file:line evidence)

### Topic 1: Bundle file-surface reality

**`format_structured_json` — is it actually touched by two BLs?**

Confirmed. `src/bin/cli.rs:207-222` defines `format_structured_json`. Reading
both BL bodies:

- `BL-decay-json-schema-contract` explicitly calls out adding `"schema_version": 1`
  inside `format_structured_json` (cli.rs:211). Direct write.
- `BL-decay-ops-doc-polish` action 2 says "Emit both `→` and `->` in the CLI
  format string (or commit to one and update the AC + plan)". The format string
  lives in `format_dreaming_line` (cli.rs:226-243), NOT in `format_structured_json`.
  So the actual same-function overlap is **narrower than stated** — the
  json-schema-contract BL edits `format_structured_json`, the ops-doc BL edits
  `format_dreaming_line`. Both are in cli.rs (~15 lines apart), but they are
  different functions.

The analysis.md claim "same-file surface" for ops-doc-polish is correct (same
file, `src/bin/cli.rs`). But the "same function" implication in topic-01's
summary is not accurate — they are adjacent functions, not the same one.

**File-touch surface for the 3-BL decay bundle:**

| BL | Files actually touched |
|----|----------------------|
| BL-decay-json-schema-contract | `src/bin/cli.rs` (format_structured_json only, ~5 LOC add), new `docs/schemas/dreaming_pass.json`, new integration test (e2e or tests/) |
| BL-verify-decay-script-hardening | `scripts/verify-decay.sh` (114 LOC today), new test file |
| BL-decay-ops-doc-polish | `docs/operations/dreaming-decay.md` (141 LOC today), minor `src/bin/cli.rs` edit to `format_dreaming_line` |

Cross-file overlap: `src/bin/cli.rs` is touched by both json-schema-contract
(format_structured_json, line ~211) and ops-doc-polish (format_dreaming_line,
line ~226). Minimal merge risk — they edit different functions 15 lines apart.

**LOC count for the bundle:**
- `src/bin/cli.rs`: 792 LOC total. The two BLs touch ~10 lines each in
  separate functions.
- `scripts/verify-decay.sh`: 114 LOC. Actions 1-4 of hardening BL are all
  contained here plus one new test file.
- `docs/operations/dreaming-decay.md`: 141 LOC. Ops-doc-polish adds 3 sections
  (SQL snippet, arrow fix, rollback procedure) — estimate ~20-30 LOC additions.
- New files: `docs/schemas/dreaming_pass.json` (small, ~20 LOC) + one test
  file (~40-60 LOC for integration test).

Total bundle estimated: ~150-200 LOC of new/changed code + 3 new/modified
files. For reference, plan 014 ran 4 CI iterations for 90 LOC. This bundle is
~2× that size but is simpler in nature (no trait changes, no migrations).

### Topic 2: Defer-trigger items — trigger state check

**BL-decay-dreaming-pass-optim triggers:**

Confirmed as NOT fired. Triggers are:
1. Corpus > 50k long-term memories — current corpus is ~41 long-term (per
   BL-decay-dreaming-pass-optim body). NOT fired.
2. `mengdie dream` p95 > 1s — no reports. NOT fired.
3. BL-010 daemon lands — BL-010 does not exist in the codebase. It is
   referenced only in BL bodies, docs/backlog/005-phase2-roadmap.md:75, and
   dreaming-decay.md:48. There is no `src/` code, no plan, no CLI subcommand.
   NOT fired.

**BL-synthesis-preload-db-miss-edge triggers:**

Confirmed as NOT fired. Triggers are:
1. `mengdie delete` / `memory_invalidate` CLI subcommand landing — checked
   `src/bin/cli.rs` Commands enum: no Delete or Invalidate variant exists.
   `memory_invalidate` exists ONLY as an MCP tool in `src/core/mcp_tools.rs:412`
   (MCP server tool, not a CLI subcommand). The CLI has: Dream, Import, List,
   Search, Rename, Stats. No delete path. NOT fired.
2. Observed arithmetic mismatch in production — no reports. NOT fired.
3. DB tombstone semantics — not in schema. NOT fired.

**Key distinction**: `memory_invalidate` in `src/core/mcp_tools.rs:412` is an
MCP RPC tool (sets `valid_until` on a memory). It does NOT run concurrently
with `dream --synthesize` in normal operation — the MCP server and CLI are
separate processes. The trigger condition ("could run concurrently with
dream --synthesize") is genuinely not met. The BL body's concern is correct
but remains theoretical.

### Topic 3: Hardening action sizing (per-action LOC / complexity)

`scripts/verify-decay.sh` is 114 LOC. Reading each action against the current
script state:

**Action 1 — Binary preflight** (lines 35-38 current):
```bash
if ! command -v mengdie >/dev/null 2>&1; then
  echo "mengdie binary not on PATH..." >&2
  exit 2
fi
```
The BL says "already partially done; clarify the 'proceeding anyway' branch."
Looking at current script: there IS NO "proceeding anyway" branch — it exits 2
immediately. The BL's framing of "proceeding anyway" is stale or refers to an
earlier draft. Action 1 may be already complete or require only a comment
change. Estimated: 2-5 LOC.

**Action 2 — DB path param** (`--db-path <path>` flag):
Current script hardcodes nothing visible in the first 114 LOC — `mengdie` is
called without `--db-path` (line 47), meaning it uses its own default
(`~/.mengdie/db.sqlite`). Adding a `--db-path` flag to the shell script
requires: arg-parsing loop extension (~8 LOC), passing it through to
`mengdie dream --decay-dry-run --db-path "$DB_PATH"` invocation. Estimated:
10-15 LOC, low complexity.

**Action 3 — RUST_LOG normalization**:
Line 47 currently: `RUST_LOG="${RUST_LOG:-info}" mengdie dream --decay-dry-run`.
This is ALREADY implemented. The `:-info` default sets RUST_LOG=info if not
set. The BL says "forces RUST_LOG=info on subprocess" — the current code does
exactly this. Action 3 appears already done. Estimated: 0 LOC.

**Action 4 — CI coverage** (new test file):
No existing test for the script. Requires creating either
`scripts/test-verify-decay.sh` (shell test against seeded DB) or a cargo
integration test. The latter pairs with the integration test in
`BL-decay-json-schema-contract` (which also spawns `mengdie dream
--decay-dry-run` and captures stderr). These two integration tests are
**strongly convergent** — both spawn the binary, capture stderr, parse the
JSON line. They could share setup infrastructure. Estimated: 40-60 LOC for the
integration test, medium complexity (needs DB seeding + process spawn).

**Action 5 — Threshold-mode for daemon** (`--threshold=N` flag):
No `--threshold` in current `scripts/verify-decay.sh`. Would require: flag
parsing (~5 LOC), conditional branch replacing the `exit 1` path with a JSON
`decay_spike` event emit (~15 LOC), documentation in script header (~5 LOC).
Estimated: 25-30 LOC, low code complexity but high coupling to BL-010 semantics.

BL-010 is absent from the codebase — no daemon, no daemon config, no schema
for `decay_spike` consumers. Action 5 would ship dead code: the `--threshold`
flag would exist with no caller. However, since the flag lives in the shell
script (not compiled Rust), there is no "dead Rust code" issue — it's just an
undocumented/uncalled flag until BL-010 lands.

**Critical observation on action 3**: The analysis and topic summaries treat
all 5 actions as unimplemented. Action 3 (RUST_LOG normalization) is already
live in the current script at line 47. Action 1 binary preflight is also
present at lines 35-38, though the BL questions the "proceeding anyway"
branch. If the team treats both as done, the effective hardening scope drops
to actions 2 + 4 (+ possibly 5), reducing the M-sized BL closer to S.

### Topic 4: Sprint-commitment policy evidence

No direct code evidence applicable — this is a process topic. Relevant
observation: the v0.8.0 roadmap item count and the fact that BL-decay-
dreaming-pass-optim's own body says "NOT pursuing this now" (line 71 of the
BL) could not be more explicit. This is not ambiguous intent — the BL itself
says it exists "so the trigger is recorded for the day it matters."

## Agreements (with analysis.md claims verified)

1. **Zero cross-cluster dependencies** — confirmed by code inspection.
   `format_structured_json` and synthesis functions are entirely separate;
   no shared state touched by decay vs synthesis BLs.

2. **Hard dependency: json-schema-contract before verify-decay-hardening**
   — confirmed. `scripts/verify-decay.sh:62` greps for
   `"event":"dreaming_pass"`. If the contract BL adds `schema_version: 1`,
   the integration test in the hardening BL would naturally assert on it.
   The ordering constraint is real.

3. **BL-decay-dreaming-pass-optim trigger NOT fired** — confirmed. All 3
   trigger conditions verified against code: corpus at ~41 long-term (body
   says so), no p95 measurements, BL-010 absent from codebase.

4. **BL-synthesis-preload-db-miss-edge trigger NOT fired** — confirmed.
   No CLI `delete` subcommand in Commands enum; `memory_invalidate` is
   MCP-only; no arithmetic mismatch observed.

5. **`format_structured_json` is the primary target of json-schema-contract**
   — confirmed at cli.rs:207-222. The function currently emits 7 fields with
   no schema_version.

6. **`scripts/verify-decay.sh` parses the JSON line the contract BL emits**
   — confirmed at line 62 of the script. The grep pattern
   `'^\{.*"event":"dreaming_pass".*\}$'` directly consumes the output of
   `format_structured_json`. Hard dependency is real.

## Disagreements (claims NOT supported by code, with evidence)

1. **"RUST_LOG normalization" (Action 3) is not yet done** — this is WRONG.
   `scripts/verify-decay.sh:47` already has `RUST_LOG="${RUST_LOG:-info}"`.
   Action 3 of the hardening BL is effectively already implemented. The BL
   body was written before verifying current script state, or it was already
   fixed post-filing.

2. **"Binary preflight" (Action 1) needs the 'proceeding anyway' branch fixed**
   — current script at lines 35-38 exits 2 immediately on missing binary, no
   "proceeding anyway" path. Either the BL's framing is stale (old draft had
   this path), or action 1 is already fully done. Either way, "clarify the
   proceeding anyway branch" may be a no-op.

3. **ops-doc-polish edits `format_structured_json`** — topic-01's summary
   says "both touch cli.rs format_structured_json." Actually ops-doc-polish
   edits `format_dreaming_line` (cli.rs:226), not `format_structured_json`
   (cli.rs:207). They are separate functions 19 lines apart. Merge risk is
   minimal but the two BLs do not conflict on the same function.

4. **Rollback procedure references `breaches[]` array** — analysis.md says
   ops-doc polish "includes a rollback procedure that references the `breaches[]`
   array." Checking the current `dreaming-decay.md`: the doc has no rollback
   section (confirmed by reading all 141 lines). The BL body says to add one.
   The claim is accurate about what the BL WILL do, but the current doc does
   not have this yet — which is expected since the BL is open.

## Open Questions

1. **Are actions 1 and 3 of BL-verify-decay-script-hardening already
   implemented?** If yes, the effective M-size BL shrinks to actions 2 + 4
   (+ decision on 5) — possibly closer to S. This affects the bundle's total
   estimated size.

2. **Does the action-4 integration test overlap with the json-schema-contract
   integration test?** Both would spawn `mengdie dream --decay-dry-run` and
   parse stderr. Could they be one test file, reducing duplication? If bundled
   into one plan, the answer is yes — they'd share the test infrastructure.

3. **Action 5 threshold-mode: dead flag vs useful stub.** Since action 5 lives
   in a shell script (not compiled Rust), shipping a `--threshold` flag with no
   current caller is low cost. But it requires defining the `decay_spike` JSON
   schema even though no consumer exists. Is a schema-less `--threshold` flag
   acceptable, or does it need the `decay_spike` event shape first?

4. **sprint-commitment policy (Topic 4): does this team discussion produce an
   AE-skill-level change or a project-local convention?** The AE roadmap skill
   behavior is out of scope for this discussion per framing.md. But if the
   policy conclusion is "always `/ae:roadmap remove` trigger-gated items before
   sprint-plan," that's a convention that needs to live somewhere durable
   (CLAUDE.md or a project doc).
