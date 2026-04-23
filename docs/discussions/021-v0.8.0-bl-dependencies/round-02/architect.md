---
agent: architect
round: 2
created: 2026-04-23
---

# Architect — Round 2 Positions

## Position updates per topic

### Topic 1: Bundle boundary — REVISED

**Round 1 position**: Keep all 3 BLs together; argued "same function" overlap.
**Round 2 position**: Shift to split 2+1, but for different reasons than
minimal-change-engineer. The factual correction from archaeologist overturns
my primary bundling argument; the reasons to keep the split are still real
but now narrower.

**Factual correction accepted**:
Archaeologist:archaeologist.md lines 13–19 establishes that `BL-decay-ops-doc-polish`
edits `format_dreaming_line` (cli.rs:226), NOT `format_structured_json` (cli.rs:207).
My Round 1 argument rested on "same function, same file" — the same-function half
is refuted. The "same file" half is true but weak coupling: two edits to adjacent
functions 19 lines apart in an 800-line file present near-zero merge conflict risk.

**Responding to minimal-change-engineer's split argument** (minimal-change-engineer.md:14–49):
Their core claim: "review-risk coupling" is the right criterion, not file overlap.
json-schema-contract and verify-decay-hardening have a HARD test coupling (the
integration test asserts `schema_version: 1`); ops-doc-polish has only a SOFT
coupling (the doc references `breaches[]`, which already exists). The shared
test harness argument is also compelling: both json-schema-contract (spawn
`mengdie dream --decay-dry-run`, parse stderr) and verify-decay-hardening action
4 (same subprocess + JSON parse) converge on the SAME test infrastructure.
ops-doc-polish contributes no test infrastructure.

**Where I still diverge from minimal-change-engineer**:
They argue that a docs-only plan has lower review overhead (minimal-change-engineer.md:36–38).
This is true IF ops-doc-polish is pure docs. But it has a cli.rs change
(format_dreaming_line arrow/ASCII fix). That puts it back in "code + docs" territory —
not pure docs. So the "docs reviews land fast" argument is partially true but not
fully clean.

**Revised position**: Split 2+1.
- Plan 015: json-schema-contract + verify-decay-hardening (M+M). These share a
  HARD test coupling and share test harness infrastructure. Co-shipping them is
  not convenience bundling — it avoids a stub-test or a re-work at the seam.
- Plan 016: ops-doc-polish (S) as a follow-up. Its cli.rs touch is `format_dreaming_line`
  only; it has no hard coupling to the schema contract outcome; and the rollback
  section can be written against the already-locked schema once Plan 015 is done.
  Sequencing Plan 016 after Plan 015 is cleaner: the doc references `breaches[]`
  fields that will be stable post-Plan 015.

This 2+1 split also resolves archaeologist.md's open question 1 (line 79): if
actions 1 and 3 are already done in the script, the effective hardening BL shrinks
to actions 2+4 (+ decision on 5). That makes the M+M bundle more like M+S in
practice — still comfortable within one plan.

**On "should the bundle also include release.yml follow-ups"** (topic-01 key
question 3): no, and agree with minimal-change-engineer.md:53–57 that this
question should be deflected. No dependency, different origin (CI, not decay operator
surface). Stay out of scope.

---

### Topic 2: Defer-trigger items — UPDATED with skill semantics

**Round 1 position**: `/ae:roadmap remove` both.
**Round 2 position**: Still `/ae:roadmap remove` both, but now grounded in the
actual skill behavior — and the sprint close question from challenger is resolved.

**`/ae:roadmap remove` semantics confirmed** (SKILL.md:500–510):
The operation is `mv .ae/backlog/<source>/BL-<ID>-*.md .ae/backlog/unscheduled/`.
It is a MOVE, not a delete. The BL file body is preserved intact. Re-filing is
trivial — it's already filed, just in unscheduled/. No information loss.
gemini-proxy.md:21 asked whether this was archiving or destructive — it is archiving.
This confirms the remove option has near-zero irreversibility cost.

**Sprint close semantics confirmed** (SKILL.md:448–453):
The `/ae:roadmap close` command uses **warn-by-default** for open items (not refuse).
For each open item, it emits `⚠ BL-NNN (status: open): not marked done — closing anyway.`
and proceeds. `--strict` escalates to refusal. So leaving the 2 trigger-gated items
in v0.8.0 does NOT technically block sprint close with default behavior.

