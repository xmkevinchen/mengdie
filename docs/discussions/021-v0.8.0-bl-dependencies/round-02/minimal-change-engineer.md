---
agent: minimal-change-engineer
round: 2
created: 2026-04-23
lens: minimum-viable diffs, refuse scope creep, 3 similar lines beats premature abstraction
---

# Round 2 — Minimal-change Engineer

## Round 2 Tasks

### Topic 1: Split 2+1 is now clearly the minimum

**Position firm: json-schema-contract + verify-decay-hardening bundled.
Ops-doc-polish splits out.**

Archaeologist's fact kills architect's strongest argument. Architect's
case for bundling ops-doc-polish rested on three claims; two just
fell:

- **architect.md:17-21** argued `cli.rs format_structured_json` is
  touched by BOTH json-schema-contract AND ops-doc-polish — "same
  function, same file — two separate plans would produce a conflicting
  diff or force artificial sequencing." False.
  **archaeologist.md:201-205** confirms ops-doc-polish edits
  `format_dreaming_line` (cli.rs:226), NOT `format_structured_json`
  (cli.rs:207). Different functions, 19 lines apart. Zero diff
  conflict. Zero forced sequencing.

- **architect.md:39-41** argued ops-doc-polish's rollback procedure
  references `breaches[]` from the schema contract, so the rollback
  shouldn't be written until the schema is locked. Partially false.
  **archaeologist.md:207-212** — rollback section doesn't yet exist,
  and `breaches[]` is a field that's already present in the current
  JSON output regardless of `schema_version`. The rollback text can
  reference `breaches[]` without waiting for the schema bump. The
  SOFT coupling is even softer than architect implied.

- **architect.md:42-44** argued "splitting an S out of M+M+S saves
  roughly zero plan cycles (it generates its own plan + review
  overhead equivalent to what it saves)." This is the claim TL asks
  me to answer directly.

**On "S-plan overhead = its own savings":**

It's not zero savings. Here's the concrete accounting:

1. **Ops-doc-polish has ZERO Rust test interaction.** The
   json-schema+hardening bundle has to build a stderr integration
   test harness (per archaeologist.md:121-129, a 40-60 LOC cargo
   integration test that spawns the binary). Any plan touching that
   harness runs it through `cargo test` every CI iteration. Plan 014
   ran 4 CI iterations for 90 LOC. **If ops-doc-polish joins the
   bundle**, every iteration of "fix the ops-doc SQL snippet" or
   "tweak the rollback SQL" triggers the full test harness rebuild
   and run. That's the concrete overhead cost of including a docs BL
   in a code+test plan: churn in the docs section drags the test
   cycle along.

2. **`/ae:review` agent team cost scales with review-surface
   diversity.** challenger.md:26-31 makes this point:
   "A 4-agent review covering M+M+S simultaneously has to spread
   attention across: (a) a JSON schema correctness question, (b)
   shell script robustness, (c) documentation accuracy. These are
   three different skill domains." A docs-only review by contrast is
   uniform — one agent can competently review SQL snippets, rollback
   procedure, arrow fallback. Agent-team cost isn't just "N plans
   × M agents"; it's "coherent review surface vs diffuse review
   surface."

3. **Action-2 in ops-doc-polish has a live controversy** (BL body
   line 47-48): "Emit both `→` and `->` in the CLI format string
   (or commit to one and update the AC + plan)." This is a
   not-yet-resolved design choice. Bundling it with the
   schema-contract work means that choice has to be resolved as part
   of a plan whose main scope is schema stability. Splitting lets the
   docs plan own that choice cleanly.

**What IS the per-plan overhead?** Honest accounting based on plan 014:

- Plan skill invocation: ~5 min
- Plan review team: ~10-15 min of review-time per agent, 4 agents
- CI iterations: variable, 1-4 per plan (plan 014 hit 4 due to
  environment issues; most smaller plans hit 1-2)
- Close-out commit churn: ~5 min

For a docs-only S plan with no cargo tests to run, realistic
overhead is ~30 min total vs a code-plan's potentially 2-4 hours of
CI iteration churn. Docs plans are genuinely lighter.

