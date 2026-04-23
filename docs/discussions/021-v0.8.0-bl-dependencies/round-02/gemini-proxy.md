---
round: 2
proxy: gemini-proxy
date: 2026-04-23
lens: scope & integration risks (updated with peer evidence)
---

# Gemini-Proxy Round 2: Refined Scope & Integration Risk Analysis

## Findings (per task — incorporating peer context)

### Task 1: Bundling risk re-evaluation (smaller effective scope)

**Revised verdict**: Atomic-rollback-friction concern from Round 1 is **significantly mitigated**. Archaeologist confirmed that actions 1 (binary preflight) and 3 (RUST_LOG normalization) are already implemented in verify-decay.sh:35-38 and :47. This means BL-verify-decay-script-hardening's actual shipped work is:

- Action 2: `--db-path` flag (new, ~10 lines)
- Action 4: CI integration test (new infrastructure)
- Action 5: threshold-mode (to be decided in Topic 3)

So the bundled decay plan's net new code is **smaller than originally framed**. A surgical rollback of actions 2+4 (or 2+4+5) requires reverting only the new additions, not untangling existing logic. **This reduces "rollback pain" from "moderate" to "low."** The "inconsistency risk" Gemini flagged (script too correct for current DB state) is real but orthogonal to bundling vs. splitting — applies to either path.

**Conclusion**: Bundling the 3 BLs still holds up. The shared test harness (mentioned by minimal-change-engineer) between schema-contract and verify-decay's CI test is a real binding point, and the reduced scope makes the bundle even safer to review.

---

### Task 2: Sprint gate consistency after item removal

**Critical risk identified**: Challenger's "sprint gate text breaks" concern is concrete and multi-faceted.

**Problem 1: Metadata Drift**
- v0.8.0 roadmap frontmatter says `initial_items: 9 items` and `initial_points: 29` (locked at sprint commit).
- If Topic 2 removes 2 items via `/ae:roadmap remove`, the *active* item count drops to 7.
- Retrospective tools (like `/ae:retrospect`) will see a mismatch: initial_items=9, final_items=7.
- **Risk**: unclear whether the delta is "scope reduction" (a planned decision) or "planning error" (an admission-gate failure). Velocity/burndown metrics become misleading if the tool doesn't distinguish removal from natural completion.

**Problem 2: Gate Text is Stale**
- v0.8.0 gate text says: "All 7 review-originated follow-ups closed" (from the framing doc, which counted 7 open items after the 2 earlier closures).
- If 2 items are removed, the gate still says "All 7" but the roadmap shows 5 remaining.
- **Risk**: Sprint close succeeds only if the gate checks *actual current items* (not hardcoded counts). If the gate is prose + frontmatter (`gate: "All 7..."` field), it's stale and nobody notices until retrospective analysis.

**Problem 3: Retrospect Output Inconsistency**
- `/ae:retrospect v0.8.0` may report success/failure based on gate evaluation.
- If the gate references "All 7 BL-decay-*" and only 5 are present (2 removed + 3 completed), does the tool report a gate-miss or assume the removed items are "implicitly satisfied"?

**Gemini's recommendation confirmed**: You need a "scope adjustment" protocol. Options:
1. **Gate updates on removal** (automatic): When `/ae:roadmap remove` is called, update the gate text if it references a count. Requires tooling.
2. **Scope-delta notes**: Add a frontmatter field like `scope_delta: "2 items removed (defer-until-trigger) before close"`. Auditable.
3. **Gate is per-cluster, not total**: Rewrite as "All BL-decay-* closed + All BL-synthesis-* closed (deferred trigger items excluded)." Allows semantic correctness.

**Open question for TL**: Which protocol does `/ae:roadmap` already follow? This determines whether removal is safe or requires manual gate edits.

---

### Task 3: Codex proposal on `admission_status: defer-until-trigger` marker

**Scope/risks of the marker scheme**:

**False Negatives (Ghost Defer Risk)**:
- If a BL is functionally "defer-until-trigger" but lacks the marker, it will be pulled into future sprint planning as "active."
- Example: A future BL with body text "waiting for X to stabilize" could be admitted to v0.9.0 if the marker is missing, even though X hasn't stabilized.
- **Severity**: Medium. The cost is one sprint-planning mistake per n missing markers (where n=project maturity). For a 2-item dataset, n is small, but the cost of re-detection is non-zero.

**False Positives (Zombie Task Risk)**:
- If a BL is marked `defer-until-trigger` but the trigger fires *during* the sprint-plan window (e.g., corpus grows to 50k while planning is happening), the BL stays marked as deferred even though it's now active.
- **Severity**: High. The BL becomes "stuck in limbo" — not promoted, not removed, just ignored. Requires manual intervention to reconcile.
- **Mitigation**: The trigger condition must be evaluated at sprint-close time, not at plan-time. If the trigger fired, the marker is automatically cleared.