This means minimal-change-engineer.md:90–94's "do nothing" option is technically
viable — v0.8.0 CAN close with warnings, without needing removal. That is a real
option.

**Why I still recommend remove over do-nothing**:
Three reasons:

1. The scope-delta self-check at close (SKILL.md:454): `/ae:roadmap close` computes
   `removed_after_commit` by comparing current sprint dir contents against
   `initial_items`. If we remove BEFORE close, the removal is logged in Notes
   (SKILL.md:509: `YYYY-MM-DD | descope | BL-<ID> | <reason>`) and the scope-delta
   audit at close sees a clean set. If we leave the items and close-with-warnings,
   the BL files get archived to `done/v0.8.0/` with `status: open` — their trigger
   conditions sit in the done/shipped tier rather than unscheduled. That's a
   discoverability regression: the next trigger-check looks in `unscheduled/`, not
   `done/v0.8.0/`.

2. challenger.md:105–114 raises the gate text coherence problem: the v0.8.0 gate says
   "All 7 review-originated follow-ups closed" and "All 4 BL-decay-* items closed."
   If we close-with-warnings, the gate is vacuously met (close command doesn't enforce
   the gate text — it's descriptive). Removing both items before close means the gate
   text should be updated to say "All 5 review-originated follow-ups closed (2 deferred
   pending trigger)" — honest. We should update the gate text at the time of remove.

3. The `--bump-remaining` flag at close (SKILL.md:455) could also handle this: move
   open items to v0.9.0 at close time. But this pollutes v0.9.0 with items that don't
   belong there (their triggers are unrelated to v0.9.0 scope). Remove back to
   unscheduled is cleaner.

**Addressing challenger's gate-coherence problem** (challenger.md:105–114):
When `/ae:roadmap remove` is run on both items, log the remove in Notes with reason.
Then update the gate text in the roadmap doc to: "All 5 actively-workable
review-originated follow-ups closed; 2 deferred pending trigger (BL-decay-dreaming-
pass-optim, BL-synthesis-preload-db-miss-edge) removed to unscheduled." This is the
Notes + gate-text update pair that closes the coherence gap the challenger raised.
The gate-text update is in the roadmap doc, which is git-ignored local state — no
commit needed, just prose clarity for the close-out.

---

### Topic 3: Hardening scope — UPDATED

**Round 1 position**: Actions 1–4 ship, action 5 defers.
**Round 2 position**: Actions 2+4 are the operative work. Actions 1 and 3 may
already be done. Action 5 defers. Position on 5 unchanged.

**Factual correction accepted** (archaeologist.md lines 125–140):
- Action 1 (binary preflight): lines 35–38 of verify-decay.sh already have
  `command -v mengdie` with exit 2 and actionable message. No "proceeding anyway"
  branch exists. Action 1 is likely already done — confirm at plan time, but
  treat as 0-LOC.
- Action 3 (RUST_LOG normalization): line 47 of verify-decay.sh already has
  `RUST_LOG="${RUST_LOG:-info}"`. Action 3 is already done — 0 LOC.

**Effective work in v0.8.0 plan**:
- Action 2 (--db-path flag): ~10 lines shell. Real current correctness problem
  (silent wrong-DB). SHIP.
- Action 4 (CI coverage): 40–60 LOC integration test. Test harness shared with
  json-schema-contract BL (both spawn `mengdie dream --decay-dry-run`). SHIP —
  and co-ship with Plan 015 (json-schema-contract bundle) to share the test harness.
- Actions 1+3: verify current state, mark done (or confirm already done). No code.

So "BL-verify-decay-script-hardening" in Plan 015 is effectively: action 2 (new flag)
+ action 4 (integration test, shared harness) + verification that 1+3 already done.
That's closer to S than M — but the integration test infrastructure may bump it back
to M depending on DB seeding complexity.