**Minimum shape** (unchanged from Round 1):
- Plan 015: json-schema-contract + verify-decay-hardening (M+M,
  shared stderr test harness — see archaeologist.md:221-224 "they'd
  share the test infrastructure")
- Plan 016: ops-doc-polish (S, docs + 1-line format_dreaming_line
  tweak, no Rust test involvement)

This is concretely smaller total work than "bundle all 3."

### Topic 2: Archaeologist's verification shifts my position

**Round 1 position: "do nothing, leave them in v0.8.0."**

**Round 2 position: `/ae:roadmap remove` both — but only because
archaeologist and codex showed me the close-gate isn't trigger-aware.**

What changed my mind:

- **codex-proxy.md:31** — "The roadmap gate is absolute: 'All 7
  review-originated follow-ups closed' and 'All 4 `BL-decay-*` items
  closed' + 'All 3 open `BL-synthesis-*` items closed.' 'Open but
  unmet' ≠ 'closed.'" This is the answer to my Round 1 Open
  Question #1 ("What does `/ae:roadmap close` actually do with open
  items?"). Leaving them open will either block close or force a
  messy wontfix classification. That's a real concrete cost I
  couldn't confirm in Round 1.

- **archaeologist.md:70** confirms `/ae:roadmap remove` "returns the
  BL file to `unscheduled/` intact, preserving full body text
  including the trigger conditions" — via architect.md:69-71 quoting
  the same behavior. So the destructiveness risk gemini-proxy flagged
  (gemini-proxy.md:20-21 "If removal is *destructive*... risk is
  **high**") is answered: it's administrative, not destructive. No
  trigger context lost.

So the cost comparison updates:
- Do nothing → guaranteed close-gate conflict at sprint close. Costs
  real work to resolve then.
- `/ae:roadmap remove --reason "..."` → one command per item, BL
  file preserved intact in unscheduled/, close-gate becomes clean.
  Cost: 2 commands + a 1-line reason each.

`/ae:roadmap remove` IS smaller total work than "do nothing + fix
close-gate conflict later." I concede this topic.

**Pushback on architect's "symmetry" claim**: architect.md:79-80
says both items should be handled identically because both have
"filed for trigger" character. Agree on the action (remove), but
symmetry isn't the reason — the reason is that each item
independently fails the close-gate. If one had a trigger-firing
expected within v0.8.0, it should stay. They just happen to both
not meet that bar. Don't generalize from n=2.

**Reject challenger's "foreclosed option 4" (challenger.md:43-47)**:
"conditional BL with automatic promotion" + `promote_to: v0.9.0`
stanza. This introduces new frontmatter semantics, new tooling for
auto-promotion, and a new state machine — all to avoid typing
`/ae:roadmap remove` twice. Classic premature abstraction. Reject.

### Topic 3: Update BL, ship smaller plan — do not re-plan from scratch

**With archaeologist.md:115-119 confirming action 3 is already live
(line 47 of the script) and archaeologist.md:94-105 confirming
action 1 has no "proceeding anyway" branch in the current script
(the BL's framing is stale)**, the actual v0.8.0 work shrinks:

- Action 1: possibly complete; worst case, a 2-5 LOC clarification.
  **archaeologist.md:216-219** — "If yes, the effective M-size BL
  shrinks to actions 2 + 4 (+ decision on 5) — possibly closer to S."
- Action 2: 10-15 LOC shell arg parsing (still to do)
- Action 3: already done (0 LOC)
- Action 4: 40-60 LOC test (still to do)
- Action 5: defer

**Minimum-change answer to TL's question "do we need a new plan at
all?":**

Yes, one plan still. But the minimum version is:

1. **Update `BL-verify-decay-script-hardening` in-place** to reflect
   reality: mark action 3 as already-shipped (cite
   `scripts/verify-decay.sh:47` with the `${RUST_LOG:-info}` default).
   Action 1 — verify and either mark as already-shipped or narrow
   to "2-5 LOC clarification." Do this as a single-file edit to the
   BL markdown. No plan cycle needed. Pure bookkeeping.

2. **Plan 015 scope** becomes `BL-decay-json-schema-contract` +
   the surviving actions of `BL-verify-decay-script-hardening`
   (actions 2 + 4, maybe action 1 clarification). Action 5 splits
   out.

**Why still one plan and not just in-place PRs:** action 4 (the CI
integration test) shares test infrastructure with json-schema-contract's
integration test (archaeologist.md:122-129). Building two test harnesses
when one suffices IS waste. So the bundle of schema-contract +
hardening-2+4 still wins on the shared-harness argument.

**Counter to "just ship actions 2+4 as their own plan":**

Not minimum. Actions 2 and 4 are mostly in `scripts/verify-decay.sh`
and a new test file. Those don't touch `cli.rs` at all. Splitting
them from schema-contract (which IS `cli.rs`) would seem to argue
for separate plans. BUT: the integration test for action 4 spawns
`mengdie dream` and asserts on the stderr JSON line's shape — which
is exactly what json-schema-contract is changing. If the
schema-contract changes `format_structured_json` to include
`schema_version: 1`, the hardening test must assert the post-change
shape. Same test file, same run, same harness. Co-plan.

**Summary of minimum actions**:
1. Edit BL-verify-decay-script-hardening's "How to apply" to mark
   action 3 as done (and action 1 to the extent already done). No
   plan cycle.
2. Plan 015 = json-schema-contract + hardening actions 2 + 4
   (+ action 1 clarification if still needed).
3. Split action 5 into its own BL
   (`BL-verify-decay-threshold-mode`) filed to `unscheduled/` with
   trigger "BL-010 daemon design commits to a threshold semantics."
4. Plan 016 = ops-doc-polish (independent, per Topic 1).

### Topic 4: Reject codex's `admission_status` marker — still premature abstraction

Codex proposes (codex-proxy.md:59 + topic-04 context,
and architect.md:156-159 reaches the same proposal):
> "add an admission marker (e.g., `admission_status: defer-until-trigger`)
> and a rule that such items are excluded from sprint commitment
> unless the trigger is expected within the sprint."

**Apply minimum-change discipline item by item:**

1. **New frontmatter field** — a new YAML key in the BL schema.
   Every future BL writer must now consider whether to set it.
   Every reader (human + tooling) must now know what it means.
   That's schema expansion. Not free.

2. **Scan-filter check in `/ae:roadmap plan`** — this requires a
   change to the AE plugin skill itself (confirmed out-of-scope by
   framing.md:46-50 and architect.md:161-162 notes "requires an AE
   plugin change (out of this discussion's scope)"). The fact that
   executing the policy requires out-of-scope work is itself a
   signal that the proposal is premature.

3. **False-positive / false-negative handling** — what does a
   reviewer do when a BL has `admission_status: defer-until-trigger`
   but the trigger IS expected in the sprint? Add a new override
   field? Negotiate in the BL body? The proposal smells simple
   because it papers over the edge cases.

4. **n=2 data points** — this is still the fundamental issue. We
   have seen this failure mode twice. Designing a schema extension
   for it is the same shape as action 5 designing for BL-010: a
   plausible-sounding abstraction for a future that may never
   recur. The honest question: is there any evidence this problem
   recurred BEFORE v0.8.0? If not, it happened once; n=2 items in
   the same sprint is one failure event, not two.

**Defend "no policy":**

- Challenger's angle is correct (challenger.md:82-101):
  "The underlying problem... has a one-time fix (Topic 2) and
  doesn't require a policy... The same outcome — noticing the
  mismatched BLs before commit — would be achieved by a single
  sentence in the `ae:roadmap plan` checklist, not a standalone
  policy document."

- Gemini-proxy confirms (gemini-proxy.md:32-34): "Introducing a gate
  that filters 'not now' BLs at plan-time sounds defensive but
  creates **operational debt**... On a solo-dev project, this
  overhead is likely unjustified."

- If Kai wants a mnemonic to prevent repeat, the minimum is a
  ONE-LINE addition to the top of `v0.9.0.md` (or the sprint-plan
  template): "Before committing: grep each BL body for 'not now',
  'filed for trigger', 'NOT pursuing' — if matched, re-justify or
  leave unscheduled." That's a grep command, not a schema change.

**My concession point**: IF the AE plugin team is already touching
the roadmap skill for some other reason and the `admission_status`
field would be a tiny addendum, I'd be less opposed. But absent a
pre-existing planned change, introducing it now is speculative
tooling.

**Position**: no policy, no frontmatter marker. If anything, add
exactly one checklist line to the sprint-plan workflow. That's it.

## Responses to Specific Peer Claims

- **architect.md:38 "ops-doc-polish has a format-string change in
  cli.rs (arrow/ASCII fallback — item 2)"** — confirmed true
  (archaeologist.md:22), but it's a ONE-LINE change in a DIFFERENT
  FUNCTION than schema-contract touches. The "code change" framing
  overstates. It's a tweak, not code-review-relevant coupling.

- **architect.md:42-44 "splitting S from M+M is zero net savings"** —
  addressed above. Not zero: docs-only plans skip cargo-test CI
  cycles; agent-team review cost scales with surface diversity.

- **archaeologist.md:226-230 "action 5 threshold-mode: dead flag vs
  useful stub... Since action 5 lives in a shell script (not compiled
  Rust), shipping... is low cost."** — I partially disagree. The code
  cost IS low (shell script, no Rust type churn). But the SEMANTIC
  cost is still there: by shipping `--threshold=N` now, we commit to
  what N means (breach count? percentage? absolute floor?) and what
  the `decay_spike` event shape is. BL-010 may want different
  semantics. challenger.md:77-80 says this better than I did:
  "if action 5 ships and BL-010 makes incompatible interface
  assumptions, the script must be modified twice." Defer still wins.

- **challenger.md:104-115 "Missing: Sprint gate coherence after item
  removal"** — real concrete problem. If we remove 2 items from
  v0.8.0, the gate statement "All 7 review-originated follow-ups
  closed" becomes incoherent. **Minimum fix**: when invoking
  `/ae:roadmap remove`, also update the gate statement in the sprint
  frontmatter to "All 5 review-originated follow-ups closed" with a
  Notes entry citing the 2 removed items and their triggers. One
  text edit, done in the same commit as the removes.

- **challenger.md:117-126 "Missing: Whether Plan B (synthesis) is
  actually v0.8.0"** — out of this discussion's scope per
  framing.md:72-74 ("Changes to any individual BL's fix option ...
  Out"). Not my topic to resolve. Flag for a separate discussion
  if synthesis commitment is really in doubt.

- **gemini-proxy.md:28 "--db-path flag test isolation risk"** —
  interesting angle but out of topic scope. Whether the flag exists
  at all is Topic 3; how the test suite uses it is a plan-level
  detail. Note it, defer it.

## Scope-creep risks (Round 2 update)

1. **Codex's `admission_status` frontmatter** — n=2 abstraction, new
   schema, requires out-of-scope plugin change. New this round.
2. **Challenger's "conditional BL with auto-promotion"** — new
   frontmatter, new tooling, new state machine. Reject.
3. **Architect's "AE-backlog item for upstream convention"
   (architect.md:168-169)** — creating upstream work for a
   convention we don't have evidence we need. Reject for now; file
   later if the pattern actually recurs.
4. **Gate-text update scope creep** — when we fix the gate count
   post-remove, resist the temptation to rewrite the whole roadmap.
   Just the count + a Notes line.

## Open Questions (Round 2)

1. **Is action 1 of hardening BL actually done or needs a 2-5 LOC
   clarification?** archaeologist.md:94-105 suggests it may be done
   but notes ambiguity around the "proceeding anyway" branch that
   no longer exists. A direct read of `scripts/verify-decay.sh:35-38`
   in Round 3 would settle it. If done, BL-verify-decay-script-hardening
   drops further to actions 2+4 only (solidly S-sized).

2. **Does plan 015's shared stderr test harness live in `tests/` as
   a single integration test file, or as two separate files that
   share a helper module?** Affects "shared harness" savings claim.
   One test file is strictly smaller code; two files with a helper
   is more flexible. Given minimum-change, prefer one test file.

3. **Post-remove gate text**: do we phrase the update as "All 5
   (was 7)" or rewrite cleanly to "All 5"? Small but real question
   about how roadmap notes capture scope deltas.