**Migration Cost (Metadata Debt)**:
- Existing backlog items may already be defer-until-trigger (e.g., BL-decay-dreaming-pass-optim, BL-synthesis-preload-db-miss-edge) without the marker.
- **Option A**: Retroactive marking of the entire backlog. ~2-4 hours of manual audit.
- **Option B**: Accept that the marker only applies going forward (new BLs). Existing unmarked items are handled case-by-case.
- **Option C**: Automation discovers unmarked defer-until-trigger items via body text heuristics (regex for "not now", "filed for trigger", "awaiting"). Risk of false positives.

**Conclusion**: The marker is valuable for **prospective governance** (future sprints) but has **retroactive cost**. Recommend:
1. Adopt the marker for new BLs (Codex's proposal).
2. Retroactively mark the current 2 defer-until-trigger items as a one-shot decision.
3. Do NOT backfill the entire backlog unless the marker becomes critical to other tooling.

---

### Task 4: Concrete test isolation risk on `--db-path` flag

**Revised analysis based on concrete scenarios**:

Gemini's verdict: Low risk if the script is file-scoped; high risk if it modifies environment or shared state.

**In Mengdie's CI context** (from `.forgejo/workflows/ci.yml`):
- Forgejo runner executes jobs sequentially (one job runs fmt, then clippy, then test, then cross-check).
- But within a single `cargo test` invocation, tests can run in parallel by default (`--test-threads=N`).
- The verify-decay shell script is invoked by a Rust integration test via `std::process::Command`.

**Isolation check**:
- `--db-path` parameter: If each test passes a unique path (e.g., `/tmp/test_${test_name}_db.sqlite`), tests run in parallel without collision. ✓ Safe.
- **But**: If the script hardcodes `~/.mengdie/db.sqlite` anywhere (even as a fallback), and the test doesn't override it, two parallel tests might hit the same DB file. ✗ Risk.
- Environment variables: If the script modifies `RUST_LOG` or other env vars globally (instead of passing them to the subprocess), parallel tests could interfere. The BL's action 3 already handles this (`env RUST_LOG=info`), so it's scoped per invocation. ✓ Safe.
- Temporary files: If the script creates temp files in a shared dir without unique names, parallel runs collide. **This is the real risk** — verify the shell script's temp file strategy.

**Concrete risk scenario**:
```bash
# If verify-decay.sh does something like:
tmpdir="/tmp/verify-decay"  # SHARED across parallel tests
mkdir -p "$tmpdir"
# ... test runs here, tmpdir persists
# ... second parallel test tries to use same tmpdir → collision
```

**Recommendation**: The plan's CI integration test must specify `--db-path /tmp/test_verify_decay_${RANDOM}.sqlite` or equivalent. Inspect the shell test code to confirm temp file isolation.

**Severity**: Medium (unlikely given Rust ecosystem conventions, but worth verifying). **Open question for archaeologist**: Does the current shell test harness use unique temp paths?

---

## Synthesis & Cross-Family Integration

**Gemini's insights integrated**:
1. Task 1: Rollback friction mitigated by smaller scope — bundling is safe.
2. Task 2: Sprint gate consistency is a **real operational risk** that other lenses may miss (it's not just "semantics").
3. Task 3: Marker scheme trades prospective governance benefit against retroactive metadata cost — reasonable tradeoff only if applied prospectively.
4. Task 4: Test isolation is file-scoped and **likely safe** (Rust tests are pragmatic about temp isolation), but shell script details matter.

**Disagreements with Gemini**:
- **Task 2**: Gemini called removal a "high risk." I'd refine to "high risk IF gate tooling is brittle; low risk IF gate updates automatically or sprint-close is manual verification." The risk is operational, not technical.
- **Task 4**: Gemini was cautious about side effects. I'd note that Forgejo CI runs tests serially by default (not parallel), which further reduces the real risk. The primary isolation concern is within `cargo test`, not across CI jobs.

---

## Open Questions for Team Lead

1. **Gate update protocol** (Task 2): Does `/ae:roadmap remove` auto-update gate text, or does removal require manual gate edits?

2. **Marker schema** (Task 3): If `admission_status: defer-until-trigger` is adopted, where does the metadata live — frontmatter or BL body tags?

3. **Test harness details** (Task 4): Does the verify-decay shell test use unique temp paths? Confirm before shipping action 4.

4. **Sprint-close semantics** (Task 2, Task 4 intersection): Does `/ae:roadmap close` succeed with open-but-removed items? Or does removal have to happen before close?