**On action 5** — challenger.md:61–79 adds useful framing: the "first-caller rework"
anti-pattern. Shipping `--threshold=N` now anchors BL-010's interface to the stub's
semantics. Since BL-010 has no design (verified by archaeologist.md:47–49 — "no daemon,
no daemon config, no schema for `decay_spike` consumers"), action 5 should stay deferred.
Shell script dead flags are cheap (no compiled artifact) but the `decay_spike` event
schema is not — defining it now without a consumer commits to a shape BL-010 may
need to change. Defer.

**BL close state**: when Plan 015 ships actions 1–4 (with 1+3 confirmed-done),
close BL-verify-decay-script-hardening with a note: "action 5 (threshold-mode)
not included; depends on BL-010 daemon design. Re-file as BL-decay-threshold-mode
in the BL-010 sprint." Prefer a clean close over an "asterisk close." Filing a new
BL at that time is the right mechanism — minimal-change-engineer.md:149–153 agrees.

---

### Topic 4: Sprint-commitment policy — UNCHANGED in direction, sharper on mechanism

**Round 1 position**: Case-by-case for v0.8.0; `admission_status: defer-until-trigger`
frontmatter for future; file upstream AE backlog item.
**Round 2 position**: Same, with tighter scoping on WHO owns what.

**Alignment with codex-proxy.md**:
codex-proxy.md:57–59 proposes `admission_status: defer-until-trigger` frontmatter as
the preferred mechanism. Same as my Round 1 proposal. codex-proxy asks whether it
belongs in frontmatter or body (codex-proxy.md:81–83) — frontmatter is right for
tooling; body is right for documentation. Both can coexist: frontmatter field for
machine filtering, body already has the human-readable "NOT pursuing this now" language.

**On challenger's "is this ceremony"** (challenger.md:83–101):
Challenger argues Topic 4 is ceremony because a single sentence in the roadmap
checklist would prevent the same failure. That's correct for the immediate case —
but the `admission_status:` frontmatter approach costs ~2 minutes to implement once
(add field to BL template) and zero maintenance thereafter. It's lighter than a
checklist sentence (which requires a human reader to apply correctly every time).
The ceremony argument applies when a policy has recurring maintenance cost; a
frontmatter field has none. So the policy is justified — barely, but it is.

**On WHERE the policy lives**:
The AE plugin owns `/ae:roadmap plan`. A filter on `admission_status: defer-until-trigger`
belongs in the AE plugin's roadmap skill, not in mengdie's CLAUDE.md. Mengdie
CLAUDE.md is for project-specific conventions, not tool behavior.
Action: file one line in AE's upstream backlog: "roadmap plan should warn (not refuse)
when including a BL with `admission_status: defer-until-trigger`." That's the
appropriate scope. No mengdie-local change needed beyond the two removes.

---

## Changed positions summary

| Topic | Round 1 | Round 2 | Driver |
|-------|---------|---------|--------|
| T1: Bundle | Bundle all 3 | Split 2+1 | Archaeologist refuted "same function"; MCE's review-risk-coupling argument correct |
| T2: Remove | Remove both | Remove both + update gate text | Skill semantics confirmed; challenger's gate coherence gap has a concrete resolution |
| T3: Hardening | Actions 1–4 | Effectively actions 2+4 (1+3 already done) | Archaeologist confirmed current script state |
| T4: Policy | AE upstream BL | Same, but frontmatter (not body) and explicitly AE-plugin scope | Codex alignment; challenger ceremony argument addressed |

## Open Questions (surviving from Round 1 + new)

1. **Gate text update**: when removing the 2 items, who writes the gate text update
   in the roadmap doc? That should be part of the `/ae:roadmap remove` invocation's
   follow-up step, not an afterthought.

2. **Plan 016 sequencing for ops-doc-polish**: does it need to wait for Plan 015
   to merge, or can it be a concurrent PR? Given it edits `format_dreaming_line`
   (not `format_structured_json`) and the rollback section references `breaches[]`
   (which already exists in the current schema), it could theoretically run in
   parallel. But the sequencing benefit of having the schema locked first is still
   real for the doc quality. Recommend sequential unless Plan 016 is blocked by
   Plan 015 review time.

3. **Integration test infrastructure for action 4**: is there existing prior art
   in `tests/` for spawning `mengdie` as a subprocess? If yes, the infrastructure
   cost is low. If no, someone needs to set up DB seeding + process spawn — that's
   the non-trivial part. This affects whether Plan 015 is M or L.
