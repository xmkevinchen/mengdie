---
id: "03"
title: "Which hardening actions ship in v0.8.0"
status: converged
current_round: 2
created: 2026-04-23
decision: "Ship actions 1 + 2 + 4 in Plan A (alongside json-schema-contract). Action 3 already implemented at scripts/verify-decay.sh:47 — mark done in BL body, no plan work. Action 5 (threshold-mode for daemon) defers — close BL when 1+2+4 land; re-file as BL-decay-threshold-mode targeting the BL-010 sprint. Plan step order: action 2 (--db-path) before action 4 (CI test) because the test needs the flag to avoid hitting operator's default DB."
rationale: "Unanimous Round 2 on defer action 5 (first-caller anti-pattern per challenger — BL-010 has no design, shipping --threshold=N now anchors BL-010 to stub semantics). Archaeologist Round 2 self-correction: action 1 IS real work — the 'proceeding anyway' branch exists at verify-decay.sh:64-73 (JSON-parse fallback, not binary preflight which is also present at :35-38). Action 3 (RUST_LOG=info) confirmed already at :47 across commits fd910e3 + e882be9. Challenger added intra-plan sequencing constraint (action 2 before action 4) — the CI test invokes mengdie with --db-path to isolate from operator's DB."
reversibility: "medium"
reversibility_basis: "Shipping 1+2+4 is standard plan-cycle (revertable). Action 5 defer is a no-op — future BL-decay-threshold-mode can ship at any time. Closing the BL with re-file note is reversible via filing the new BL immediately; trigger context preserved in commit history."
---

# Topic: Which hardening actions ship in v0.8.0

## Current Status

**Converged**: actions 1 + 2 + 4 ship in Plan A; action 3 marked done in BL (already implemented); action 5 defers to BL-010 sprint.

## Round History

| Round | Score | Key Outcome |
|-------|-------|-------------|
| 1 | pending | Unanimous on defer action 5 (all agents); split on whether actions 1/3 already done |
| 2 | converged | Archaeologist verified: action 3 done, action 1 NOT done (Round 1 self-correction), actions 2+4+5 all remaining. Challenger added intra-plan sequencing constraint. |

## Decision Details

Per-action status:

| # | Action | Status | In v0.8.0? |
|---|--------|--------|-----------|
| 1 | Binary preflight + remove "proceeding anyway" branch | NOT DONE (branch at scripts/verify-decay.sh:64-73) | **YES** — Plan A |
| 2 | `--db-path <path>` flag | NOT DONE | **YES** — Plan A (before action 4) |
| 3 | RUST_LOG normalization | ALREADY DONE (verify-decay.sh:47) | mark BL checkbox; no plan work |
| 4 | CI coverage integration test | NOT DONE | **YES** — Plan A (after action 2) |
| 5 | `--threshold=N` daemon mode | NOT DONE | **NO** — defer to BL-010 sprint |

**Sequencing constraint in Plan A**: action 2 must precede action 4. The CI test spawns `mengdie dream --decay-dry-run` and needs `--db-path` to isolate from the operator's `~/.mengdie/db.sqlite`. Without --db-path (action 2), action 4's test either fails or clobbers the operator's real DB.

**BL close procedure**:
- On Plan A merge: update BL-verify-decay-script-hardening body to mark actions 1, 2, 4 done (action 3 already checkmarked as verified-done).
- Close BL with status `done` and a note: "action 5 (threshold-mode for daemon) not included in this BL — re-filed as BL-decay-threshold-mode targeting BL-010 sprint."
- File BL-decay-threshold-mode immediately (in same commit as BL close) to preserve trigger context. Trigger: "BL-010 daemon plan approved / work starts."

**Rejected alternatives**:
- Ship all 5: refuted by unanimous "first-caller anti-pattern" on action 5.
- Ship only 2+4: refuted by archaeologist's action-1 self-correction — the JSON-parse fallback branch is real remaining work.
- Leave BL open with "action 5 pending": refuted — a partially-shipped BL is ambiguous at sprint close; clean close + new BL preserves trigger better.
